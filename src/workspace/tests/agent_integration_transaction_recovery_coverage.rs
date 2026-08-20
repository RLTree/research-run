use std::fs;

use crate::Error;
use crate::domain::AgentIntegrationOperation;

use super::*;
use crate::workspace::Workspace;
use crate::workspace::tests::temporary;

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
