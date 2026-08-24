use std::fs;

use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan};
use crate::{Error, workspace::Workspace};

use super::super::super::agent_integration_content::compose_planned_instruction;
use super::super::super::{inject_storage_failure, tests::temporary};
use super::super::witnesses::{PREVIOUS_TRANSACTION_VERSION, TRANSACTION_VERSION};

#[test]
fn staged_receipt_recovery_repairs_only_an_exact_post_exchange_state() {
    for state in ["post", "pre", "partial"] {
        let (root, plan, original, planned) = fixture(state);
        let transaction = root.join(".AGENTS.md.777.1.txn");
        fs::create_dir(&transaction).unwrap();
        let exchanged = if state == "pre" { &planned } else { &original };
        write_transaction(
            &transaction,
            TRANSACTION_VERSION,
            &original,
            &planned,
            exchanged,
        );
        fs::write(transaction.join("completion"), b"{").unwrap();
        if state != "pre" {
            fs::write(root.join("AGENTS.md"), &planned).unwrap();
        }
        if state == "partial" {
            fs::remove_file(transaction.join("reviewed")).unwrap();
        }

        let result = Workspace::apply_agent_integration(&root, plan);
        if state == "post" {
            assert!(!result.unwrap().changed);
            assert!(completion_artifacts(&root).is_empty());
        } else {
            assert!(matches!(result, Err(Error::AmbiguousEffect(_))), "{state}");
            assert!(transaction.exists(), "{state}");
        }
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn previous_full_transaction_version_remains_recoverable() {
    for (state, fault) in [
        ("pre", None),
        ("pre", Some("sync project instruction completion receipt")),
        ("post", None),
        ("post", Some("sync project instruction completion receipt")),
    ] {
        let (root, plan, original, planned) = fixture("previous transaction version");
        let transaction = root.join(".AGENTS.md.778.1.txn");
        fs::create_dir(&transaction).unwrap();
        let exchanged = if state == "pre" { &planned } else { &original };
        write_transaction(
            &transaction,
            PREVIOUS_TRANSACTION_VERSION,
            &original,
            &planned,
            exchanged,
        );
        if state == "post" {
            fs::write(root.join("AGENTS.md"), &planned).unwrap();
        }

        if let Some(fault) = fault {
            inject_storage_failure(fault);
            assert!(matches!(
                Workspace::apply_agent_integration(&root, plan.clone()),
                Err(Error::AmbiguousEffect(_))
            ));
            assert_eq!(transaction_names(&transaction).len(), 5);
        }
        assert!(
            !Workspace::apply_agent_integration(&root, plan)
                .unwrap()
                .changed
        );
        assert!(completion_artifacts(&root).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn post_exchange_recovery_open_failure_is_ambiguous_and_retryable() {
    let (root, plan, original, planned) = fixture("post exchange recovery open");
    let transaction = root.join(".AGENTS.md.780.1.txn");
    fs::create_dir(&transaction).unwrap();
    write_transaction(
        &transaction,
        TRANSACTION_VERSION,
        &original,
        &planned,
        &original,
    );
    fs::write(root.join("AGENTS.md"), &planned).unwrap();

    inject_storage_failure("open recovered project instruction transaction after exchange");
    let result = Workspace::apply_agent_integration(&root, plan.clone());
    assert!(
        matches!(result, Err(Error::AmbiguousEffect(_))),
        "{result:?}"
    );
    assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), planned);
    assert_eq!(transaction_names(&transaction).len(), 4);

    assert!(
        !Workspace::apply_agent_integration(&root, plan)
            .unwrap()
            .changed
    );
    assert!(completion_artifacts(&root).is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_terminal_receipt_never_authorizes_transaction_cleanup() {
    let (root, plan, original, planned) = fixture("malformed terminal receipt");
    let transaction = root.join(".AGENTS.md.779.1.txn");
    fs::create_dir(&transaction).unwrap();
    write_transaction(
        &transaction,
        TRANSACTION_VERSION,
        &original,
        &planned,
        &original,
    );
    fs::write(root.join("AGENTS.md"), &planned).unwrap();
    let receipt = root.join(".AGENTS.md.779.1.done");
    fs::write(&receipt, b"{").unwrap();

    assert!(matches!(
        Workspace::apply_agent_integration(&root, plan),
        Err(Error::AmbiguousEffect(_))
    ));
    assert_eq!(transaction_names(&transaction).len(), 4);
    assert!(receipt.exists());
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

fn completion_artifacts(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy();
            name.starts_with(".AGENTS.md.") && (name.ends_with(".txn") || name.ends_with(".done"))
        })
        .collect()
}

fn transaction_names(transaction: &std::path::Path) -> Vec<String> {
    fs::read_dir(transaction)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

fn write_transaction(
    transaction: &std::path::Path,
    version: &[u8],
    original: &[u8],
    reviewed: &[u8],
    exchange: &[u8],
) {
    fs::write(transaction.join("version"), version).unwrap();
    fs::write(transaction.join("original"), original).unwrap();
    fs::write(transaction.join("reviewed"), reviewed).unwrap();
    fs::write(transaction.join("exchange"), exchange).unwrap();
}
