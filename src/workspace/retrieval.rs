use std::collections::BTreeSet;

use crate::domain::{KnowledgeKind, KnowledgeState, MAX_LIST_ITEMS};
use crate::{Error, Result};

use super::Workspace;
use super::retrieval_items::{projection_items, relationship_items};
use super::retrieval_types::{
    ContextBundle, HandoffBundle, ProjectionItem, ProjectionResult, RelationshipProjection,
};

impl Workspace {
    pub fn list(&self, kind: Option<&str>, limit: usize) -> Result<ProjectionResult> {
        let limit = validate_limit(limit)?;
        let mut items = projection_items(&self.load_snapshot()?);
        if let Some(kind) = kind {
            items.retain(|item| item.kind == kind || item.subtype == kind);
        }
        Ok(result("list", items, limit))
    }

    pub fn show(&self, kind: &str, id: &str) -> Result<ProjectionItem> {
        let mut matches = projection_items(&self.load_snapshot()?)
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
        let items = projection_items(&self.load_snapshot()?)
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
        let mut items = projection_items(&self.load_snapshot()?);
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
        let mut items = projection_items(&self.load_snapshot()?);
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
        self.knowledge_subset("blockers", limit, |kind| kind == KnowledgeKind::Blocker)
    }

    pub fn next_action_items(&self, limit: usize) -> Result<ProjectionResult> {
        self.knowledge_subset("next-actions", limit, |kind| {
            kind == KnowledgeKind::NextAction
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
        let selected = snapshot
            .knowledge
            .iter()
            .filter(|record| {
                predicate(record.record_type)
                    && matches!(record.state, KnowledgeState::Open | KnowledgeState::Active)
            })
            .map(|record| record.id.as_str())
            .collect::<BTreeSet<_>>();
        let items = projection_items(&snapshot)
            .into_iter()
            .filter(|item| item.kind == "knowledge" && selected.contains(item.id.as_str()))
            .collect();
        Ok(result(output_kind, items, limit))
    }

    pub fn context(&self, query: Option<&str>, limit: usize) -> Result<ContextBundle> {
        let limit = validate_limit(limit)?;
        let snapshot = self.load_snapshot()?;
        let matches = match query {
            Some(query) => self.search(query, limit)?.items,
            None => self.recent(limit)?.items,
        };
        let unresolved = self.unresolved(limit)?.items;
        let blockers = self.blocker_items(limit)?.items;
        let next_actions = self.next_action_items(limit)?.items;
        let ids = matches
            .iter()
            .map(|item| item.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut relationships = relationship_items(&snapshot)
            .into_iter()
            .filter(|item| ids.contains(item.from_id.as_str()) || ids.contains(item.to_id.as_str()))
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

    pub fn handoff(
        &self,
        id: &str,
        generated_at: &str,
        query: Option<&str>,
        limit: usize,
    ) -> Result<HandoffBundle> {
        let bundle = HandoffBundle {
            schema_version: 1,
            kind: "handoff".to_owned(),
            id: id.to_owned(),
            generated_at: generated_at.to_owned(),
            context: self.context(query, limit)?,
        };
        bundle.validate()?;
        Ok(bundle)
    }
}

fn unresolved_kind(kind: KnowledgeKind) -> bool {
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

fn validate_limit(limit: usize) -> Result<usize> {
    if (1..=MAX_LIST_ITEMS).contains(&limit) {
        Ok(limit)
    } else {
        Err(Error::Budget(format!(
            "retrieval limit must be between 1 and {MAX_LIST_ITEMS}"
        )))
    }
}

fn validate_query(query: &str) -> Result<String> {
    let query = query.trim();
    if query.is_empty() || query.len() > 256 {
        return Err(Error::invalid(
            "search query",
            "must contain 1 to 256 bytes",
        ));
    }
    Ok(query.to_lowercase())
}

fn match_reasons(item: &ProjectionItem, query: &str) -> Vec<String> {
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
