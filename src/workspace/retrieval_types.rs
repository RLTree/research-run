use serde::{Deserialize, Serialize};

use crate::domain::{
    ContributionProtocol, MAX_LIST_ITEMS, required_text, validate_id, validate_timestamp,
    validate_workspace_locator,
};
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

impl ProjectionItem {
    fn validate(&self) -> Result<()> {
        if !matches!(
            self.kind.as_str(),
            "source"
                | "claim"
                | "evidence"
                | "experiment"
                | "review"
                | "relationship"
                | "knowledge"
                | "inventory"
                | "material"
        ) {
            return Err(Error::invalid("handoff projection kind", "is unknown"));
        }
        if self.kind == "material" {
            validate_workspace_locator(&self.id)?;
        } else {
            validate_id(&self.id, "handoff projection id")?;
        }
        bounded_text(&self.subtype, "handoff projection subtype")?;
        bounded_text(&self.title, "handoff projection title")?;
        if self.summary.chars().count() > 512 {
            return Err(Error::Budget(
                "handoff projection summary exceeds the 512 character budget".to_owned(),
            ));
        }
        if let Some(occurred_at) = &self.occurred_at {
            validate_timestamp(occurred_at)?;
        }
        if let Some(state) = &self.state {
            bounded_text(state, "handoff projection state")?;
        }
        validate_authority_path(&self.authority_path)?;
        if self.matched_by.len() > 5
            || self.matched_by.iter().any(|field| {
                !matches!(
                    field.as_str(),
                    "id" | "type" | "title" | "summary" | "authority-path"
                )
            })
        {
            return Err(Error::invalid(
                "handoff projection matched_by",
                "contains an unknown or excessive match field",
            ));
        }
        Ok(())
    }
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

impl RelationshipProjection {
    fn validate(&self) -> Result<()> {
        validate_id(&self.id, "handoff relationship id")?;
        if !matches!(
            self.relationship.as_str(),
            "related-to"
                | "depends-on"
                | "derived-from"
                | "uses"
                | "supersedes"
                | "revises"
                | "invalidates"
                | "resolves"
                | "blocks"
                | "contradicts"
        ) {
            return Err(Error::invalid("handoff relationship kind", "is unknown"));
        }
        for (field, kind, id) in [
            ("from", self.from_kind.as_str(), self.from_id.as_str()),
            ("to", self.to_kind.as_str(), self.to_id.as_str()),
        ] {
            if !matches!(
                kind,
                "source"
                    | "claim"
                    | "evidence"
                    | "experiment"
                    | "review"
                    | "knowledge"
                    | "inventory"
            ) {
                return Err(Error::invalid(
                    "handoff relationship endpoint kind",
                    "is unknown",
                ));
            }
            validate_id(id, &format!("handoff relationship {field} id"))?;
        }
        if self.from_kind == self.to_kind && self.from_id == self.to_id {
            return Err(Error::invalid(
                "handoff relationship",
                "endpoints must be distinct",
            ));
        }
        bounded_text(&self.rationale, "handoff relationship rationale")?;
        validate_timestamp(&self.occurred_at)?;
        validate_authority_path(&self.authority_path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextBundle {
    pub kind: String,
    pub project_id: String,
    pub project_name: String,
    pub claim_ceiling: String,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution_protocol: Option<ContributionProtocol>,
    pub matches: Vec<ProjectionItem>,
    pub unresolved: Vec<ProjectionItem>,
    pub blockers: Vec<ProjectionItem>,
    pub next_actions: Vec<ProjectionItem>,
    pub relationships: Vec<RelationshipProjection>,
}

impl ContextBundle {
    pub fn validate(&self) -> Result<()> {
        if self.kind != "context" {
            return Err(Error::invalid("handoff context", "unknown record kind"));
        }
        validate_id(&self.project_id, "handoff project_id")?;
        bounded_text(&self.project_name, "handoff project_name")?;
        if self.claim_ceiling != super::CLAIM_CEILING {
            return Err(Error::invalid(
                "handoff claim_ceiling",
                "must equal the canonical Research Run claim ceiling",
            ));
        }
        bounded_text(&self.scope, "handoff scope")?;
        if let Some(protocol) = &self.contribution_protocol {
            crate::domain::CanonicalRecord::validate(protocol)?;
        }
        for (name, items) in [
            ("matches", &self.matches),
            ("unresolved", &self.unresolved),
            ("blockers", &self.blockers),
            ("next_actions", &self.next_actions),
        ] {
            if items.len() > MAX_LIST_ITEMS {
                return Err(Error::Budget(format!(
                    "handoff {name} exceeds the {MAX_LIST_ITEMS} item budget"
                )));
            }
            for item in items {
                item.validate()?;
            }
        }
        if self.relationships.len() > MAX_LIST_ITEMS {
            return Err(Error::Budget(format!(
                "handoff relationships exceed the {MAX_LIST_ITEMS} item budget"
            )));
        }
        for relationship in &self.relationships {
            relationship.validate()?;
        }
        Ok(())
    }
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
        if !matches!(self.schema_version, 1 | 2) || self.kind != "handoff" {
            return Err(Error::invalid("handoff", "unknown version or record kind"));
        }
        if self.schema_version == 2 && self.context.contribution_protocol.is_none() {
            return Err(Error::invalid(
                "handoff",
                "version 2 requires the contribution protocol",
            ));
        }
        validate_id(&self.id, "handoff id")?;
        validate_timestamp(&self.generated_at)?;
        self.context.validate()
    }
}

fn bounded_text(value: &str, field: &str) -> Result<()> {
    required_text(value, field)?;
    Ok(())
}

fn validate_authority_path(path: &str) -> Result<()> {
    validate_workspace_locator(path)?;
    if !path.starts_with(".research-run/") {
        return Err(Error::invalid(
            "handoff authority_path",
            "must identify canonical .research-run authority",
        ));
    }
    Ok(())
}
