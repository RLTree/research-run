use super::*;
use crate::domain::InventorySnapshot;

pub(super) fn publish_inventory(workspace: &Workspace) {
    let inventory = InventorySnapshot {
        schema_version: 1,
        kind: "inventory".to_owned(),
        id: "inventory-one".to_owned(),
        project_name: "Relations".to_owned(),
        project_id: "relations".to_owned(),
        observed_at: "2026-07-18T20:00:00Z".to_owned(),
        previous_snapshot_id: None,
        policy: None,
        entries: Vec::new(),
        changes: Vec::new(),
    };
    workspace
        .publish_record("inventories", &inventory)
        .expect("inventory");
}
