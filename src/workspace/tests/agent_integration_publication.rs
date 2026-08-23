use std::fs;

use crate::Error;
use crate::domain::AgentIntegrationOperation;

use super::super::agent_integration_publication::publish_instruction;
use super::{inject_storage_failure, temporary};

const TEST_PLAN_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn instruction_publication_covers_noop_races_and_atomic_failures() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    assert_basic_publication(&target);
    assert_publication_faults(&root, &target);
    assert_create_failure(&root, &target);
    fs::remove_dir_all(root).unwrap();
}

fn assert_basic_publication(target: &std::path::Path) {
    assert!(
        !publish_instruction(
            target,
            b"same",
            AgentIntegrationOperation::NoOp,
            b"",
            TEST_PLAN_SHA256,
        )
        .unwrap()
    );

    fs::write(target, b"same").unwrap();
    assert!(
        !publish_instruction(
            target,
            b"same",
            AgentIntegrationOperation::Create,
            b"",
            TEST_PLAN_SHA256,
        )
        .unwrap()
    );
    assert_eq!(fs::read(target).unwrap(), b"same");
    assert!(
        publish_instruction(
            target,
            b"different",
            AgentIntegrationOperation::Create,
            b"",
            TEST_PLAN_SHA256,
        )
        .is_err()
    );
    assert_eq!(fs::read(target).unwrap(), b"same");
    assert!(
        publish_instruction(
            target,
            b"replacement",
            AgentIntegrationOperation::Append,
            b"same",
            TEST_PLAN_SHA256,
        )
        .unwrap()
    );
    assert_eq!(fs::read(target).unwrap(), b"replacement");
}

fn assert_publication_faults(root: &std::path::Path, target: &std::path::Path) {
    for point in [
        "create pending record",
        "inspect project instruction permissions",
        "replace project instructions",
        "sync project instruction root after exchange",
    ] {
        fs::write(target, b"before").unwrap();
        inject_storage_failure(point);
        let error = publish_instruction(
            target,
            b"after",
            AgentIntegrationOperation::Append,
            b"before",
            TEST_PLAN_SHA256,
        )
        .expect_err(point);
        if point == "sync project instruction root after exchange" {
            assert!(matches!(error, Error::AmbiguousEffect(_)), "{error:?}");
            assert_eq!(fs::read(target).unwrap(), b"after");
        } else {
            assert!(!matches!(error, Error::AmbiguousEffect(_)), "{error:?}");
            assert_eq!(fs::read(target).unwrap(), b"before");
        }
        remove_pending(root);
    }
}

fn assert_create_failure(root: &std::path::Path, target: &std::path::Path) {
    fs::remove_file(target).unwrap();
    inject_storage_failure("create project instructions");
    assert!(
        publish_instruction(
            target,
            b"new",
            AgentIntegrationOperation::Create,
            b"",
            TEST_PLAN_SHA256,
        )
        .is_err()
    );
    assert!(!target.exists());
    remove_pending(root);
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
        TEST_PLAN_SHA256,
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
        TEST_PLAN_SHA256,
    )
    .expect_err("concurrent canonical claimant must fail closed");
    assert!(matches!(error, Error::AmbiguousEffect(_)), "{error:?}");
    assert_eq!(
        fs::read(&target).unwrap(),
        b"reviewed source\nmanaged block"
    );
    let transaction = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.extension().is_some_and(|extension| extension == "txn"))
        .expect("preserved append transaction");
    assert_eq!(
        fs::read(transaction.join("original")).unwrap(),
        b"reviewed source"
    );
    assert_eq!(
        fs::read(transaction.join("reviewed")).unwrap(),
        b"reviewed source\nmanaged block"
    );
    assert_eq!(
        fs::read(transaction.join("exchange")).unwrap(),
        b"concurrent target claimant"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        let target_metadata = fs::metadata(&target).unwrap();
        let original_metadata = fs::metadata(transaction.join("original")).unwrap();
        let reviewed_metadata = fs::metadata(transaction.join("reviewed")).unwrap();
        let exchange_metadata = fs::metadata(transaction.join("exchange")).unwrap();
        assert_eq!(original_metadata.nlink(), 1);
        assert_eq!(reviewed_metadata.nlink(), 1);
        assert_eq!(exchange_metadata.nlink(), 1);
        assert_ne!(original_metadata.ino(), reviewed_metadata.ino());
        assert_ne!(original_metadata.ino(), exchange_metadata.ino());
        assert_ne!(reviewed_metadata.ino(), exchange_metadata.ino());
        assert_eq!(target_metadata.mode(), original_metadata.mode());
        assert_eq!(target_metadata.mode(), reviewed_metadata.mode());
        assert_eq!(target_metadata.mode(), exchange_metadata.mode());
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn post_publication_cleanup_failures_are_ambiguous_effects() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    for point in [
        "remove project instruction completion receipt",
        "sync project instruction root after receipt cleanup",
    ] {
        fs::write(&target, b"before").unwrap();
        inject_storage_failure(point);
        let error = publish_instruction(
            &target,
            b"after",
            AgentIntegrationOperation::Append,
            b"before",
            TEST_PLAN_SHA256,
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
