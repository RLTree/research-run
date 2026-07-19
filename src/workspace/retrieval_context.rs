use std::collections::BTreeSet;

use crate::Result;
use crate::domain::{KnowledgeKind, KnowledgeState};

use super::retrieval::{match_reasons, unresolved_kind, validate_query};
use super::retrieval_canonical::relationship_items;
use super::retrieval_items::projection_items;
use super::{ContextBundle, ProjectionItem, Snapshot};

pub(super) fn context_from_snapshot(
    snapshot: Snapshot,
    query: Option<&str>,
    limit: usize,
) -> Result<ContextBundle> {
    let items = projection_items(&snapshot);
    let matches = match query {
        Some(query) => search_items(items.clone(), query, limit)?,
        None => recent_items(items.clone(), limit),
    };
    let unresolved = knowledge_subset(&snapshot, &items, limit, unresolved_kind);
    let blockers = knowledge_subset(&snapshot, &items, limit, |kind| {
        kind == KnowledgeKind::Blocker
    });
    let next_actions = knowledge_subset(&snapshot, &items, limit, |kind| {
        kind == KnowledgeKind::NextAction
    });
    let ids = matches
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut relationships = relationship_items(&snapshot)
        .into_iter()
        .filter(|item| {
            [item.from_id.as_str(), item.to_id.as_str()]
                .iter()
                .any(|id| ids.contains(id))
        })
        .collect::<Vec<_>>();
    relationships.truncate(limit);
    Ok(ContextBundle {
        kind: "context".to_owned(),
        project_id: snapshot.manifest.project_id,
        project_name: snapshot.manifest.name,
        claim_ceiling:
            "Workspace review only; validation and retrieval do not prove scientific truth."
                .to_owned(),
        scope: query.unwrap_or("recent workspace state").to_owned(),
        matches,
        unresolved,
        blockers,
        next_actions,
        relationships,
    })
}

fn search_items(
    mut items: Vec<ProjectionItem>,
    query: &str,
    limit: usize,
) -> Result<Vec<ProjectionItem>> {
    let query = validate_query(query)?;
    for item in &mut items {
        item.matched_by = match_reasons(item, &query);
    }
    items.retain(|item| !item.matched_by.is_empty());
    items.truncate(limit);
    Ok(items)
}

fn recent_items(mut items: Vec<ProjectionItem>, limit: usize) -> Vec<ProjectionItem> {
    items.retain(|item| item.occurred_at.is_some());
    items.sort_by(|left, right| {
        (&right.occurred_at, &right.kind, &right.id).cmp(&(&left.occurred_at, &left.kind, &left.id))
    });
    items.truncate(limit);
    items
}

fn knowledge_subset(
    snapshot: &Snapshot,
    items: &[ProjectionItem],
    limit: usize,
    predicate: impl Fn(KnowledgeKind) -> bool,
) -> Vec<ProjectionItem> {
    let selected = snapshot
        .knowledge
        .iter()
        .filter(|record| {
            predicate(record.record_type)
                && matches!(record.state, KnowledgeState::Open | KnowledgeState::Active)
        })
        .map(|record| record.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut subset = items
        .iter()
        .filter(|item| item.kind == "knowledge" && selected.contains(item.id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    subset.truncate(limit);
    subset
}
