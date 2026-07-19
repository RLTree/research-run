use serde::{Deserialize, Serialize};

use crate::domain::{validate_id, validate_timestamp};
use crate::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextBundle {
    pub kind: String,
    pub project_id: String,
    pub project_name: String,
    pub claim_ceiling: String,
    pub scope: String,
    pub matches: Vec<ProjectionItem>,
    pub unresolved: Vec<ProjectionItem>,
    pub blockers: Vec<ProjectionItem>,
    pub next_actions: Vec<ProjectionItem>,
    pub relationships: Vec<RelationshipProjection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffBundle {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub generated_at: String,
    pub context: ContextBundle,
}

impl HandoffBundle {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 || self.kind != "handoff" || self.context.kind != "context" {
            return Err(Error::invalid("handoff", "unknown version or record kind"));
        }
        validate_id(&self.id, "handoff id")?;
        validate_timestamp(&self.generated_at)?;
        validate_id(&self.context.project_id, "handoff project_id")
    }
}
