use std::fs;

use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan};
use crate::{Error, workspace::Workspace};

use super::super::super::agent_integration_content::compose_planned_instruction;
use super::super::super::agent_integration_publication::publish_instruction;
use super::super::super::{inject_storage_failure, tests::temporary};

#[cfg(unix)]
#[test]
fn committed_cleanup_rejects_hardlinks_symlinks_extras_and_identity_conflicts() {
    use std::os::unix::fs::symlink;

    for state in [
        "hardlink",
        "symlink",
        "survivor-symlink",
        "extra",
        "canonical",
        "multiple",
        "conflict",
    ] {
        let (root, plan, original, planned) = fixture(state);
        inject_storage_failure("remove project instruction witness");
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
        let (transaction, receipt) = completion_pair(&root);
        let receipt_bytes = fs::read(&receipt).unwrap();
        match state {
            "hardlink" => fs::hard_link(&receipt, root.join("outside-receipt")).unwrap(),
            "symlink" => {
                fs::remove_file(&receipt).unwrap();
                symlink(root.join("outside-receipt"), &receipt).unwrap();
            }
            "survivor-symlink" => {
                fs::remove_file(transaction.join("original")).unwrap();
                fs::write(root.join("outside-original"), &original).unwrap();
                symlink(root.join("outside-original"), transaction.join("original")).unwrap();
            }
            "extra" => fs::write(transaction.join("unexpected"), b"x").unwrap(),
            "canonical" => fs::write(root.join("AGENTS.md"), b"changed").unwrap(),
            "multiple" => {
                fs::copy(&receipt, root.join(".AGENTS.md.880.2.done")).unwrap();
            }
            "conflict" => alter_plan_digest(&receipt, &receipt_bytes),
            _ => unreachable!(),
        }

        let result = Workspace::apply_agent_integration(&root, plan.clone());
        assert!(
            matches!(result, Err(Error::AmbiguousEffect(_))),
            "{state}: {result:?}"
        );
        assert!(transaction.exists());
        repair_test_state(
            state,
            &root,
            &transaction,
            &receipt,
            &receipt_bytes,
            &original,
            &planned,
        );
        assert!(
            !Workspace::apply_agent_integration(&root, plan)
                .unwrap()
                .changed
        );
        assert!(completion_artifacts(&root).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn receipt_only_cleanup_rejects_an_unexpected_hard_link() {
    let (root, plan, original, planned) = fixture("receipt only hardlink");
    inject_storage_failure("remove project instruction completion receipt");
    let result = publish_instruction(
        &root.join("AGENTS.md"),
        &planned,
        AgentIntegrationOperation::Append,
        &original,
        &plan.plan_sha256,
    );
    assert!(matches!(result, Err(Error::AmbiguousEffect(_))));
    let receipt = completion_artifacts(&root).pop().unwrap();
    assert_eq!(receipt.extension().unwrap(), "done");

    let alias = root.join("outside-receipt");
    fs::hard_link(&receipt, &alias).unwrap();
    let result = Workspace::apply_agent_integration(&root, plan.clone());
    assert!(matches!(result, Err(Error::AmbiguousEffect(_))));
    assert!(receipt.exists());

    fs::remove_file(alias).unwrap();
    assert!(
        !Workspace::apply_agent_integration(&root, plan)
            .unwrap()
            .changed
    );
    assert!(completion_artifacts(&root).is_empty());
    fs::remove_dir_all(root).unwrap();
}

fn alter_plan_digest(receipt: &std::path::Path, receipt_bytes: &[u8]) {
    let mut value: serde_json::Value = serde_json::from_slice(receipt_bytes).unwrap();
    value["plan_sha256"] = serde_json::json!("1".repeat(64));
    fs::write(receipt, serde_json::to_vec(&value).unwrap()).unwrap();
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

fn completion_pair(root: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let artifacts = completion_artifacts(root);
    let transaction = artifacts
        .iter()
        .find(|path| path.extension().is_some_and(|extension| extension == "txn"))
        .unwrap()
        .clone();
    let receipt = artifacts
        .into_iter()
        .find(|path| {
            path.extension()
                .is_some_and(|extension| extension == "done")
        })
        .unwrap();
    (transaction, receipt)
}

fn repair_test_state(
    state: &str,
    root: &std::path::Path,
    transaction: &std::path::Path,
    receipt: &std::path::Path,
    receipt_bytes: &[u8],
    original: &[u8],
    planned: &[u8],
) {
    match state {
        "hardlink" => fs::remove_file(root.join("outside-receipt")).unwrap(),
        "symlink" => {
            fs::remove_file(receipt).unwrap();
            fs::hard_link(transaction.join("completion"), receipt).unwrap();
        }
        "survivor-symlink" => {
            fs::remove_file(transaction.join("original")).unwrap();
            fs::write(transaction.join("original"), original).unwrap();
            fs::remove_file(root.join("outside-original")).unwrap();
        }
        "extra" => fs::remove_file(transaction.join("unexpected")).unwrap(),
        "canonical" => fs::write(root.join("AGENTS.md"), planned).unwrap(),
        "multiple" => fs::remove_file(root.join(".AGENTS.md.880.2.done")).unwrap(),
        "conflict" => fs::write(receipt, receipt_bytes).unwrap(),
        _ => unreachable!(),
    }
}
