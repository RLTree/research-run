use std::fs;

use crate::Error;
use crate::domain::AgentIntegrationOperation;

use super::super::agent_integration_content::compose_planned_instruction;
use super::super::agent_integration_publication::publish_instruction;
use super::super::agent_integration_transaction::witnesses::TRANSACTION_VERSION;
use super::super::agent_integration_transaction_recovery::recover_transaction;
use super::{Workspace, inject_storage_failure, temporary};

const TEST_PLAN_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn append_publication_keeps_the_canonical_instruction_path_bound() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    fs::write(&target, b"before").unwrap();
    inject_storage_failure("probe continuous agent instruction path");

    let changed = publish_instruction(
        &target,
        b"after",
        AgentIntegrationOperation::Append,
        b"before",
        TEST_PLAN_SHA256,
    )
    .expect("append must never expose an unbound canonical path");

    assert!(changed);
    assert_eq!(fs::read(&target).unwrap(), b"after");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn planned_only_missing_target_is_ambiguous_and_preserves_recovery_bytes() {
    let root = temporary();
    Workspace::initialize(&root, "Missing append target recovery").unwrap();
    fs::write(root.join("AGENTS.md"), b"before").unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let planned = compose_planned_instruction(
        b"before",
        &plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    let transaction = root.join(".AGENTS.md.899.1.txn");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("planned"), &planned).unwrap();
    fs::remove_file(root.join("AGENTS.md")).unwrap();

    let error = recover_transaction(&transaction, &root.join("AGENTS.md"), &plan).unwrap_err();
    assert!(matches!(error, Error::AmbiguousEffect(_)));
    assert!(
        error
            .to_string()
            .contains("legacy project instruction transaction")
    );
    assert_eq!(fs::read(transaction.join("planned")).unwrap(), planned);
    assert!(transaction.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn versioned_append_recovery_completes_exact_states_and_retains_unknown_bytes() {
    let root = temporary();
    Workspace::initialize(&root, "Append transaction recovery").unwrap();
    let original = b"# Existing\n";
    fs::write(root.join("AGENTS.md"), original).unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let planned = compose_planned_instruction(
        original,
        &plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    let transaction = root.join(".AGENTS.md.900.1.txn");
    fs::create_dir(&transaction).unwrap();
    write_transaction(&transaction, original, &planned, &planned);

    let result = Workspace::apply_agent_integration(&root, plan).unwrap();
    assert!(!result.changed);
    assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), planned);
    assert!(!transaction.exists());

    fs::write(root.join("AGENTS.md"), original).unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let planned = compose_planned_instruction(
        original,
        &plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    fs::create_dir(&transaction).unwrap();
    write_transaction(&transaction, original, &planned, b"concurrent source");
    fs::write(root.join("AGENTS.md"), &planned).unwrap();

    assert!(matches!(
        Workspace::apply_agent_integration(&root, plan),
        Err(Error::AmbiguousEffect(_))
    ));
    assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), planned);
    assert_eq!(
        fs::read(transaction.join("exchange")).unwrap(),
        b"concurrent source"
    );
    assert!(transaction.exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn append_transaction_symlink_content_fails_closed_without_path_escape() {
    use std::os::unix::fs::symlink;

    let root = temporary();
    Workspace::initialize(&root, "Append transaction symlink").unwrap();
    let original = b"# Existing\n";
    fs::write(root.join("AGENTS.md"), original).unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let planned = compose_planned_instruction(
        original,
        &plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    let outside = root.join("outside");
    fs::write(&outside, &planned).unwrap();
    let transaction = root.join(".AGENTS.md.901.1.txn");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("version"), TRANSACTION_VERSION).unwrap();
    fs::write(transaction.join("original"), original).unwrap();
    fs::write(transaction.join("reviewed"), &planned).unwrap();
    symlink(&outside, transaction.join("exchange")).unwrap();

    let error = Workspace::apply_agent_integration(&root, plan).unwrap_err();
    let Error::AmbiguousEffect(message) = error else {
        panic!("expected retained symlink ambiguity, observed {error:?}");
    };
    assert!(message.contains("symlink is forbidden"), "{message}");
    assert_eq!(fs::read(&outside).unwrap(), planned);
    assert!(transaction.exists());
    assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), original);
    fs::remove_dir_all(root).unwrap();
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
