use std::fs;

use crate::Error;

#[cfg(target_os = "linux")]
use super::super::super::agent_integration_transaction::create_transaction;
use super::super::super::agent_integration_transaction::directory_creation::{
    create_transaction_directory, transaction_directory_permissions_are_private,
};
use super::super::{inject_storage_failure, temporary};

#[cfg(unix)]
#[test]
fn transaction_creation_fails_before_effect_when_parent_cannot_be_observed() {
    let root = temporary();
    let transaction = root.join(".AGENTS.md.989.1.txn");
    inject_storage_failure("inspect project instruction transaction parent");
    let error = create_transaction_directory(&transaction).unwrap_err();

    assert!(matches!(
        error,
        Error::Io {
            action: "inspect project instruction transaction parent",
            ..
        }
    ));
    assert!(!transaction.exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn transaction_creation_retains_ambiguous_post_mkdir_effects() {
    for (sequence, point) in [
        (1, "inspect project instruction transaction permissions"),
        (2, "open record directory for sync"),
        (3, "sync record directory"),
    ] {
        let root = temporary();
        let transaction = root.join(format!(".AGENTS.md.989.{sequence}.txn"));
        inject_storage_failure(point);
        let error = create_transaction_directory(&transaction).unwrap_err();

        assert!(
            matches!(error, Error::AmbiguousEffect(_)),
            "{point}: {error:?}"
        );
        assert!(
            transaction.is_dir(),
            "{point}: transaction evidence missing"
        );
        assert_eq!(fs::read_dir(&transaction).unwrap().count(), 0, "{point}");
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(target_os = "linux")]
#[test]
fn setgid_project_root_keeps_transaction_access_private() {
    use std::os::unix::fs::PermissionsExt;

    let root = temporary();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o2770)).unwrap();
    let root_mode = fs::metadata(&root).unwrap().permissions().mode() & 0o7777;
    assert_eq!(root_mode & 0o2000, 0o2000, "setgid precondition missing");
    let transaction = root.join(".AGENTS.md.990.1.txn");
    create_transaction(&transaction).unwrap();
    let mode = fs::metadata(&transaction).unwrap().permissions().mode() & 0o7777;
    assert_eq!(mode & 0o2000, 0o2000, "setgid inheritance missing");
    assert_eq!(mode & 0o077, 0, "group/other transaction access: {mode:o}");
    assert_eq!(mode & 0o5000, 0, "unexpected special bits: {mode:o}");
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
