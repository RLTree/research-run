use std::fs;

use crate::Error;
use crate::domain::AgentIntegrationOperation;

use super::super::agent_integration_content::compose_planned_instruction;
use super::{Workspace, temporary};

#[test]
fn append_transaction_recovery_completes_or_restores_without_losing_bytes() {
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
    fs::write(transaction.join("planned"), &planned).unwrap();
    fs::rename(root.join("AGENTS.md"), transaction.join("source")).unwrap();

    let result = Workspace::apply_agent_integration(&root, plan).unwrap();
    assert!(!result.changed);
    assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), planned);
    assert!(!transaction.exists());

    fs::remove_file(root.join("AGENTS.md")).unwrap();
    fs::write(root.join("AGENTS.md"), original).unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let planned = compose_planned_instruction(
        original,
        &plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("planned"), planned).unwrap();
    fs::write(transaction.join("source"), b"concurrent source").unwrap();
    fs::remove_file(root.join("AGENTS.md")).unwrap();

    assert!(matches!(
        Workspace::apply_agent_integration(&root, plan),
        Err(Error::Conflict(_))
    ));
    assert_eq!(
        fs::read(root.join("AGENTS.md")).unwrap(),
        b"concurrent source"
    );
    assert!(!transaction.exists());
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
    symlink(&outside, transaction.join("planned")).unwrap();
    fs::rename(root.join("AGENTS.md"), transaction.join("source")).unwrap();

    assert!(Workspace::apply_agent_integration(&root, plan).is_err());
    assert_eq!(fs::read(&outside).unwrap(), planned);
    assert!(transaction.exists());
    assert!(!root.join("AGENTS.md").exists());
    fs::remove_dir_all(root).unwrap();
}
