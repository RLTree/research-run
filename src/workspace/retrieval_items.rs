use std::collections::BTreeSet;

use crate::domain::{BoundaryEntry, InventoryEntry, RelationshipKind};

use super::retrieval_canonical::add_canonical;
use super::retrieval_types::ProjectionItem;
use super::{Snapshot, Workspace};

pub(super) fn projection_items(
    workspace: &Workspace,
    snapshot: &Snapshot,
) -> crate::Result<Vec<ProjectionItem>> {
    let (stale, invalidated) = history_flags(snapshot);
    let mut items = Vec::new();
    add_canonical(workspace, snapshot, &mut items)?;
    for record in &snapshot.knowledge {
        items.push(item(ItemSpec {
            kind: "knowledge",
            id: &record.id,
            subtype: &enum_name(&record.record_type),
            title: &record.title,
            summary: &record.body,
            occurred_at: Some(&record.occurred_at),
            state: Some(&enum_name(&record.state)),
            authority_path: &format!(".research-run/knowledge/{}.json", record.id),
            stale: stale.contains(record.id.as_str()),
            invalidated: invalidated.contains(record.id.as_str()),
        }));
    }
    add_inventories(snapshot, &mut items);
    items.sort_by(|left, right| (&left.kind, &left.id).cmp(&(&right.kind, &right.id)));
    Ok(items)
}

fn add_inventories(snapshot: &Snapshot, items: &mut Vec<ProjectionItem>) {
    let latest = snapshot
        .inventories
        .iter()
        .max_by(|left, right| (&left.observed_at, &left.id).cmp(&(&right.observed_at, &right.id)))
        .map(|inventory| inventory.id.as_str());
    for inventory in &snapshot.inventories {
        let stale = Some(inventory.id.as_str()) != latest;
        let authority = format!(".research-run/inventories/{}.json", inventory.id);
        let indexed = inventory
            .entries
            .iter()
            .filter(|entry| matches!(entry, InventoryEntry::Indexed(_)))
            .count();
        let boundaries = inventory.entries.len() - indexed;
        items.push(item(ItemSpec {
            kind: "inventory",
            id: &inventory.id,
            subtype: "inventory",
            title: &format!("Inventory {}", inventory.id),
            summary: &format!("{indexed} indexed materials and {boundaries} explicit boundaries"),
            occurred_at: Some(&inventory.observed_at),
            state: None,
            authority_path: &authority,
            stale,
            invalidated: false,
        }));
        for entry in &inventory.entries {
            add_inventory_entry(entry, inventory, &authority, stale, items);
        }
    }
}

fn add_inventory_entry(
    entry: &InventoryEntry,
    inventory: &crate::domain::InventorySnapshot,
    authority: &str,
    stale: bool,
    items: &mut Vec<ProjectionItem>,
) {
    let (id, subtype, summary) = match entry {
        InventoryEntry::Indexed(material) => (
            material.path.as_str(),
            enum_name(&material.class),
            format!("{} bytes SHA-256 {}", material.bytes, material.sha256),
        ),
        InventoryEntry::Boundary(BoundaryEntry::ReferenceOnly { path, .. }) => (
            path.as_str(),
            "reference-only".to_owned(),
            "reference-only root metadata; descendant content was not inspected, hashed, validated, or scientifically verified".to_owned(),
        ),
        InventoryEntry::Boundary(BoundaryEntry::ChildWorkspace {
            path,
            workspace_observation,
            ..
        }) => (
            path.as_str(),
            "child-workspace".to_owned(),
            format!(
                "child workspace {} identity observed; child material was not inspected, hashed, validated, or scientifically verified",
                workspace_observation.workspace_id
            ),
        ),
    };
    items.push(item(ItemSpec {
        kind: "material",
        id,
        subtype: &subtype,
        title: id,
        summary: &summary,
        occurred_at: Some(&inventory.observed_at),
        state: None,
        authority_path: authority,
        stale,
        invalidated: false,
    }));
}

fn history_flags(snapshot: &Snapshot) -> (BTreeSet<&str>, BTreeSet<&str>) {
    let mut stale = BTreeSet::new();
    let mut invalidated = BTreeSet::new();
    for relationship in &snapshot.relationships {
        if matches!(
            relationship.relationship,
            RelationshipKind::Supersedes | RelationshipKind::Revises
        ) {
            stale.insert(relationship.to.id.as_str());
        }
        if relationship.relationship == RelationshipKind::Invalidates {
            invalidated.insert(relationship.to.id.as_str());
        }
    }
    (stale, invalidated)
}

pub(super) struct ItemSpec<'a> {
    pub(super) kind: &'a str,
    pub(super) id: &'a str,
    pub(super) subtype: &'a str,
    pub(super) title: &'a str,
    pub(super) summary: &'a str,
    pub(super) occurred_at: Option<&'a str>,
    pub(super) state: Option<&'a str>,
    pub(super) authority_path: &'a str,
    pub(super) stale: bool,
    pub(super) invalidated: bool,
}

pub(super) fn item(spec: ItemSpec<'_>) -> ProjectionItem {
    ProjectionItem {
        kind: spec.kind.to_owned(),
        id: spec.id.to_owned(),
        subtype: spec.subtype.to_owned(),
        title: spec.title.to_owned(),
        summary: truncate(spec.summary, 512),
        occurred_at: spec.occurred_at.map(str::to_owned),
        state: spec.state.map(str::to_owned),
        authority_path: spec.authority_path.to_owned(),
        matched_by: Vec::new(),
        stale: spec.stale,
        invalidated: spec.invalidated,
    }
}

fn truncate(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

pub(super) fn enum_name(value: &impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .expect("domain enum serialization is infallible")
        .as_str()
        .expect("domain enums serialize as strings")
        .to_owned()
}
