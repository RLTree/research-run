use std::collections::BTreeSet;

use crate::domain::{Assessment, RelationshipKind};

use super::Snapshot;
use super::references::current_review_bindings;
use super::retrieval_types::{ProjectionItem, RelationshipProjection};

pub(super) fn projection_items(snapshot: &Snapshot) -> Vec<ProjectionItem> {
    let (stale, invalidated) = history_flags(snapshot);
    let mut items = Vec::new();
    add_sources(snapshot, &mut items);
    add_claims(snapshot, &mut items);
    add_experiments(snapshot, &mut items);
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

pub(super) fn relationship_items(snapshot: &Snapshot) -> Vec<RelationshipProjection> {
    let mut items = snapshot
        .relationships
        .iter()
        .map(|record| RelationshipProjection {
            id: record.id.clone(),
            relationship: enum_name(&record.relationship),
            from_kind: enum_name(&record.from.kind),
            from_id: record.from.id.clone(),
            to_kind: enum_name(&record.to.kind),
            to_id: record.to.id.clone(),
            rationale: record.rationale.clone(),
            occurred_at: record.occurred_at.clone(),
            authority_path: format!(".research-run/relationships/{}.json", record.id),
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    items
}

fn add_sources(snapshot: &Snapshot, items: &mut Vec<ProjectionItem>) {
    for record in &snapshot.sources {
        items.push(item(ItemSpec {
            kind: "source",
            id: &record.id,
            subtype: &enum_name(&record.provenance),
            title: &record.citation,
            summary: &format!("{} {}", record.locator, record.notes),
            occurred_at: None,
            state: None,
            authority_path: &format!(".research-run/sources/{}.json", record.id),
            stale: false,
            invalidated: false,
        }));
    }
}

fn add_claims(snapshot: &Snapshot, items: &mut Vec<ProjectionItem>) {
    let reviews = current_review_bindings(snapshot);
    for record in &snapshot.claims {
        let assessment = reviews
            .get(record.id.as_str())
            .map(|review| review.decision)
            .unwrap_or(Assessment::Unreviewed);
        items.push(item(ItemSpec {
            kind: "claim",
            id: &record.id,
            subtype: "claim",
            title: &record.text,
            summary: &format!("{} Owner: {}", record.scope, record.owner),
            occurred_at: None,
            state: Some(&enum_name(&assessment)),
            authority_path: &format!(".research-run/claims/{}.json", record.id),
            stale: false,
            invalidated: false,
        }));
    }
}

fn add_experiments(snapshot: &Snapshot, items: &mut Vec<ProjectionItem>) {
    for record in &snapshot.experiments {
        items.push(item(ItemSpec {
            kind: "experiment",
            id: &record.id,
            subtype: &enum_name(&record.outcome),
            title: &record.question,
            summary: &format!(
                "{} {}",
                record.observations.join(" "),
                record.interpretation
            ),
            occurred_at: None,
            state: Some(&enum_name(&record.outcome)),
            authority_path: &format!(".research-run/experiments/{}.json", record.id),
            stale: false,
            invalidated: false,
        }));
    }
}

fn add_inventories(snapshot: &Snapshot, items: &mut Vec<ProjectionItem>) {
    for inventory in &snapshot.inventories {
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
            stale: false,
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
                stale: inventory.previous_snapshot_id.is_some(),
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

struct ItemSpec<'a> {
    kind: &'a str,
    id: &'a str,
    subtype: &'a str,
    title: &'a str,
    summary: &'a str,
    occurred_at: Option<&'a str>,
    state: Option<&'a str>,
    authority_path: &'a str,
    stale: bool,
    invalidated: bool,
}

fn item(spec: ItemSpec<'_>) -> ProjectionItem {
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

fn enum_name(value: &impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .expect("domain enum serialization is infallible")
        .as_str()
        .expect("domain enums serialize as strings")
        .to_owned()
}
