use std::fs;

use crate::Error;
use crate::domain::AgentIntegrationOperation;

use super::*;
use crate::workspace::tests::temporary;
use crate::workspace::{Workspace, inject_storage_failure};

#[test]
fn transaction_recovery_rejects_mismatch_conflict_and_invalid_children() {
    let (root, plan, original, planned) = fixture("Recovery invalid states");
    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.3.1.txn");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("planned"), b"wrong").unwrap();
    assert!(matches!(
        recover_transaction(&transaction, &target, &plan),
        Err(Error::Conflict(_))
    ));
    fs::remove_dir_all(&transaction).unwrap();

    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("planned"), &planned).unwrap();
    fs::write(transaction.join("source"), b"concurrent").unwrap();
    fs::write(&target, b"claimant").unwrap();
    assert!(matches!(
        recover_transaction(&transaction, &target, &plan),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_dir_all(&transaction).unwrap();

    fs::create_dir(&transaction).unwrap();
    fs::create_dir(transaction.join("planned")).unwrap();
    assert!(recover_transaction(&transaction, &target, &plan).is_err());
    fs::remove_dir_all(&transaction).unwrap();
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("unexpected"), b"unexpected").unwrap();
    assert!(recover_transaction(&transaction, &target, &plan).is_err());
    assert_eq!(original, b"existing");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn transaction_recovery_covers_each_interrupted_file_combination() {
    for state in ["source-only", "planned-only", "empty", "planned-target"] {
        let (root, plan, original, planned) = fixture(state);
        let target = root.join("AGENTS.md");
        let transaction = root.join(".AGENTS.md.4.1.txn");
        fs::create_dir(&transaction).unwrap();
        match state {
            "source-only" => {
                fs::write(transaction.join("source"), &original).unwrap();
                fs::remove_file(&target).unwrap();
            }
            "planned-only" => fs::write(transaction.join("planned"), &planned).unwrap(),
            "planned-target" => {
                fs::write(transaction.join("planned"), &planned).unwrap();
                fs::write(&target, &planned).unwrap();
            }
            "empty" => {}
            _ => unreachable!(),
        }
        let result = recover_transaction(&transaction, &target, &plan);
        if state == "source-only" {
            assert!(matches!(result, Err(Error::Conflict(_))));
            assert_eq!(fs::read(&target).unwrap(), original);
        } else {
            result.unwrap();
        }
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn transaction_inspection_rejects_non_directory_and_target_directory() {
    let (root, plan, _, _) = fixture("Recovery shape");
    let target = root.join("AGENTS.md");
    let transaction = root.join("transaction");
    fs::write(&transaction, b"file").unwrap();
    assert!(recover_transaction(&transaction, &target, &plan).is_err());
    fs::remove_file(&transaction).unwrap();
    fs::remove_file(&target).unwrap();
    fs::create_dir(&target).unwrap();
    fs::create_dir(&transaction).unwrap();
    assert!(recover_transaction(&transaction, &target, &plan).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recovery_covers_restored_empty_and_conflicting_interrupted_states() {
    let (root, plan, original, planned) = fixture("Recovery state matrix");
    let target = root.join("AGENTS.md");

    let restored = root.join(".AGENTS.md.5.1.txn");
    fs::create_dir(&restored).unwrap();
    fs::write(restored.join("source"), &original).unwrap();
    recover_transaction(&restored, &target, &plan).unwrap();
    assert!(!restored.exists());

    for (sequence, with_planned) in [(2, true), (3, false)] {
        let transaction = root.join(format!(".AGENTS.md.5.{sequence}.txn"));
        fs::create_dir(&transaction).unwrap();
        if with_planned {
            fs::write(transaction.join("planned"), &planned).unwrap();
        }
        fs::remove_file(&target).unwrap();
        recover_transaction(&transaction, &target, &plan).unwrap();
        assert!(!transaction.exists());
        fs::write(&target, &original).unwrap();
    }

    let conflict = root.join(".AGENTS.md.5.4.txn");
    fs::create_dir(&conflict).unwrap();
    fs::write(conflict.join("source"), &original).unwrap();
    fs::write(conflict.join("planned"), &planned).unwrap();
    fs::write(&target, b"claimant").unwrap();
    assert!(matches!(
        recover_transaction(&conflict, &target, &plan),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recovery_inspection_and_post_link_sync_failures_are_explicit() {
    for (sequence, fault) in [
        (1, "inspect project instruction transaction"),
        (2, "inspect project instruction transaction#2"),
        (3, "inspect transaction file"),
        (4, "inspect project instruction transaction#3"),
    ] {
        let (root, plan, _, planned) = fixture(fault);
        let target = root.join("AGENTS.md");
        let transaction = root.join(format!(".AGENTS.md.6.{sequence}.txn"));
        fs::create_dir(&transaction).unwrap();
        fs::write(transaction.join("planned"), planned).unwrap();
        inject_storage_failure(fault);
        assert!(recover_transaction(&transaction, &target, &plan).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    let (root, plan, original, planned) = fixture("post-link sync");
    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.6.4.txn");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("source"), original).unwrap();
    fs::write(transaction.join("planned"), planned).unwrap();
    fs::remove_file(&target).unwrap();
    inject_storage_failure("open record directory for sync");
    assert!(matches!(
        recover_transaction(&transaction, &target, &plan),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn recovery_propagates_transaction_reads_and_concurrent_claims() {
    for (sequence, fault) in [(1, "inspect record#2"), (2, "inspect record#3")] {
        let (root, plan, original, planned) = fixture(fault);
        let target = root.join("AGENTS.md");
        let transaction = root.join(format!(".AGENTS.md.7.{sequence}.txn"));
        fs::create_dir(&transaction).unwrap();
        fs::write(transaction.join("planned"), &planned).unwrap();
        if sequence == 2 {
            fs::write(transaction.join("source"), &original).unwrap();
        }
        inject_storage_failure(fault);
        assert!(recover_transaction(&transaction, &target, &plan).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    let (root, plan, original, planned) = fixture("concurrent recovery claim");
    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.7.3.txn");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("source"), original).unwrap();
    fs::write(transaction.join("planned"), planned).unwrap();
    fs::remove_file(&target).unwrap();
    inject_storage_failure("agent instruction appeared during append publication");
    assert!(matches!(
        recover_transaction(&transaction, &target, &plan),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn concurrent_source_restoration_sync_failure_is_ambiguous() {
    let (root, plan, _, _) = fixture("concurrent source restoration");
    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.8.1.txn");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("source"), b"concurrent").unwrap();
    fs::remove_file(&target).unwrap();
    inject_storage_failure("open record directory for sync");
    assert!(matches!(
        recover_transaction(&transaction, &target, &plan),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_dir_all(root).unwrap();

    let (root, plan, original, _) = fixture("restoration target race");
    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.8.2.txn");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("source"), &original).unwrap();
    assert!(matches!(
        recover_state(&transaction, &target, &plan, None, None, Some(&original)),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn recovery_rejects_transaction_and_target_symlinks() {
    use std::os::unix::fs::symlink;

    let (root, plan, _, _) = fixture("Recovery symlinks");
    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.9.1.txn");
    symlink(root.join("missing"), &transaction).unwrap();
    assert!(recover_transaction(&transaction, &target, &plan).is_err());
    fs::remove_file(&transaction).unwrap();

    fs::create_dir(&transaction).unwrap();
    fs::remove_file(&target).unwrap();
    symlink(root.join("outside"), &target).unwrap();
    assert!(recover_transaction(&transaction, &target, &plan).is_err());
    fs::remove_dir_all(root).unwrap();
}

fn fixture(name: &str) -> (PathBuf, AgentIntegrationPlan, Vec<u8>, Vec<u8>) {
    let root = temporary();
    Workspace::initialize(&root, name).unwrap();
    let original = b"existing".to_vec();
    fs::write(root.join("AGENTS.md"), &original).unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let planned = super::super::agent_integration_content::compose_planned_instruction(
        &original,
        &plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    (root, plan, original, planned)
}
