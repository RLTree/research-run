const FIFO_CHILD_ROOT: &str = "RESEARCH_RUN_STAGING_FIFO_CHILD_ROOT";
const FIFO_CHILD_COMPLETE: &str = ".staging-fifo-child-complete";

#[cfg(unix)]
#[test]
fn staging_cleanup_cannot_follow_a_replaced_transaction_directory() {
    use std::fs;
    use std::os::unix::fs::symlink;

    use crate::Error;

    use super::super::agent_integration_transaction::cleanup_staging_transaction;
    use super::temporary;

    let root = temporary();
    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.993.1.txn");
    let retained = root.join("retained-transaction");
    let outside = temporary();
    fs::write(&target, b"original").unwrap();
    fs::create_dir(&transaction).unwrap();
    for name in ["version", "exchange", "original", "reviewed"] {
        fs::write(transaction.join(name), b"transaction evidence").unwrap();
        fs::write(outside.join(name), b"outside sentinel").unwrap();
    }
    let handles = super::super::agent_integration_transaction::directories::open_exchange_handles(
        &target,
        &transaction,
    )
    .unwrap();
    fs::rename(&transaction, &retained).unwrap();
    symlink(&outside, &transaction).unwrap();

    let error = cleanup_staging_transaction(&target, &transaction, &handles)
        .expect_err("replaced transaction cleanup must fail closed");

    assert!(matches!(error, Error::AmbiguousEffect(_)), "{error:?}");
    for name in ["version", "exchange", "original", "reviewed"] {
        assert_eq!(fs::read(outside.join(name)).unwrap(), b"outside sentinel");
        assert_eq!(
            fs::read(retained.join(name)).unwrap(),
            b"transaction evidence"
        );
    }
    fs::remove_file(&transaction).unwrap();
    fs::create_dir(&transaction).unwrap();
    for name in ["version", "exchange", "original", "reviewed"] {
        fs::write(transaction.join(name), b"replacement sentinel").unwrap();
    }
    let error = cleanup_staging_transaction(&target, &transaction, &handles)
        .expect_err("replacement directory cleanup must fail closed");
    assert!(matches!(error, Error::AmbiguousEffect(_)), "{error:?}");
    for name in ["version", "exchange", "original", "reviewed"] {
        assert_eq!(
            fs::read(transaction.join(name)).unwrap(),
            b"replacement sentinel"
        );
        assert_eq!(
            fs::read(retained.join(name)).unwrap(),
            b"transaction evidence"
        );
    }
    fs::remove_dir_all(transaction).unwrap();
    fs::remove_dir_all(retained).unwrap();
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[cfg(unix)]
#[test]
fn staging_cleanup_rejects_a_fifo_before_any_deletion_without_blocking() {
    if let Some(root) = std::env::var_os(FIFO_CHILD_ROOT) {
        exercise_fifo_staging_cleanup(std::path::Path::new(&root));
        return;
    }
    let root = super::temporary();
    let module = module_path!()
        .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
        .unwrap_or(module_path!());
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg(format!(
            "{}::{}",
            module,
            stringify!(staging_cleanup_rejects_a_fifo_before_any_deletion_without_blocking)
        ))
        .arg("--exact")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(FIFO_CHILD_ROOT, &root)
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
    let status = status.expect("staging cleanup blocked on a FIFO for over 5 seconds");
    assert!(status.success(), "FIFO staging child failed: {status}");
    assert!(
        root.join(FIFO_CHILD_COMPLETE).is_file(),
        "FIFO staging child did not execute the exact regression"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[cfg(unix)]
fn exercise_fifo_staging_cleanup(root: &std::path::Path) {
    use std::os::unix::fs::FileTypeExt;

    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.994.1.txn");
    std::fs::write(&target, b"original").unwrap();
    std::fs::create_dir(&transaction).unwrap();
    for name in ["version", "original", "reviewed"] {
        std::fs::write(transaction.join(name), b"transaction evidence").unwrap();
    }
    let fifo = transaction.join("exchange");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let handles = super::super::agent_integration_transaction::directories::open_exchange_handles(
        &target,
        &transaction,
    )
    .unwrap();

    let result = super::super::agent_integration_transaction::cleanup_staging_transaction(
        &target,
        &transaction,
        &handles,
    );

    assert!(matches!(result, Err(crate::Error::AmbiguousEffect(_))));
    for name in ["version", "original", "reviewed"] {
        assert!(transaction.join(name).is_file(), "{name} was deleted");
    }
    assert!(
        std::fs::symlink_metadata(fifo)
            .unwrap()
            .file_type()
            .is_fifo()
    );
    std::fs::write(root.join(FIFO_CHILD_COMPLETE), b"complete").unwrap();
}
