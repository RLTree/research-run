use std::fs;

use crate::Error;
use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan};
use crate::workspace::tests::temporary;
use crate::workspace::{Workspace, inject_storage_failure};

use super::*;
use crate::workspace::agent_integration_content::compose_planned_instruction;
use crate::workspace::agent_integration_transaction::witnesses::TRANSACTION_VERSION;

#[test]
fn coverage_recovery_inspection_faults_and_invalid_shapes_fail_closed() {
    for (sequence, point) in [
        (1, "inspect project instruction transaction"),
        (2, "inspect project instruction transaction#2"),
        (3, "inspect transaction witness"),
    ] {
        let (root, plan, original, planned) = fixture(point);
        let transaction = root.join(format!(".AGENTS.md.88.{sequence}.txn"));
        fs::create_dir(&transaction).unwrap();
        write_transaction(&transaction, &original, &planned, &planned);
        inject_storage_failure(point);
        assert!(recover_transaction(&transaction, &root.join("AGENTS.md"), &plan).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    let (root, plan, _, _) = fixture("shape");
    let transaction = root.join(".AGENTS.md.88.4.txn");
    fs::write(&transaction, b"not a directory").unwrap();
    assert!(recover_transaction(&transaction, &root.join("AGENTS.md"), &plan).is_err());
    fs::remove_dir_all(root).unwrap();

    let (root, plan, _, _) = fixture("child directory");
    let transaction = root.join(".AGENTS.md.88.5.txn");
    fs::create_dir(&transaction).unwrap();
    fs::create_dir(transaction.join("exchange")).unwrap();
    assert!(recover_transaction(&transaction, &root.join("AGENTS.md"), &plan).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn coverage_recovery_retains_oversized_witnesses() {
    let (root, plan, original, planned) = fixture("oversized witness");
    let transaction = root.join(".AGENTS.md.89.2.txn");
    fs::create_dir(&transaction).unwrap();
    write_transaction(&transaction, &original, &planned, &planned);
    fs::write(
        transaction.join("original"),
        vec![b'x'; MAX_INSTRUCTION_BYTES as usize + 1],
    )
    .unwrap();

    assert!(matches!(
        recover_transaction(&transaction, &root.join("AGENTS.md"), &plan),
        Err(Error::AmbiguousEffect(_))
    ));
    assert!(transaction.exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn coverage_recovery_rejects_post_exchange_mode_drift() {
    use std::os::unix::fs::PermissionsExt;

    let (root, plan, original, planned) = fixture("post exchange mode drift");
    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.89.3.txn");
    fs::create_dir(&transaction).unwrap();
    write_transaction(&transaction, &original, &planned, &original);
    fs::write(&target, &planned).unwrap();
    let mode = fs::metadata(transaction.join("reviewed"))
        .unwrap()
        .permissions()
        .mode();
    fs::set_permissions(
        transaction.join("reviewed"),
        fs::Permissions::from_mode(mode ^ 0o100),
    )
    .unwrap();

    assert!(matches!(
        recover_transaction(&transaction, &target, &plan),
        Err(Error::AmbiguousEffect(_))
    ));
    assert!(transaction.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn coverage_recovery_rejects_mismatched_version_plan_and_target_shapes() {
    for state in ["version", "original", "reviewed", "target-directory"] {
        let (root, plan, original, planned) = fixture(state);
        let target = root.join("AGENTS.md");
        let transaction = root.join(".AGENTS.md.89.1.txn");
        fs::create_dir(&transaction).unwrap();
        write_transaction(&transaction, &original, &planned, &planned);
        match state {
            "version" => fs::write(transaction.join("version"), b"unknown").unwrap(),
            "original" => fs::write(transaction.join("original"), b"wrong").unwrap(),
            "reviewed" => fs::write(transaction.join("reviewed"), b"wrong").unwrap(),
            "target-directory" => {
                fs::remove_file(&target).unwrap();
                fs::create_dir(&target).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(recover_transaction(&transaction, &target, &plan).is_err());
        assert!(transaction.exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn coverage_recovery_error_types_and_legacy_classifier_are_explicit() {
    let (root, plan, _, _) = fixture("legacy");
    let transaction = root.join(".AGENTS.md.90.1.txn");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("source"), b"legacy source").unwrap();
    assert!(matches!(
        recover_transaction(&transaction, &root.join("AGENTS.md"), &plan),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_dir_all(root).unwrap();

    let error = retained_state(std::path::Path::new("transaction"), "reason");
    assert!(matches!(error, Error::AmbiguousEffect(_)));
    assert!(source_matches_plan(&plan, b"existing instructions\n"));
    let missing_root = super::temporary();
    let missing = read_optional(&missing_root.join("missing-target")).unwrap();
    assert!(missing.is_none());
    fs::remove_dir_all(missing_root).unwrap();
}

#[cfg(unix)]
#[test]
fn coverage_recovery_rejects_non_utf8_and_symlink_children() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::symlink;

    let (root, plan, _, _) = fixture("non utf8");
    let transaction = root.join(".AGENTS.md.91.1.txn");
    fs::create_dir(&transaction).unwrap();
    let non_utf8 = transaction.join(OsString::from_vec(vec![0xff]));
    if fs::write(non_utf8, b"x").is_ok() {
        assert!(recover_transaction(&transaction, &root.join("AGENTS.md"), &plan).is_err());
    }
    fs::remove_dir_all(&transaction).unwrap();
    fs::create_dir(&transaction).unwrap();
    symlink(root.join("AGENTS.md"), transaction.join("exchange")).unwrap();
    assert!(recover_transaction(&transaction, &root.join("AGENTS.md"), &plan).is_err());
    fs::remove_dir_all(root).unwrap();
}

fn fixture(name: &str) -> (std::path::PathBuf, AgentIntegrationPlan, Vec<u8>, Vec<u8>) {
    let root = temporary();
    Workspace::initialize(&root, name).unwrap();
    let original = b"existing instructions\n".to_vec();
    fs::write(root.join("AGENTS.md"), &original).unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let planned = compose_planned_instruction(
        &original,
        &plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    (root, plan, original, planned)
}

fn write_transaction(
    transaction: &std::path::Path,
    original: &[u8],
    reviewed: &[u8],
    exchanged: &[u8],
) {
    fs::write(transaction.join("version"), TRANSACTION_VERSION).unwrap();
    fs::write(transaction.join("original"), original).unwrap();
    fs::write(transaction.join("reviewed"), reviewed).unwrap();
    fs::write(transaction.join("exchange"), exchanged).unwrap();
}
