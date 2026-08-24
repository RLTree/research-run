#[cfg(unix)]
fn fixture(
    sequence: u64,
) -> (
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    super::agent_integration_transaction::directories::ExchangeHandles,
) {
    let root = temporary();
    let target = root.join("AGENTS.md");
    let transaction = root.join(format!(".AGENTS.md.995.{sequence}.txn"));
    std::fs::write(&target, b"original").unwrap();
    std::fs::create_dir(&transaction).unwrap();
    for name in ["version", "exchange", "original", "reviewed"] {
        std::fs::write(transaction.join(name), b"transaction evidence").unwrap();
    }
    let handles = super::agent_integration_transaction::directories::open_exchange_handles(
        &target,
        &transaction,
    )
    .unwrap();
    (root, target, transaction, handles)
}

#[cfg(unix)]
#[test]
fn staging_cleanup_removes_the_closed_witness_set_through_held_descriptors() {
    let (root, target, transaction, handles) = fixture(1);
    super::agent_integration_transaction::cleanup_staging_transaction(
        &target,
        &transaction,
        &handles,
    )
    .unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
    assert!(!transaction.exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn staging_cleanup_rejects_unexpected_or_linked_witnesses_before_deletion() {
    let (root, target, transaction, handles) = fixture(2);
    std::fs::write(transaction.join("unexpected"), b"unowned").unwrap();
    let error = super::agent_integration_transaction::cleanup_staging_transaction(
        &target,
        &transaction,
        &handles,
    )
    .expect_err("an unowned staging leaf must block cleanup");
    assert!(matches!(error, crate::Error::AmbiguousEffect(_)));
    for name in ["version", "exchange", "original", "reviewed", "unexpected"] {
        assert!(transaction.join(name).is_file(), "{name} was deleted");
    }
    std::fs::remove_dir_all(root).unwrap();

    let (root, target, transaction, handles) = fixture(3);
    let alias = root.join("exchange-alias");
    std::fs::hard_link(transaction.join("exchange"), &alias).unwrap();
    let error = super::agent_integration_transaction::cleanup_staging_transaction(
        &target,
        &transaction,
        &handles,
    )
    .expect_err("a multiply linked witness must block cleanup");
    assert!(matches!(error, crate::Error::AmbiguousEffect(_)));
    assert_eq!(std::fs::read(&alias).unwrap(), b"transaction evidence");
    for name in ["version", "exchange", "original", "reviewed"] {
        assert!(transaction.join(name).is_file(), "{name} was deleted");
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn staging_cleanup_failures_are_ambiguous_at_each_durability_boundary() {
    for (sequence, point, transaction_remains, witnesses_remain) in [
        (4, "remove abandoned pending record", true, true),
        (5, "sync project instruction staging cleanup", true, false),
        (6, "remove project instruction transaction", true, false),
        (7, "sync abandoned pending directory", false, false),
    ] {
        let (root, target, transaction, handles) = fixture(sequence);
        inject_storage_failure(point);
        let error = super::agent_integration_transaction::cleanup_staging_transaction(
            &target,
            &transaction,
            &handles,
        )
        .expect_err(point);
        assert!(matches!(error, crate::Error::AmbiguousEffect(_)), "{point}");
        assert_eq!(transaction.exists(), transaction_remains, "{point}");
        for name in ["version", "exchange", "original", "reviewed"] {
            assert_eq!(
                transaction.join(name).exists(),
                witnesses_remain,
                "{point}: {name}"
            );
        }
        assert_eq!(std::fs::read(&target).unwrap(), b"original", "{point}");
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn staging_cleanup_unlink_failure_cannot_partially_delete_witnesses() {
    use std::os::unix::fs::PermissionsExt;

    let (root, target, transaction, handles) = fixture(8);
    std::fs::set_permissions(&transaction, std::fs::Permissions::from_mode(0o500)).unwrap();
    let error = super::agent_integration_transaction::cleanup_staging_transaction(
        &target,
        &transaction,
        &handles,
    )
    .expect_err("read-only transaction directory must reject the first unlink");
    assert!(matches!(error, crate::Error::AmbiguousEffect(_)));
    for name in ["version", "exchange", "original", "reviewed"] {
        assert!(transaction.join(name).is_file(), "{name} was deleted");
    }
    std::fs::set_permissions(&transaction, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
