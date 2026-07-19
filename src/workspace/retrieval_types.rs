use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectionItem {
    pub kind: String,
    pub id: String,
    pub subtype: String,
    pub title: String,
    pub summary: String,
    pub occurred_at: Option<String>,
    pub state: Option<String>,
    pub authority_path: String,
    pub matched_by: Vec<String>,
    pub stale: bool,
    pub invalidated: bool,
}

#[derive(Debug, Serialize)]
pub struct ProjectionResult {
    pub kind: &'static str,
    pub limit: usize,
    pub total_matches: usize,
    pub items: Vec<ProjectionItem>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RelationshipProjection {
    pub id: String,
    pub relationship: String,
    pub from_kind: String,
    pub from_id: String,
    pub to_kind: String,
    pub to_id: String,
    pub rationale: String,
    pub occurred_at: String,
    pub authority_path: String,
}

#[derive(Debug, Serialize)]
pub struct ContextBundle {
    pub kind: &'static str,
    pub project_id: String,
    pub project_name: String,
    pub claim_ceiling: &'static str,
    pub scope: String,
    pub matches: Vec<ProjectionItem>,
    pub unresolved: Vec<ProjectionItem>,
    pub blockers: Vec<ProjectionItem>,
    pub next_actions: Vec<ProjectionItem>,
    pub relationships: Vec<RelationshipProjection>,
}
