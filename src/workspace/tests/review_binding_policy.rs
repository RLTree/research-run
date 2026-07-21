use super::*;
use crate::domain::{InventoryPolicy, InventorySnapshot};

#[test]
fn review_binding_uses_the_current_inventory_file_limit() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Current inventory limit").expect("workspace");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    File::create(root.join("artifact.txt"))
        .expect("artifact")
        .set_len(64 * 1_048_576 + 1)
        .expect("sparse artifact");
    workspace
        .add_evidence(&evidence("evidence-one", "claim-one", None))
        .expect("evidence");
    let manifest = workspace.read_manifest().expect("manifest");
    workspace
        .publish_record(
            "inventories",
            &InventorySnapshot {
                schema_version: 1,
                kind: "inventory".to_owned(),
                id: "inventory-policy".to_owned(),
                project_name: manifest.name,
                project_id: manifest.project_id,
                observed_at: "2026-07-21T03:00:00Z".to_owned(),
                previous_snapshot_id: None,
                policy: Some(InventoryPolicy::default()),
                entries: Vec::new(),
                changes: Vec::new(),
            },
        )
        .expect("inventory");
    assert!(workspace.review_subject_binding("claim-one").is_ok());
    fs::remove_dir_all(root).expect("remove fixture");
}
