use std::fs;

use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan};
use crate::{Error, workspace::Workspace};

use super::super::super::agent_integration_content::compose_planned_instruction;
use super::super::super::agent_integration_publication::publish_instruction;
use super::super::super::{inject_storage_failure, tests::temporary};

const CHILD_ROOT: &str = "RESEARCH_RUN_FIFO_RECOVERY_CHILD_ROOT";

#[cfg(unix)]
#[test]
fn completion_recovery_rejects_fifo_survivor_without_blocking() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        exercise_fifo_recovery(std::path::Path::new(&root));
        return;
    }

    let root = temporary();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg("completion_recovery_rejects_fifo_survivor_without_blocking")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(CHILD_ROOT, &root)
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            break None;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    fs::remove_dir_all(&root).unwrap();
    let status = status.expect("completion recovery blocked on a FIFO survivor for over 5 seconds");
    assert!(status.success(), "FIFO recovery child failed: {status}");
}

#[cfg(unix)]
fn exercise_fifo_recovery(root: &std::path::Path) {
    use std::os::unix::fs::FileTypeExt;

    let (plan, original, planned) = fixture(root);
    inject_storage_failure("remove project instruction witness");
    let result = publish_instruction(
        &root.join("AGENTS.md"),
        &planned,
        AgentIntegrationOperation::Append,
        &original,
        &plan.plan_sha256,
    );
    assert!(matches!(result, Err(Error::AmbiguousEffect(_))));
    let (transaction, receipt) = completion_pair(root);
    let fifo = transaction.join("original");
    fs::remove_file(&fifo).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );

    let result = Workspace::apply_agent_integration(root, plan);
    assert!(
        matches!(result, Err(Error::AmbiguousEffect(_))),
        "{result:?}"
    );
    assert!(transaction.exists());
    assert!(receipt.exists());
    assert!(fs::symlink_metadata(fifo).unwrap().file_type().is_fifo());
}

fn fixture(root: &std::path::Path) -> (AgentIntegrationPlan, Vec<u8>, Vec<u8>) {
    Workspace::initialize(root, "FIFO survivor").unwrap();
    let original = b"existing instructions\n".to_vec();
    fs::write(root.join("AGENTS.md"), &original).unwrap();
    let plan = Workspace::plan_agent_integration(root).unwrap();
    let planned = compose_planned_instruction(
        &original,
        &plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    (plan, original, planned)
}

fn completion_pair(root: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let artifacts: Vec<_> = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy();
            name.starts_with(".AGENTS.md.") && (name.ends_with(".txn") || name.ends_with(".done"))
        })
        .collect();
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
