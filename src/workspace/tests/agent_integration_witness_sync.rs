const FIFO_CHILD_ROOT: &str = "RESEARCH_RUN_WITNESS_SYNC_FIFO_CHILD_ROOT";
const FIFO_CHILD_COMPLETE: &str = ".witness-sync-fifo-child-complete";

#[cfg(unix)]
#[test]
fn witness_sync_rejects_a_replaced_transaction_directory() {
    use std::fs;

    use super::agent_integration_transaction::directories::open_exchange_handles;
    use super::agent_integration_transaction::witnesses::{WitnessPaths, sync_witnesses};

    let root = temporary();
    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.995.1.txn");
    let retained = root.join("retained-transaction");
    fs::write(&target, b"original").unwrap();
    fs::create_dir(&transaction).unwrap();
    for name in ["version", "original", "reviewed", "exchange"] {
        fs::write(transaction.join(name), b"transaction evidence").unwrap();
    }
    let handles = open_exchange_handles(&target, &transaction).unwrap();
    let paths = WitnessPaths::new(&transaction);
    fs::rename(&transaction, &retained).unwrap();
    fs::create_dir(&transaction).unwrap();
    for name in ["version", "original", "reviewed", "exchange"] {
        fs::write(transaction.join(name), b"replacement sentinel").unwrap();
    }

    assert!(sync_witnesses(&target, &transaction, &handles, &paths).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"original");
    for name in ["version", "original", "reviewed", "exchange"] {
        assert_eq!(
            fs::read(retained.join(name)).unwrap(),
            b"transaction evidence"
        );
        assert_eq!(
            fs::read(transaction.join(name)).unwrap(),
            b"replacement sentinel"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn witness_sync_cannot_follow_a_replaced_transaction_symlink_to_fifo() {
    if let Some(root) = std::env::var_os(FIFO_CHILD_ROOT) {
        exercise_fifo_witness_sync(std::path::Path::new(&root));
        return;
    }
    let root = temporary();
    let module = module_path!()
        .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
        .unwrap_or(module_path!());
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .arg(format!(
            "{}::{}",
            module,
            stringify!(witness_sync_cannot_follow_a_replaced_transaction_symlink_to_fifo)
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
    let status = status.expect("witness sync followed a FIFO outside the held transaction");
    assert!(status.success(), "witness sync child failed: {status}");
    assert!(
        root.join(FIFO_CHILD_COMPLETE).is_file(),
        "witness sync child did not execute the exact regression"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
fn exercise_fifo_witness_sync(root: &std::path::Path) {
    use std::fs;
    use std::os::unix::fs::{FileTypeExt, symlink};

    use super::agent_integration_transaction::directories::open_exchange_handles;
    use super::agent_integration_transaction::witnesses::{WitnessPaths, sync_witnesses};

    let target = root.join("AGENTS.md");
    let transaction = root.join(".AGENTS.md.996.1.txn");
    let retained = root.join("retained-transaction");
    let outside = root.join("outside");
    fs::write(&target, b"original").unwrap();
    fs::create_dir(&transaction).unwrap();
    fs::create_dir(&outside).unwrap();
    for name in ["version", "original", "reviewed", "exchange"] {
        fs::write(transaction.join(name), b"transaction evidence").unwrap();
    }
    let handles = open_exchange_handles(&target, &transaction).unwrap();
    let paths = WitnessPaths::new(&transaction);
    fs::rename(&transaction, &retained).unwrap();
    assert!(
        std::process::Command::new("mkfifo")
            .arg(outside.join("version"))
            .status()
            .unwrap()
            .success()
    );
    for name in ["original", "reviewed", "exchange"] {
        fs::write(outside.join(name), b"outside sentinel").unwrap();
    }
    symlink(&outside, &transaction).unwrap();

    assert!(sync_witnesses(&target, &transaction, &handles, &paths).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"original");
    assert!(
        fs::symlink_metadata(outside.join("version"))
            .unwrap()
            .file_type()
            .is_fifo()
    );
    for name in ["version", "original", "reviewed", "exchange"] {
        assert_eq!(
            fs::read(retained.join(name)).unwrap(),
            b"transaction evidence"
        );
    }
    fs::write(root.join(FIFO_CHILD_COMPLETE), b"complete").unwrap();
}
