use std::fs;

use crate::Error;

use super::*;
use crate::workspace::inject_storage_failure;
use crate::workspace::tests::temporary;

#[test]
fn append_transaction_storage_failures_preserve_explicit_effect_state() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    fs::write(&target, b"before").unwrap();
    inject_storage_failure("inspect record");
    assert!(matches!(
        publish_append(&target, b"after", b"before"),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_dir_all(&root).unwrap();
    fs::create_dir(&root).unwrap();
    fs::write(&target, b"before").unwrap();
    inject_storage_failure("open record directory for sync");
    assert!(publish_append(&target, b"after", b"before").is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn link_verify_restore_and_cleanup_cover_each_fail_closed_branch() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    let planned = root.join("planned");
    assert!(matches!(
        link_planned(&target, &planned),
        Err(Error::AmbiguousEffect(_))
    ));

    fs::write(&planned, b"planned").unwrap();
    assert!(verify_published(&target, &planned, b"planned", b"planned").is_err());
    fs::write(&target, b"other").unwrap();
    assert!(verify_published(&target, &planned, b"planned", b"planned").is_err());

    let transaction = root.join("transaction");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("source"), b"source").unwrap();
    assert!(matches!(
        restore_source(&target, &transaction.join("source"), &transaction),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_file(&target).unwrap();
    fs::remove_file(transaction.join("source")).unwrap();
    assert!(restore_source(&target, &transaction.join("source"), &transaction).is_err());
    assert!(cleanup_after_publication(&target, &root.join("missing")).is_ok());

    fs::write(transaction.join("unexpected"), b"keeps directory nonempty").unwrap();
    assert!(cleanup_after_publication(&target, &transaction).is_err());
    let original = Error::Conflict("original".to_owned());
    assert!(matches!(
        abort_transaction(&transaction, original),
        Error::AmbiguousEffect(_)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn transaction_collision_and_restoration_ambiguity_fail_closed() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    let transaction = root.join("transaction");
    fs::create_dir(&transaction).unwrap();
    assert!(create_transaction(&transaction).is_err());

    let source = transaction.join("source");
    fs::write(&source, b"source").unwrap();
    fs::write(&target, b"claimant").unwrap();
    let original = Error::Conflict("original".to_owned());
    assert!(matches!(
        restore_after_failure(&target, &source, &transaction, original),
        Error::AmbiguousEffect(_)
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn verification_restoration_and_cleanup_failures_are_ambiguous() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    let missing_source = root.join("missing-source");
    fs::write(&target, b"planned").unwrap();
    assert!(matches!(
        verify_published(&target, &missing_source, b"planned", b"source"),
        Err(Error::AmbiguousEffect(_))
    ));

    let transaction = root.join("transaction");
    fs::create_dir(&transaction).unwrap();
    let source = transaction.join("source");
    fs::write(&source, b"source").unwrap();
    fs::remove_file(&target).unwrap();
    inject_storage_failure("open record directory for sync");
    assert!(matches!(
        restore_source(&target, &source, &transaction),
        Err(Error::AmbiguousEffect(_))
    ));

    fs::remove_file(&target).unwrap();
    inject_storage_failure("remove abandoned pending record");
    assert!(matches!(
        cleanup_after_publication(&target, &transaction),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_dir_all(root).unwrap();

    let root = temporary();
    let target = root.join("AGENTS.md");
    let transaction = root.join("transaction");
    fs::create_dir(&transaction).unwrap();
    fs::write(transaction.join("planned"), b"planned").unwrap();
    fs::write(transaction.join("source"), b"source").unwrap();
    inject_storage_failure("open record directory for sync#3");
    assert!(matches!(
        cleanup_after_publication(&target, &transaction),
        Err(Error::AmbiguousEffect(_))
    ));
    assert!(!transaction.exists());
    fs::remove_dir_all(root).unwrap();
}
