use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::AgentIntegrationOperation;

use super::super::agent_integration::private_staging::create_private_file;
use super::super::agent_integration_publication::publish_instruction;
use super::super::agent_integration_transaction::create_transaction;
use super::super::agent_integration_transaction::directories::open_exchange_handles;
use super::super::agent_integration_transaction::receipt_io::stage_receipt;
use super::super::agent_integration_transaction::witnesses::{
    WitnessPaths, inspect_independent_append_source, stage_witnesses,
};
use super::super::inject_storage_failure;

const CHILD_ROOT: &str = "RESEARCH_RUN_PERMISSION_CHILD_ROOT";
const CHILD_COMPLETE: &str = ".permission-child-complete";
const TEST_PLAN_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[path = "agent_integration_transaction_creation.rs"]
mod creation;

#[cfg(unix)]
#[test]
fn private_agent_integration_staging_respects_permission_ceilings() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        exercise_permission_boundaries(Path::new(&root));
        return;
    }
    let mut failed = Vec::new();
    let module = module_path!()
        .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
        .unwrap_or(module_path!());
    let filter = format!(
        "{}::{}",
        module,
        stringify!(private_agent_integration_staging_respects_permission_ceilings)
    );
    for mask in ["022", "000"] {
        let root = child_root(mask);
        let status = Command::new("sh")
            .arg("-c")
            .arg("umask \"$1\"; exec \"$2\" \"$3\" --exact --nocapture --test-threads=1")
            .arg("research-run-permission-test")
            .arg(mask)
            .arg(std::env::current_exe().unwrap())
            .arg(&filter)
            .env(CHILD_ROOT, &root)
            .status()
            .unwrap();
        if !status.success() || !root.join(CHILD_COMPLETE).is_file() {
            failed.push(mask);
        }
        let _ = fs::remove_dir_all(&root);
    }
    assert!(failed.is_empty(), "permission children failed: {failed:?}");
}

#[cfg(unix)]
fn exercise_permission_boundaries(root: &Path) {
    use std::os::unix::fs::PermissionsExt;

    fs::create_dir_all(root).unwrap();
    let target = root.join("AGENTS.md");
    fs::write(&target, b"original").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
    let (transaction, paths, pending) = stage_private_intermediates(root, &target);
    assert_permission_ceilings(&transaction, &paths, &pending);
    assert_exact_witness_modes(root, &target);
    fs::write(root.join(CHILD_COMPLETE), b"complete").unwrap();
}

#[cfg(unix)]
fn stage_private_intermediates(root: &Path, target: &Path) -> (PathBuf, WitnessPaths, PathBuf) {
    let transaction = root.join(".AGENTS.md.991.1.txn");
    create_transaction(&transaction).unwrap();
    let paths = WitnessPaths::new(&transaction);
    let handles = open_exchange_handles(target, &transaction).unwrap();
    let source_metadata = inspect_independent_append_source(target).unwrap();
    inject_storage_failure("preserve project instruction Unix mode");
    assert!(
        stage_witnesses(
            target,
            &transaction,
            &handles,
            &paths,
            &source_metadata,
            b"original",
            b"reviewed",
        )
        .is_err()
    );
    stage_receipt(&handles.transaction, &transaction, b"completion").unwrap();
    assert_private_create_collision(&handles.transaction, &transaction);

    let create_target = root.join("AGENTS.override.md");
    fs::write(&create_target, b"conflict").unwrap();
    inject_storage_failure("remove abandoned pending record");
    assert!(
        publish_instruction(
            &create_target,
            b"private pending instructions",
            AgentIntegrationOperation::Create,
            b"",
            TEST_PLAN_SHA256,
        )
        .is_err()
    );
    let pending = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".AGENTS.override.md.")
        })
        .unwrap();
    (transaction, paths, pending)
}

#[cfg(unix)]
fn assert_permission_ceilings(transaction: &Path, paths: &WitnessPaths, pending: &Path) {
    let mut violations = Vec::new();
    check_ceiling(transaction, 0o700, &mut violations);
    for path in [
        &paths.version,
        &paths.original,
        &paths.reviewed,
        &paths.exchange,
        &transaction.join("completion"),
        pending,
    ] {
        check_ceiling(path, 0o600, &mut violations);
    }
    assert!(
        violations.is_empty(),
        "private staging permission violations: {}",
        violations.join(", ")
    );
}

#[cfg(unix)]
fn assert_exact_witness_modes(root: &Path, target: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let exact_transaction = root.join(".AGENTS.md.992.1.txn");
    create_transaction(&exact_transaction).unwrap();
    let exact_paths = WitnessPaths::new(&exact_transaction);
    let exact_handles = open_exchange_handles(target, &exact_transaction).unwrap();
    let source_metadata = inspect_independent_append_source(target).unwrap();
    stage_witnesses(
        target,
        &exact_transaction,
        &exact_handles,
        &exact_paths,
        &source_metadata,
        b"original",
        b"reviewed",
    )
    .unwrap();
    let expected = fs::metadata(target).unwrap().permissions().mode();
    for path in [
        &exact_paths.original,
        &exact_paths.reviewed,
        &exact_paths.exchange,
    ] {
        assert_eq!(fs::metadata(path).unwrap().permissions().mode(), expected);
    }
}

#[cfg(unix)]
fn assert_private_create_collision(transaction: &fs::File, path: &Path) {
    assert!(
        create_private_file(transaction, &path.join("completion")).is_err(),
        "private staging must preserve create-only witness identity"
    );
}

#[cfg(unix)]
fn check_ceiling(path: &Path, ceiling: u32, violations: &mut Vec<String>) {
    use std::os::unix::fs::PermissionsExt;

    let mode = fs::symlink_metadata(path).unwrap().permissions().mode() & 0o7777;
    if mode & !ceiling != 0 {
        violations.push(format!("{}={mode:o}>{ceiling:o}", path.display()));
    }
}

#[cfg(unix)]
fn child_root(mask: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "research-run-permission-test-{}-{mask}",
        std::process::id()
    ))
}
