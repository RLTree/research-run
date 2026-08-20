use std::collections::BTreeSet;

use crate::domain::{KnowledgeKind, KnowledgeState, MAX_LIST_ITEMS};
use crate::{Error, Result};

use super::Workspace;
use super::retrieval_canonical::relationship_items;
use super::retrieval_context::{
    claim_blockers, claim_next_actions, context_from_snapshot, sort_and_truncate,
};
use super::retrieval_items::projection_items;
use super::retrieval_types::{
    ContextBundle, ProjectionItem, ProjectionResult, RelationshipProjection,
};

impl Workspace {
    pub fn list(&self, kind: Option<&str>, limit: usize) -> Result<ProjectionResult> {
        let limit = validate_limit(limit)?;
        let snapshot = self.load_snapshot()?;
        let mut items = projection_items(self, &snapshot)?;
        if let Some(kind) = kind {
            items.retain(|item| item.kind == kind || item.subtype == kind);
        }
        Ok(result("list", items, limit))
    }

    pub fn show(&self, kind: &str, id: &str) -> Result<ProjectionItem> {
        let snapshot = self.load_snapshot()?;
        let mut matches = projection_items(self, &snapshot)?
            .into_iter()
            .filter(|item| item.kind == kind && item.id == id);
        let item = matches
            .next()
            .ok_or_else(|| Error::NotFound(format!("no {kind} record with id {id}")))?;
        if matches.next().is_some() {
            return Err(Error::Conflict(format!(
                "multiple {kind} projections share id {id}; use canonical authority paths"
            )));
        }
        Ok(item)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<ProjectionResult> {
        let limit = validate_limit(limit)?;
        let query = validate_query(query)?;
        let snapshot = self.load_snapshot()?;
        let items = projection_items(self, &snapshot)?
            .into_iter()
            .filter_map(|mut item| {
                item.matched_by = match_reasons(&item, &query);
                (!item.matched_by.is_empty()).then_some(item)
            })
            .collect();
        Ok(result("search", items, limit))
    }

    pub fn recent(&self, limit: usize) -> Result<ProjectionResult> {
        let limit = validate_limit(limit)?;
        let snapshot = self.load_snapshot()?;
        let mut items = projection_items(self, &snapshot)?;
        items.retain(|item| item.occurred_at.is_some());
        items.sort_by(|left, right| {
            (&right.occurred_at, &right.kind, &right.id).cmp(&(
                &left.occurred_at,
                &left.kind,
                &left.id,
            ))
        });
        Ok(result("recent", items, limit))
    }

    pub fn timeline(&self, limit: usize) -> Result<ProjectionResult> {
        let limit = validate_limit(limit)?;
        let snapshot = self.load_snapshot()?;
        let mut items = projection_items(self, &snapshot)?;
        items.retain(|item| item.occurred_at.is_some());
        items.sort_by(|left, right| {
            (&left.occurred_at, &left.kind, &left.id).cmp(&(
                &right.occurred_at,
                &right.kind,
                &right.id,
            ))
        });
        Ok(result("timeline", items, limit))
    }

    pub fn related(
        &self,
        kind: &str,
        id: &str,
        limit: usize,
    ) -> Result<Vec<RelationshipProjection>> {
        let limit = validate_limit(limit)?;
        let snapshot = self.load_snapshot()?;
        let mut items = relationship_items(&snapshot)
            .into_iter()
            .filter(|item| {
                (item.from_kind == kind && item.from_id == id)
                    || (item.to_kind == kind && item.to_id == id)
            })
            .collect::<Vec<_>>();
        items.truncate(limit);
        Ok(items)
    }

    pub fn unresolved(&self, limit: usize) -> Result<ProjectionResult> {
        self.knowledge_subset("unresolved", limit, unresolved_kind)
    }

    pub fn blocker_items(&self, limit: usize) -> Result<ProjectionResult> {
        let limit = validate_limit(limit)?;
        let snapshot = self.load_snapshot()?;
        let mut items = self.knowledge_items(&snapshot, |kind| kind == KnowledgeKind::Blocker)?;
        items.extend(claim_blockers(&self.status_from_snapshot(&snapshot)?));
        let total_matches = items.len();
        sort_and_truncate(&mut items, limit);
        Ok(ProjectionResult {
            kind: "blockers",
            limit,
            total_matches,
            items,
        })
    }

    pub fn next_action_items(&self, limit: usize) -> Result<ProjectionResult> {
        let limit = validate_limit(limit)?;
        let snapshot = self.load_snapshot()?;
        let mut items =
            self.knowledge_items(&snapshot, |kind| kind == KnowledgeKind::NextAction)?;
        items.extend(claim_next_actions(&self.status_from_snapshot(&snapshot)?));
        let total_matches = items.len();
        sort_and_truncate(&mut items, limit);
        Ok(ProjectionResult {
            kind: "next-actions",
            limit,
            total_matches,
            items,
        })
    }

    fn knowledge_subset(
        &self,
        output_kind: &'static str,
        limit: usize,
        predicate: impl Fn(KnowledgeKind) -> bool,
    ) -> Result<ProjectionResult> {
        let limit = validate_limit(limit)?;
        let snapshot = self.load_snapshot()?;
        let items = self.knowledge_items(&snapshot, predicate)?;
        Ok(result(output_kind, items, limit))
    }

    fn knowledge_items(
        &self,
        snapshot: &super::Snapshot,
        predicate: impl Fn(KnowledgeKind) -> bool,
    ) -> Result<Vec<ProjectionItem>> {
        let selected = snapshot
            .knowledge
            .iter()
            .filter(|record| {
                predicate(record.record_type)
                    && matches!(record.state, KnowledgeState::Open | KnowledgeState::Active)
            })
            .map(|record| record.id.as_str())
            .collect::<BTreeSet<_>>();
        let items = projection_items(self, snapshot)?
            .into_iter()
            .filter(|item| item.kind == "knowledge" && selected.contains(item.id.as_str()))
            .collect();
        Ok(items)
    }

    pub fn context(&self, query: Option<&str>, limit: usize) -> Result<ContextBundle> {
        let limit = validate_limit(limit)?;
        let snapshot = self.load_snapshot()?;
        context_from_snapshot(self, snapshot, query, limit)
    }
}

pub(super) fn unresolved_kind(kind: KnowledgeKind) -> bool {
    matches!(
        kind,
        KnowledgeKind::ResearchQuestion
            | KnowledgeKind::Hypothesis
            | KnowledgeKind::Risk
            | KnowledgeKind::Blocker
            | KnowledgeKind::Uncertainty
            | KnowledgeKind::Contradiction
    )
}

pub(super) fn validate_limit(limit: usize) -> Result<usize> {
    if (1..=MAX_LIST_ITEMS).contains(&limit) {
        Ok(limit)
    } else {
        Err(Error::Budget(format!(
            "retrieval limit must be between 1 and {MAX_LIST_ITEMS}"
        )))
    }
}

pub(super) fn validate_query(query: &str) -> Result<String> {
    let query = query.trim();
    if query.is_empty() || query.len() > 256 {
        return Err(Error::invalid(
            "search query",
            "must contain 1 to 256 bytes",
        ));
    }
    Ok(query.to_lowercase())
}

pub(super) fn match_reasons(item: &ProjectionItem, query: &str) -> Vec<String> {
    [
        ("id", item.id.as_str()),
        ("type", item.subtype.as_str()),
        ("title", item.title.as_str()),
        ("summary", item.summary.as_str()),
        ("authority-path", item.authority_path.as_str()),
    ]
    .into_iter()
    .filter(|(_, value)| value.to_lowercase().contains(query))
    .map(|(field, _)| field.to_owned())
    .collect()
}

fn result(kind: &'static str, mut items: Vec<ProjectionItem>, limit: usize) -> ProjectionResult {
    let total_matches = items.len();
    items.truncate(limit);
    ProjectionResult {
        kind,
        limit,
        total_matches,
        items,
    }
}
