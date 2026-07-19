use super::*;

#[test]
fn recovery_batch_rejects_manifest_parse_validation_and_directory_failures() {
    for (label, bytes, fault) in [
        (
            "pending-manifest-budget",
            valid_manifest_bytes(),
            Some("snapshot byte budget#2"),
        ),
        ("pending-manifest-malformed", b"{".to_vec(), None),
        (
            "pending-manifest-invalid",
            serde_json::to_vec(&json!({
                "schema_version": 1, "kind": "project-manifest", "project_id": "bad id",
                "name": "Coverage", "declared_roots": ["."]
            }))
            .expect("manifest"),
            None,
        ),
    ] {
        let root = initialize(label);
        let state = root.0.join(".research-run");
        fs::remove_file(state.join("manifest.json")).expect("remove manifest");
        fs::write(state.join(".manifest.json.11.1.tmp"), bytes).expect("pending manifest");
        assert!(!run(&root.0, &["recover", "--json"], fault).status.success());
    }

    reject_symlinked_recovery_directory();
}

#[test]
fn recovery_rejects_a_record_published_under_another_identity() {
    let root = initialize("recovery-mismatched-source-identity");
    let pending_path = pending(
        &root.0,
        "sources",
        "source-one",
        pending_source_value("source-two"),
    );

    assert!(!run(&root.0, &["recover", "--json"], None).status.success());
    assert!(pending_path.exists());
    assert!(
        !root
            .0
            .join(".research-run/sources/source-one.json")
            .exists()
    );

    let invalid = initialize("recovery-invalid-source-record");
    let mut source = pending_source_value("source-one");
    source["citation"] = json!("");
    let invalid_pending = pending(&invalid.0, "sources", "source-one", source);
    assert!(
        !run(&invalid.0, &["recover", "--json"], None)
            .status
            .success()
    );
    assert!(invalid_pending.exists());
}

#[test]
fn compact_pending_manifests_compare_as_canonical_state() {
    let identical = initialize("compact-identical-manifest");
    let state = identical.0.join(".research-run");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(state.join("manifest.json")).expect("canonical manifest"))
            .expect("manifest JSON");
    let identical_pending = state.join(".manifest.json.11.1.tmp");
    fs::write(
        &identical_pending,
        serde_json::to_vec(&manifest).expect("compact manifest"),
    )
    .expect("pending manifest");
    assert!(
        run(&identical.0, &["recover", "--json"], None)
            .status
            .success()
    );
    assert!(!identical_pending.exists());

    let conflicting = initialize("compact-conflicting-manifest");
    let state = conflicting.0.join(".research-run");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(state.join("manifest.json")).expect("canonical manifest"))
            .expect("manifest JSON");
    manifest["name"] = json!("Different name");
    let conflicting_pending = state.join(".manifest.json.11.1.tmp");
    fs::write(
        &conflicting_pending,
        serde_json::to_vec(&manifest).expect("compact conflicting manifest"),
    )
    .expect("pending manifest");
    assert!(
        !run(&conflicting.0, &["recover", "--json"], None)
            .status
            .success()
    );
    assert!(conflicting_pending.exists());
}

fn valid_manifest_bytes() -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version": 1, "kind": "project-manifest", "project_id": "coverage",
        "name": "Coverage", "declared_roots": ["."]
    }))
    .expect("manifest")
}

#[cfg(unix)]
fn reject_symlinked_recovery_directory() {
    use std::os::unix::fs::symlink;

    let root = initialize("recovery-symlinked-directory");
    let outside = TempDir::new("recovery-symlinked-directory-outside");
    fs::remove_dir(root.0.join(".research-run/sources")).expect("remove sources");
    symlink(&outside.0, root.0.join(".research-run/sources")).expect("symlink sources");
    assert!(!run(&root.0, &["recover", "--json"], None).status.success());
}

#[cfg(not(unix))]
fn reject_symlinked_recovery_directory() {}
