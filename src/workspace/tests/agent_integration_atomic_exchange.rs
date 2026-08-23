use std::fs;

use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan};
use crate::{Error, workspace::Workspace};

use super::super::super::agent_integration_content::compose_planned_instruction;
use super::super::super::agent_integration_publication::publish_instruction;
use super::super::super::agent_integration_transaction_recovery::recover_transaction;
use super::super::super::{inject_storage_failure, tests::temporary};
use super::super::witnesses::TRANSACTION_VERSION;

#[test]
fn exchange_and_durability_faults_retain_exact_retryable_states_without_fallback() {
    for point in [
        "atomic exchange project instructions",
        "sync transaction witness before exchange#5",
        "sync project instruction transaction after exchange",
        "sync project instruction root after exchange",
        "inspect project instruction directory anchor#4",
        "inspect project instruction directory anchor#5",
    ] {
        let (root, plan, original, planned) = fixture(point);
        inject_storage_failure(point);
        let error = publish_instruction(
            &root.join("AGENTS.md"),
            &planned,
            AgentIntegrationOperation::Append,
            &original,
            &plan.plan_sha256,
        )
        .expect_err(point);
        assert!(
            matches!(error, Error::AmbiguousEffect(_)),
            "{point}: {error:?}"
        );
        let transaction = find_transaction(&root);
        assert_eq!(fs::read(transaction.join("original")).unwrap(), original);
        assert_eq!(fs::read(transaction.join("reviewed")).unwrap(), planned);
        if point == "atomic exchange project instructions" {
            assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), original);
            assert_eq!(fs::read(transaction.join("exchange")).unwrap(), planned);
        }

        let result = Workspace::apply_agent_integration(&root, plan).unwrap();
        assert!(!result.changed);
        assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), planned);
        assert!(!transaction.exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn post_exchange_canonical_mutation_is_retained_as_an_impossible_state() {
    let (root, plan, original, planned) = fixture("post exchange mutation");
    inject_storage_failure("agent instruction changed after atomic exchange");

    let error = publish_instruction(
        &root.join("AGENTS.md"),
        &planned,
        AgentIntegrationOperation::Append,
        &original,
        &plan.plan_sha256,
    )
    .expect_err("post-exchange mutation must be ambiguous");

    assert!(matches!(error, Error::AmbiguousEffect(_)));
    assert_eq!(
        fs::read(root.join("AGENTS.md")).unwrap(),
        b"post-exchange instruction bytes"
    );
    let transaction = find_transaction(&root);
    assert_eq!(fs::read(transaction.join("original")).unwrap(), original);
    assert_eq!(fs::read(transaction.join("reviewed")).unwrap(), planned);
    assert_eq!(fs::read(transaction.join("exchange")).unwrap(), original);
    assert!(matches!(
        Workspace::apply_agent_integration(&root, plan),
        Err(Error::AmbiguousEffect(_))
    ));
    assert!(transaction.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn staged_file_and_pre_exchange_sync_failures_leave_the_original_canonical() {
    for point in [
        "sync pending record",
        "sync pending record#2",
        "sync pending record#3",
        "sync pending record#4",
        "sync project instruction transaction before exchange",
        "sync project instruction root before exchange",
        "inspect record#5",
        "inspect project instruction directory anchor",
        "replace project instructions",
    ] {
        let (root, plan, original, planned) = fixture(point);
        inject_storage_failure(point);
        assert!(
            publish_instruction(
                &root.join("AGENTS.md"),
                &planned,
                AgentIntegrationOperation::Append,
                &original,
                &plan.plan_sha256,
            )
            .is_err()
        );
        assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), original);
        assert!(find_transactions(&root).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn recovery_rejects_incomplete_unknown_aliased_and_mode_drift_states() {
    for state in [
        "empty",
        "missing",
        "extra",
        "unknown",
        "missing-target",
        "bad-version",
        "alias",
        "mode",
    ] {
        let (root, plan, original, planned) = fixture(state);
        let target = root.join("AGENTS.md");
        let transaction = root.join(".AGENTS.md.777.1.txn");
        fs::create_dir(&transaction).unwrap();
        if state != "empty" {
            write_transaction(&transaction, &original, &planned, &planned);
        }
        match state {
            "empty" => {}
            "missing" => fs::remove_file(transaction.join("reviewed")).unwrap(),
            "extra" => fs::write(transaction.join("unexpected"), b"x").unwrap(),
            "unknown" => fs::write(&target, b"unknown canonical").unwrap(),
            "missing-target" => fs::remove_file(&target).unwrap(),
            "bad-version" => fs::write(transaction.join("version"), b"v999").unwrap(),
            "alias" => {
                fs::remove_file(transaction.join("reviewed")).unwrap();
                fs::hard_link(transaction.join("exchange"), transaction.join("reviewed")).unwrap();
            }
            "mode" => change_mode(&transaction.join("reviewed")),
            _ => unreachable!(),
        }
        assert!(matches!(
            recover_transaction(&transaction, &target, &plan),
            Err(Error::AmbiguousEffect(_))
        ));
        assert!(transaction.exists());
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(unix)]
fn change_mode(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let current = fs::metadata(path).unwrap().permissions().mode();
    fs::set_permissions(path, fs::Permissions::from_mode(current ^ 0o100)).unwrap();
}

#[cfg(not(unix))]
fn change_mode(_: &std::path::Path) {}

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

fn find_transaction(root: &std::path::Path) -> std::path::PathBuf {
    find_transactions(root).pop().expect("retained transaction")
}

fn find_transactions(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "txn"))
        .collect()
}
