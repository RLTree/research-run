use std::collections::BTreeSet;

use crate::domain::RelationshipKind;

use super::Snapshot;
use super::retrieval_canonical::add_canonical;
use super::retrieval_types::ProjectionItem;

pub(super) fn projection_items(snapshot: &Snapshot) -> Vec<ProjectionItem> {
    let (stale, invalidated) = history_flags(snapshot);
    let mut items = Vec::new();
    add_canonical(snapshot, &mut items);
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
    items
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
        items.push(item(ItemSpec {
            kind: "inventory",
            id: &inventory.id,
            subtype: "inventory",
            title: &format!("Inventory {}", inventory.id),
            summary: &format!("{} indexed materials", inventory.entries.len()),
            occurred_at: Some(&inventory.observed_at),
            state: None,
            authority_path: &authority,
            stale,
            invalidated: false,
        }));
        for material in &inventory.entries {
            items.push(item(ItemSpec {
                kind: "material",
                id: &material.path,
                subtype: &enum_name(&material.class),
                title: &material.path,
                summary: &format!("{} bytes SHA-256 {}", material.bytes, material.sha256),
                occurred_at: Some(&inventory.observed_at),
                state: None,
                authority_path: &authority,
                stale,
                invalidated: false,
            }));
        }
    }
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
