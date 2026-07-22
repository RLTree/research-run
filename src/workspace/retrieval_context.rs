use std::collections::BTreeSet;

use crate::Result;
use crate::domain::{KnowledgeKind, KnowledgeState};

use super::retrieval::{match_reasons, unresolved_kind, validate_query};
use super::retrieval_canonical::relationship_items;
use super::retrieval_items::projection_items;
use super::{ContextBundle, ProjectionItem, Snapshot, Status, Workspace};

pub(super) fn context_from_snapshot(
    workspace: &Workspace,
    snapshot: Snapshot,
    query: Option<&str>,
    limit: usize,
) -> Result<ContextBundle> {
    let items = projection_items(workspace, &snapshot)?;
    let status = workspace.status_from_snapshot(&snapshot)?;
    let matches = match query {
        Some(query) => search_items(items.clone(), query, limit)?,
        None => recent_items(items.clone(), limit),
    };
    let unresolved = knowledge_subset(&snapshot, &items, limit, unresolved_kind);
    let mut blockers = knowledge_subset(&snapshot, &items, limit, |kind| {
        kind == KnowledgeKind::Blocker
    });
    blockers.extend(claim_blockers(&status));
    sort_and_truncate(&mut blockers, limit);
    let mut next_actions = knowledge_subset(&snapshot, &items, limit, |kind| {
        kind == KnowledgeKind::NextAction
    });
    next_actions.extend(claim_next_actions(&status));
    sort_and_truncate(&mut next_actions, limit);
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
        claim_ceiling: super::CLAIM_CEILING.to_owned(),
        scope: query.unwrap_or("recent workspace state").to_owned(),
        contribution_protocol: Some(snapshot.contribution_protocol),
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
    items.sort_by(|left, right| {
        context_priority(left)
            .cmp(&context_priority(right))
            .then_with(|| right.occurred_at.cmp(&left.occurred_at))
            .then_with(|| (&left.kind, &left.id).cmp(&(&right.kind, &right.id)))
    });
    items.truncate(limit);
    items
}

fn context_priority(item: &ProjectionItem) -> u8 {
    match item.kind.as_str() {
        "claim" => 0,
        "evidence" => 1,
        "review" => 2,
        "knowledge" => 3,
        "relationship" => 4,
        _ => 5,
    }
}

pub(super) fn claim_blockers(status: &Status) -> Vec<ProjectionItem> {
    status
        .claims
        .iter()
        .filter(|claim| !claim.blockers.is_empty())
        .map(|claim| claim_action_item(claim, "blocker", claim.blockers.join(" ")))
        .collect()
}

pub(super) fn claim_next_actions(status: &Status) -> Vec<ProjectionItem> {
    status
        .claims
        .iter()
        .map(|claim| claim_action_item(claim, "next-action", claim.next_action.clone()))
        .collect()
}

fn claim_action_item(claim: &super::ClaimStatus, subtype: &str, summary: String) -> ProjectionItem {
    ProjectionItem {
        kind: "claim".to_owned(),
        id: claim.id.clone(),
        subtype: subtype.to_owned(),
        title: format!("{} for claim {}", subtype.replace('-', " "), claim.id),
        summary,
        occurred_at: None,
        state: Some(super::retrieval_items::enum_name(&claim.assessment)),
        authority_path: format!(".research-run/claims/{}.json", claim.id),
        matched_by: Vec::new(),
        stale: false,
        invalidated: false,
    }
}

pub(super) fn sort_and_truncate(items: &mut Vec<ProjectionItem>, limit: usize) {
    items.sort_by(|left, right| {
        (&left.kind, &left.id, &left.subtype).cmp(&(&right.kind, &right.id, &right.subtype))
    });
    items.dedup_by(|left, right| {
        left.kind == right.kind && left.id == right.id && left.subtype == right.subtype
    });
    items.truncate(limit);
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
