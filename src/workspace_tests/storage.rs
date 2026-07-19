use super::*;
use std::sync::{Arc, Barrier};

#[test]
fn bounded_storage_and_directory_shapes_fail_closed() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Storage boundaries").expect("initialize");
    let storage_root = workspace.root.clone();

    let mut budget = ReadBudget::default();
    budget
        .consume(&storage_root, MAX_SNAPSHOT_BYTES)
        .expect("exact budget");
    assert!(budget.consume(&storage_root, 1).is_err());

    let directory = storage_root.join("not-a-record.json");
    fs::create_dir(&directory).expect("directory fixture");
    assert!(read_bounded(&directory).is_err());

    let oversized = storage_root.join("oversized.json");
    File::create(&oversized)
        .expect("large fixture")
        .set_len(MAX_RECORD_BYTES + 1)
        .expect("size fixture");
    assert!(read_bounded(&oversized).is_err());

    let malformed = storage_root.join("malformed.json");
    fs::write(&malformed, b"{").expect("malformed fixture");
    let mut parse_budget = ReadBudget::default();
    assert!(read_json_with_budget::<ProjectManifest>(&malformed, &mut parse_budget).is_err());

    let unexpected = workspace.state.join("sources/unexpected.txt");
    fs::write(&unexpected, b"unexpected").expect("unexpected fixture");
    assert!(workspace.load_snapshot().is_err());
    fs::remove_file(&unexpected).expect("remove unexpected fixture");

    let mismatched = workspace.state.join("sources/wrong.json");
    fs::write(
        &mismatched,
        serde_json::to_vec_pretty(&source("source-one")).expect("json"),
    )
    .expect("mismatch fixture");
    assert!(workspace.load_snapshot().is_err());
    fs::remove_file(&mismatched).expect("remove mismatch fixture");

    let pending = workspace.state.join("claims/.claim-one.json.1.1.tmp");
    fs::write(&pending, b"{}").expect("pending fixture");
    assert!(workspace.load_snapshot().is_err());
    assert!(workspace.load_snapshot_allow_pending(true).is_ok());
    assert!(ensure_no_pending_effect(&workspace.state.join("claims/claim-one.json")).is_err());
    fs::remove_file(&pending).expect("remove pending fixture");

    assert!(
        workspace
            .publish_bytes(
                &storage_root.join("too-large.json"),
                &vec![0; MAX_RECORD_BYTES as usize + 1]
            )
            .is_err()
    );
    let target = storage_root.join("bytes.json");
    assert!(workspace.publish_bytes(&target, b"one").expect("publish"));
    assert!(!workspace.publish_bytes(&target, b"one").expect("retry"));
    assert!(workspace.publish_bytes(&target, b"two").is_err());

    let blocking_file = storage_root.join("blocking-file");
    fs::write(&blocking_file, b"file").expect("blocking fixture");
    assert!(create_directory_chain(&blocking_file.join("child")).is_err());

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn pending_cleanup_reports_removal_and_directory_sync_failures() {
    let root = temporary();
    let cleanup_path = root.join("cleanup.tmp");
    fs::write(&cleanup_path, b"pending").expect("cleanup fixture");
    PendingCleanup::new(cleanup_path.clone())
        .remove()
        .expect("cleanup pending file");
    assert!(!cleanup_path.exists());
    fs::write(&cleanup_path, b"pending").expect("preserve fixture");
    let _cleanup = PendingCleanup::new(cleanup_path.clone());
    assert!(cleanup_path.exists());

    let cleanup_directory = root.join("cleanup-directory");
    fs::create_dir(&cleanup_directory).expect("cleanup directory fixture");
    let mut failed_cleanup = PendingCleanup::new(cleanup_directory.clone());
    assert!(failed_cleanup.remove().is_err());
    assert!(cleanup_directory.is_dir());

    let unsynced_cleanup = root.join("unsynced-cleanup.tmp");
    fs::write(&unsynced_cleanup, b"pending").expect("unsynced cleanup fixture");
    let mut failed_sync = PendingCleanup::new(unsynced_cleanup.clone());
    inject_storage_failure("sync record directory");
    assert!(failed_sync.remove().is_err());
    assert!(!unsynced_cleanup.exists());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn write_lock_contention_fails_before_a_second_transaction() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Lock contention").expect("initialize");
    let first = WorkspaceWriteLock::acquire(&workspace.state).expect("first lock");
    let release = Arc::new(Barrier::new(2));
    let contender_release = Arc::clone(&release);
    let state = workspace.state.clone();
    let contender = std::thread::spawn(move || {
        contender_release.wait();
        WorkspaceWriteLock::acquire(&state).is_err()
    });
    release.wait();
    assert!(contender.join().expect("contender result"));
    drop(first);
    assert!(WorkspaceWriteLock::acquire(&workspace.state).is_ok());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn injected_storage_faults_prove_ambiguous_and_racing_effect_contracts() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Injected storage faults").expect("initialize");
    let manifest = workspace.state.join("manifest.json");

    let mut budget = ReadBudget::default();
    inject_storage_failure("snapshot byte budget");
    assert!(budget.consume(&manifest, 1).is_err());

    inject_storage_failure("lock identity");
    assert!(WorkspaceWriteLock::acquire(&workspace.state).is_err());

    inject_storage_failure("inspect record");
    assert!(read_bounded(&manifest).is_err());
    inject_storage_failure("record grew");
    assert!(read_bounded(&manifest).is_err());
    inject_storage_failure("record identity");
    assert!(read_bounded(&manifest).is_err());

    let concurrent_directory = workspace.root.join("concurrent/directory");
    inject_storage_failure("directory already exists");
    create_directory_chain(&concurrent_directory).expect("concurrent directory");
    assert!(concurrent_directory.is_dir());

    inject_storage_failure("inspect workspace path");
    assert!(reject_symlink_chain(&manifest).is_err());

    let identical = workspace.root.join("identical-race.json");
    inject_storage_failure("publish identical race");
    assert!(
        !workspace
            .publish_bytes(&identical, b"same")
            .expect("identical race")
    );

    let conflicting = workspace.root.join("conflicting-race.json");
    inject_storage_failure("publish conflicting race");
    assert!(workspace.publish_bytes(&conflicting, b"expected").is_err());

    let sync_target = workspace.root.join("sync-ambiguous.json");
    inject_storage_failure("sync record directory");
    assert!(workspace.publish_bytes(&sync_target, b"published").is_err());
    assert!(sync_target.exists());

    let cleanup_target = workspace.root.join("cleanup-ambiguous.json");
    inject_storage_failure("remove pending record");
    assert!(
        workspace
            .publish_bytes(&cleanup_target, b"published")
            .is_err()
    );
    assert!(cleanup_target.exists());

    let publish_error = workspace.root.join("publish-error.json");
    inject_storage_failure("publish canonical record");
    assert!(workspace.publish_bytes(&publish_error, b"pending").is_err());
    assert!(!publish_error.exists());

    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn symlinked_state_directory_is_rejected() {
    use std::os::unix::fs::symlink;

    let root = temporary();
    let outside = temporary();
    symlink(&outside, root.join(".research-run")).expect("create symlink fixture");
    let error = Workspace::initialize(&root, "Unsafe").expect_err("must reject symlink");
    assert!(error.to_string().contains("symlink"));
    fs::remove_dir_all(root).expect("remove fixture");
    fs::remove_dir_all(outside).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn symlinked_paths_are_rejected_at_each_storage_boundary() {
    use std::os::unix::fs::symlink;

    let root = temporary();
    let outside = temporary();
    let link = root.join("link");
    symlink(&outside, &link).expect("symlink fixture");
    assert!(reject_symlink_chain(&link.join("record.json")).is_err());
    assert!(Workspace::discover(&link).is_err());
    assert!(Workspace::for_recovery(&link).is_err());
    assert!(Workspace::initialize(&link.join("project"), "Unsafe").is_err());
    let canonical_child = root.join("canonical-child");
    symlink(&outside, &canonical_child).expect("canonical child symlink");
    assert!(create_directory_chain(&canonical_child).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
    fs::remove_dir_all(outside).expect("remove fixture");
}
