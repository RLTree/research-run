use super::*;
use crate::domain::{InventorySnapshot, MaterialClass, MaterialEntry};

pub(super) fn add_inventories(workspace: &Workspace) {
    for (id, at, previous) in [
        ("inventory-old", "2026-07-18T20:00:00Z", None),
        (
            "inventory-new",
            "2026-07-18T20:01:00Z",
            Some("inventory-old".to_owned()),
        ),
    ] {
        let inventory = InventorySnapshot {
            schema_version: 1,
            kind: "inventory".to_owned(),
            id: id.to_owned(),
            project_name: "Retrieval unit".to_owned(),
            project_id: "retrieval-unit".to_owned(),
            observed_at: at.to_owned(),
            previous_snapshot_id: previous,
            policy: None,
            entries: vec![
                MaterialEntry {
                    path: "note.md".to_owned(),
                    class: MaterialClass::Note,
                    bytes: 1,
                    sha256: "a".repeat(64),
                }
                .into(),
            ],
            changes: Vec::new(),
        };
        workspace
            .publish_record("inventories", &inventory)
            .expect("inventory");
    }
}
