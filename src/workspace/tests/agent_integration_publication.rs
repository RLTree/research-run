use std::fs;

use crate::Error;
use crate::domain::AgentIntegrationOperation;

use super::super::agent_integration_publication::publish_instruction;
use super::{inject_storage_failure, temporary};

#[test]
fn instruction_publication_covers_noop_races_and_atomic_failures() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    assert!(!publish_instruction(&target, b"same", AgentIntegrationOperation::NoOp, b"").unwrap());

    fs::write(&target, b"same").unwrap();
    assert!(
        !publish_instruction(&target, b"same", AgentIntegrationOperation::Create, b"").unwrap()
    );
    assert_eq!(fs::read(&target).unwrap(), b"same");
    assert!(
        publish_instruction(
            &target,
            b"different",
            AgentIntegrationOperation::Create,
            b""
        )
        .is_err()
    );
    assert_eq!(fs::read(&target).unwrap(), b"same");
    assert!(
        publish_instruction(
            &target,
            b"replacement",
            AgentIntegrationOperation::Append,
            b"same",
        )
        .unwrap()
    );
    assert_eq!(fs::read(&target).unwrap(), b"replacement");

    for point in [
        "create pending record",
        "inspect project instruction permissions",
        "replace project instructions",
        "open record directory for sync#2",
    ] {
        fs::write(&target, b"before").unwrap();
        inject_storage_failure(point);
        let error = publish_instruction(
            &target,
            b"after",
            AgentIntegrationOperation::Append,
            b"before",
        )
        .expect_err(point);
        if point == "open record directory for sync#2" {
            assert!(matches!(error, Error::AmbiguousEffect(_)), "{error:?}");
            assert_eq!(fs::read(&target).unwrap(), b"after");
        } else {
            assert!(!matches!(error, Error::AmbiguousEffect(_)), "{error:?}");
            assert_eq!(fs::read(&target).unwrap(), b"before");
        }
        remove_pending(&root);
    }
    fs::remove_file(&target).unwrap();
    inject_storage_failure("create project instructions");
    assert!(publish_instruction(&target, b"new", AgentIntegrationOperation::Create, b"").is_err());
    assert!(!target.exists());
    remove_pending(&root);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn append_publication_preserves_concurrently_observed_instruction_bytes() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    fs::write(&target, b"reviewed source").unwrap();
    inject_storage_failure("agent instruction changed before append publication");
    let error = publish_instruction(
        &target,
        b"reviewed source\nmanaged block",
        AgentIntegrationOperation::Append,
        b"reviewed source",
    )
    .expect_err("concurrent source change must fail closed");
    assert!(matches!(
        error,
        Error::Conflict(_) | Error::AmbiguousEffect(_)
    ));
    assert_eq!(fs::read(&target).unwrap(), b"concurrent instruction bytes");
    remove_pending(&root);

    fs::write(&target, b"reviewed source").unwrap();
    inject_storage_failure("agent instruction appeared during append publication");
    let error = publish_instruction(
        &target,
        b"reviewed source\nmanaged block",
        AgentIntegrationOperation::Append,
        b"reviewed source",
    )
    .expect_err("concurrent canonical claimant must fail closed");
    assert!(matches!(error, Error::AmbiguousEffect(_)), "{error:?}");
    assert_eq!(fs::read(&target).unwrap(), b"concurrent target claimant");
    let transaction = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|extension| extension == "txn"))
        .expect("preserved append transaction");
    assert_eq!(
        fs::read(transaction.join("source")).unwrap(),
        b"reviewed source"
    );
    assert_eq!(
        fs::read(transaction.join("planned")).unwrap(),
        b"reviewed source\nmanaged block"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn post_publication_cleanup_failures_are_ambiguous_effects() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    for point in [
        "remove abandoned pending record",
        "sync abandoned pending directory",
    ] {
        fs::write(&target, b"before").unwrap();
        inject_storage_failure(point);
        let error = publish_instruction(
            &target,
            b"after",
            AgentIntegrationOperation::Append,
            b"before",
        )
        .expect_err("post-publication cleanup failure");
        assert!(matches!(error, Error::AmbiguousEffect(_)), "{error:?}");
        assert_eq!(fs::read(&target).unwrap(), b"after");
        remove_pending(&root);
    }
    fs::remove_dir_all(root).unwrap();
}

fn remove_pending(root: &std::path::Path) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(".AGENTS.md.")
        {
            if path.is_dir() {
                fs::remove_dir_all(path).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
        }
    }
}
