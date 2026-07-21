use super::*;
use crate::domain::{InventoryBoundary, InventoryLimits, InventoryPolicy, ProjectManifest};

#[test]
fn child_workspace_identity_cannot_collide_with_its_parent() {
    let root = temporary();
    let child = root.join("child");
    let child_workspace = Workspace::initialize(&child, "Child").expect("child workspace");
    let mut child_manifest = child_workspace.read_manifest().expect("child manifest");
    child_manifest.project_id = ProjectManifest::new("Parent")
        .expect("parent identity")
        .project_id;
    fs::write(
        child.join(".research-run/manifest.json"),
        serde_json::to_vec_pretty(&child_manifest).expect("manifest bytes"),
    )
    .expect("colliding child fixture");
    let policy = InventoryPolicy {
        schema_version: 1,
        kind: "inventory-policy".to_owned(),
        limits: InventoryLimits::default(),
        boundaries: vec![InventoryBoundary::ChildWorkspace {
            path: "child".to_owned(),
            rationale: "Independent project authority".to_owned(),
        }],
    };
    assert!(
        Workspace::plan_retrofit_with_policy(
            &root,
            "Parent",
            "inventory-one",
            "2026-07-20T04:30:00Z",
            explicit_unanchored_retrofit(),
            Some(policy),
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}
