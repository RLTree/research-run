use super::*;
use crate::domain::{InventoryPlan, ProjectManifest};
use crate::workspace::inventory_bootstrap::initialize_inventory_bootstrap;

#[test]
fn invalid_new_workspace_plan_leaves_no_bootstrap_authority() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("material");
    let mut invalid = Workspace::plan_retrofit(
        &root,
        "Invalid bootstrap",
        "inventory-one",
        "2026-07-20T00:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    invalid.without_review_authority = false;

    let error = Workspace::apply_inventory_plan(&root, invalid.clone())
        .expect_err("missing authority must fail");
    assert!(error.to_string().contains("requires an authority"));
    assert!(!root.join(".research-run").exists());
    assert!(initialize_inventory_bootstrap(&root, &invalid).is_err());

    let corrected = InventoryPlan {
        without_review_authority: true,
        ..invalid
    };
    assert!(Workspace::apply_inventory_plan(&root, corrected).expect("corrected plan"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn precreated_bootstrap_scaffold_cannot_bypass_authority_validation() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("material");
    let mut invalid = Workspace::plan_retrofit(
        &root,
        "Precreated scaffold",
        "inventory-one",
        "2026-07-20T00:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    invalid.without_review_authority = false;
    let state = root.join(".research-run");
    fs::create_dir(&state).expect("partial state");
    fs::write(state.join("write.lock"), b"").expect("precreated lock");

    assert!(Workspace::apply_inventory_plan(&root, invalid.clone()).is_err());
    assert_eq!(fs::read_dir(&state).unwrap().count(), 1);
    assert!(!state.join("inventory-bootstrap.json").exists());

    let corrected = InventoryPlan {
        without_review_authority: true,
        ..invalid
    };
    assert!(Workspace::apply_inventory_plan(&root, corrected).expect("corrected plan"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn decoy_manifests_cannot_create_bootstrap_effects() {
    for (name, manifest) in [
        ("malformed", b"{".to_vec()),
        (
            "partial-valid",
            serde_json::to_vec(&ProjectManifest::new("Decoy").expect("manifest"))
                .expect("serialize manifest"),
        ),
    ] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("material");
        let mut invalid = Workspace::plan_retrofit(
            &root,
            "Decoy manifest",
            "inventory-one",
            "2026-07-20T00:00:00Z",
            explicit_unanchored_retrofit(),
        )
        .expect("plan");
        invalid.without_review_authority = false;
        let state = root.join(".research-run");
        fs::create_dir(&state).expect("partial state");
        fs::write(state.join("manifest.json"), manifest).expect(name);

        assert!(Workspace::apply_inventory_plan(&root, invalid).is_err());
        assert!(!state.join("write.lock").exists());
        assert!(!state.join("inventory-bootstrap.json").exists());
        assert_eq!(fs::read_dir(&state).unwrap().count(), 1);
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[cfg(unix)]
#[test]
fn symlinked_manifest_cannot_create_bootstrap_effects() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("material");
    let mut invalid = Workspace::plan_retrofit(
        &root,
        "Symlinked manifest",
        "inventory-one",
        "2026-07-20T00:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    invalid.without_review_authority = false;
    let state = root.join(".research-run");
    fs::create_dir(&state).expect("partial state");
    fs::write(state.join("decoy.json"), b"{}").expect("decoy");
    std::os::unix::fs::symlink("decoy.json", state.join("manifest.json")).expect("symlink");

    assert!(Workspace::apply_inventory_plan(&root, invalid).is_err());
    assert!(!state.join("write.lock").exists());
    assert!(!state.join("inventory-bootstrap.json").exists());
    assert_eq!(fs::read_dir(&state).unwrap().count(), 2);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn manifest_inspection_races_fail_before_bootstrap_effects() {
    for fault in [
        "inspect inventory workspace manifest",
        "inventory manifest disappeared during validation",
    ] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("material");
        let plan = Workspace::plan_retrofit(
            &root,
            "Manifest race",
            "inventory-one",
            "2026-07-20T00:00:00Z",
            explicit_unanchored_retrofit(),
        )
        .expect("plan");
        let state = root.join(".research-run");
        fs::create_dir(&state).expect("partial state");
        fs::write(
            state.join("manifest.json"),
            serde_json::to_vec(&ProjectManifest::new("Manifest race").expect("manifest"))
                .expect("serialize manifest"),
        )
        .expect("manifest");
        inject_storage_failure(fault);

        assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
        assert!(!state.join("write.lock").exists());
        assert!(!state.join("inventory-bootstrap.json").exists());
        assert_eq!(fs::read_dir(&state).unwrap().count(), 1);
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[cfg(unix)]
#[test]
fn review_binding_rejects_fifo_artifacts_before_opening() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "FIFO binding").expect("initialize");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    let status = std::process::Command::new("mkfifo")
        .arg(root.join("artifact.txt"))
        .status()
        .expect("mkfifo");
    assert!(status.success());
    workspace
        .add_evidence(&evidence("evidence-one", "claim-one", None))
        .expect("evidence");

    let error = workspace
        .review_subject_binding("claim-one")
        .expect_err("FIFO artifact must fail without blocking");
    assert!(error.to_string().contains("regular file"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn review_binding_rejects_a_fifo_swap_during_open() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "FIFO swap").expect("initialize");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    fs::write(root.join("artifact.txt"), b"regular before open").expect("artifact");
    workspace
        .add_evidence(&evidence("evidence-one", "claim-one", None))
        .expect("evidence");
    inject_storage_failure("material open fifo race");

    let error = workspace
        .review_subject_binding("claim-one")
        .expect_err("FIFO swap must fail without blocking");
    assert!(error.to_string().contains("regular file"));
    fs::remove_dir_all(root).expect("remove fixture");
}
