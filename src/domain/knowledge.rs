use serde::{Deserialize, Serialize};

use crate::{Error, Result};

use super::CanonicalRecord;
use super::validation::{
    bounded_text, required_text, validate_id, validate_record_header, validate_timestamp,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeKind {
    Goal,
    ResearchQuestion,
    Hypothesis,
    Protocol,
    Method,
    Observation,
    Measurement,
    Analysis,
    Interpretation,
    Decision,
    Risk,
    Blocker,
    Uncertainty,
    Contradiction,
    NextAction,
    Plan,
    Presentation,
    SessionSummary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeState {
    Open,
    Active,
    Completed,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeRecord {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub record_type: KnowledgeKind,
    pub title: String,
    pub body: String,
    pub occurred_at: String,
    pub state: KnowledgeState,
    pub authorship: super::Authorship,
}

impl CanonicalRecord for KnowledgeRecord {
    const KIND: &'static str = "knowledge";

    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self) -> Result<()> {
        validate_record_header(self.schema_version, &self.kind, Self::KIND, &self.id)?;
        required_text(&self.title, "knowledge title")?;
        bounded_text(&self.body, "knowledge body")?;
        validate_timestamp(&self.occurred_at)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum EntityKind {
    Source,
    Claim,
    Evidence,
    Experiment,
    Review,
    Knowledge,
    Inventory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct EntityRef {
    pub kind: EntityKind,
    pub id: String,
}

impl EntityRef {
    pub fn validate(&self, field: &str) -> Result<()> {
        validate_id(&self.id, field)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum RelationshipKind {
    RelatedTo,
    DependsOn,
    DerivedFrom,
    Uses,
    Supersedes,
    Revises,
    Invalidates,
    Resolves,
    Blocks,
    Contradicts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RelationshipRecord {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub relationship: RelationshipKind,
    pub from: EntityRef,
    pub to: EntityRef,
    pub rationale: String,
    pub occurred_at: String,
    pub authorship: super::Authorship,
}

impl CanonicalRecord for RelationshipRecord {
    const KIND: &'static str = "relationship";

    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self) -> Result<()> {
        validate_record_header(self.schema_version, &self.kind, Self::KIND, &self.id)?;
        self.from.validate("relationship from id")?;
        self.to.validate("relationship to id")?;
        if self.from == self.to {
            return Err(Error::invalid("relationship", "endpoints must be distinct"));
        }
        if matches!(
            self.relationship,
            RelationshipKind::Supersedes
                | RelationshipKind::Revises
                | RelationshipKind::Invalidates
        ) && (self.from.kind != EntityKind::Knowledge || self.to.kind != EntityKind::Knowledge)
        {
            return Err(Error::invalid(
                "relationship",
                "supersedes, revises, and invalidates require knowledge endpoints",
            ));
        }
        required_text(&self.rationale, "relationship rationale")?;
        validate_timestamp(&self.occurred_at)
    }
}
