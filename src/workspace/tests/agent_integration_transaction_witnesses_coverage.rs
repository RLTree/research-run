use std::fs;

use crate::workspace::tests::temporary;
use crate::workspace::{inject_storage_failure, publication::pending_path};

use super::*;

#[test]
fn witness_staging_rejects_non_files_and_mode_persistence_failure() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    let transaction = pending_path(&target).with_extension("txn");
    fs::create_dir(&target).unwrap();
    fs::create_dir(&transaction).unwrap();
    let paths = WitnessPaths::new(&transaction);
    assert!(stage_witnesses(&target, &transaction, &paths, b"o", b"p").is_err());

    fs::remove_dir(&target).unwrap();
    fs::write(&target, b"o").unwrap();
    inject_storage_failure("preserve project instruction Unix mode");
    assert!(stage_witnesses(&target, &transaction, &paths, b"o", b"p").is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn witness_verification_rejects_a_missing_canonical_target() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    let transaction = root.join("transaction");
    fs::write(&target, b"o").unwrap();
    fs::create_dir(&transaction).unwrap();
    let paths = WitnessPaths::new(&transaction);
    stage_witnesses(&target, &transaction, &paths, b"o", b"p").unwrap();
    fs::remove_file(&target).unwrap();

    assert!(verify_witnesses(&target, &paths, b"o", b"p", b"p").is_err());
    fs::remove_dir_all(root).unwrap();
}
