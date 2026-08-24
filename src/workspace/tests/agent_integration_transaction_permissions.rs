use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::AgentIntegrationOperation;

use super::super::agent_integration::private_staging::create_private_file;
use super::super::agent_integration_publication::publish_instruction;
use super::super::agent_integration_transaction::create_transaction;
use super::super::agent_integration_transaction::directories::open_directory;
#[cfg(unix)]
use super::super::agent_integration_transaction::directories::transaction_directory_permissions_are_private;
use super::super::agent_integration_transaction::receipt_io::stage_receipt;
use super::super::agent_integration_transaction::witnesses::{WitnessPaths, stage_witnesses};
use super::super::inject_storage_failure;

const CHILD_ROOT: &str = "RESEARCH_RUN_PERMISSION_CHILD_ROOT";
const TEST_FILTER: &str = "private_agent_integration_staging_respects_permission_ceilings";
const TEST_PLAN_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[cfg(target_os = "linux")]
#[test]
fn setgid_project_root_keeps_transaction_access_private() {
    use std::os::unix::fs::PermissionsExt;

    let root = child_root("setgid");
    fs::create_dir_all(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o2770)).unwrap();
    let root_mode = fs::metadata(&root).unwrap().permissions().mode() & 0o7777;
    assert_eq!(root_mode & 0o2000, 0o2000, "setgid precondition missing");

    let transaction = root.join(".AGENTS.md.990.1.txn");
    create_transaction(&transaction).unwrap();
    let mode = fs::metadata(&transaction).unwrap().permissions().mode() & 0o7777;
    assert_eq!(mode & 0o2000, 0o2000, "setgid inheritance missing");
    assert_eq!(mode & 0o077, 0, "group/other transaction access: {mode:o}");
    assert_eq!(
        mode & 0o5000,
        0,
        "unexpected transaction special bits: {mode:o}"
    );

    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn transaction_directory_permission_predicate_is_strict() {
    let private = |parent_mode, parent_gid, transaction_mode, transaction_gid, is_directory| {
        transaction_directory_permissions_are_private(
            parent_mode,
            parent_gid,
            transaction_mode,
            transaction_gid,
            is_directory,
        )
    };
    assert!(private(0o700, 10, 0o700, 10, true));
    assert!(private(0o700, 10, 0o500, 10, true));
    for mode in [0o740, 0o704, 0o4700, 0o1700, 0o6700] {
        assert!(!private(0o2700, 10, mode, 10, true), "mode {mode:o}");
    }
    assert!(!private(0o700, 10, 0o700, 10, false));

    #[cfg(target_os = "linux")]
    {
        assert!(private(0o2700, 10, 0o2700, 10, true));
        assert!(!private(0o700, 10, 0o2700, 10, true));
        assert!(!private(0o2700, 10, 0o2700, 11, true));
    }
    #[cfg(not(target_os = "linux"))]
    assert!(!private(0o2700, 10, 0o2700, 10, true));
}

#[cfg(unix)]
#[test]
fn private_agent_integration_staging_respects_permission_ceilings() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        exercise_permission_boundaries(Path::new(&root));
        return;
    }
    let mut failed = Vec::new();
    for mask in ["022", "000"] {
        let root = child_root(mask);
        let status = Command::new("sh")
            .arg("-c")
            .arg("umask \"$1\"; exec \"$2\" \"$3\" --nocapture --test-threads=1")
            .arg("research-run-permission-test")
            .arg(mask)
            .arg(std::env::current_exe().unwrap())
            .arg(TEST_FILTER)
            .env(CHILD_ROOT, &root)
            .status()
            .unwrap();
        let _ = fs::remove_dir_all(&root);
        if !status.success() {
            failed.push(mask);
        }
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
    let transaction = root.join(".AGENTS.md.991.1.txn");
    create_transaction(&transaction).unwrap();
    let paths = WitnessPaths::new(&transaction);
    inject_storage_failure("preserve project instruction Unix mode");
    assert!(stage_witnesses(&target, &transaction, &paths, b"original", b"reviewed").is_err());
    let transaction_handle = open_directory(&transaction).unwrap();
    stage_receipt(&transaction_handle, &transaction, b"completion").unwrap();
    assert_private_create_collision(&transaction_handle, &transaction);

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

    let mut violations = Vec::new();
    check_ceiling(&transaction, 0o700, &mut violations);
    for path in [
        &paths.version,
        &paths.original,
        &paths.reviewed,
        &paths.exchange,
        &transaction.join("completion"),
        &pending,
    ] {
        check_ceiling(path, 0o600, &mut violations);
    }
    assert!(
        violations.is_empty(),
        "private staging permission violations: {}",
        violations.join(", ")
    );

    let exact_transaction = root.join(".AGENTS.md.992.1.txn");
    create_transaction(&exact_transaction).unwrap();
    let exact_paths = WitnessPaths::new(&exact_transaction);
    stage_witnesses(
        &target,
        &exact_transaction,
        &exact_paths,
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
