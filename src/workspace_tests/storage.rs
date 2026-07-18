use super::*;

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

    let cleanup_path = storage_root.join("cleanup.tmp");
    fs::write(&cleanup_path, b"pending").expect("cleanup fixture");
    drop(PendingCleanup::new(cleanup_path.clone()));
    assert!(!cleanup_path.exists());
    fs::write(&cleanup_path, b"pending").expect("preserve fixture");
    let mut cleanup = PendingCleanup::new(cleanup_path.clone());
    cleanup.preserve();
    drop(cleanup);
    assert!(cleanup_path.exists());

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
    fs::remove_dir_all(root).expect("remove fixture");
    fs::remove_dir_all(outside).expect("remove fixture");
}
