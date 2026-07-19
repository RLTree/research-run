use serde::{Deserialize, Serialize};

use crate::{Error, Result};

use super::validation::{
    required_text, validate_id, validate_record_header, validate_text_list,
    validate_workspace_locator,
};
use super::{Assessment, CanonicalRecord, MAX_ARTIFACT_POINTERS, MAX_LIST_ITEMS, Outcome};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactPointer {
    pub locator_type: ArtifactLocatorType,
    pub locator: String,
    pub description: String,
    pub digest: Option<String>,
}

impl ArtifactPointer {
    pub fn validate(&self) -> Result<()> {
        required_text(&self.locator, "artifact locator")?;
        required_text(&self.description, "artifact description")?;
        if let Some(digest) = &self.digest {
            required_text(digest, "artifact digest")?;
        }
        if self.locator_type == ArtifactLocatorType::Workspace {
            validate_workspace_locator(&self.locator)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactLocatorType {
    Workspace,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExperimentReceipt {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub question: String,
    pub method_ref: String,
    pub observations: Vec<String>,
    pub interpretation: String,
    pub limitations: Vec<String>,
    pub outcome: Outcome,
    pub next_move: String,
    pub artifacts: Vec<ArtifactPointer>,
}

impl CanonicalRecord for ExperimentReceipt {
    const KIND: &'static str = "experiment";

    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self) -> Result<()> {
        validate_record_header(self.schema_version, &self.kind, Self::KIND, &self.id)?;
        required_text(&self.question, "experiment question")?;
        required_text(&self.method_ref, "method reference")?;
        validate_text_list(&self.observations, "observations", true)?;
        required_text(&self.interpretation, "interpretation")?;
        validate_text_list(&self.limitations, "limitations", true)?;
        required_text(&self.next_move, "next move")?;
        if self.artifacts.len() > MAX_ARTIFACT_POINTERS {
            return Err(Error::Budget(format!(
                "experiment {} exceeds the {MAX_ARTIFACT_POINTERS} artifact pointer budget",
                self.id
            )));
        }
        for artifact in &self.artifacts {
            artifact.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReviewDecision {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub claim_id: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_sha256: Option<String>,
    pub decision: Assessment,
    pub rationale: String,
    pub reviewer: String,
}

impl CanonicalRecord for ReviewDecision {
    const KIND: &'static str = "review";

    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self) -> Result<()> {
        validate_record_header(self.schema_version, &self.kind, Self::KIND, &self.id)?;
        validate_id(&self.claim_id, "claim_id")?;
        if self.evidence_ids.len() > MAX_LIST_ITEMS {
            return Err(Error::Budget(format!(
                "review evidence_ids exceeds the {MAX_LIST_ITEMS} item budget"
            )));
        }
        for evidence_id in &self.evidence_ids {
            validate_id(evidence_id, "evidence_id")?;
        }
        if !self.evidence_ids.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(Error::invalid(
                "review evidence_ids",
                "must be sorted and unique",
            ));
        }
        if let Some(digest) = &self.subject_sha256
            && (digest.len() != 64
                || !digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        {
            return Err(Error::invalid(
                "review subject_sha256",
                "must be a lowercase SHA-256 digest",
            ));
        }
        if self.decision == Assessment::Unreviewed {
            return Err(Error::invalid(
                "review decision",
                "human reviews cannot set the unreviewed assessment",
            ));
        }
        if self.decision == Assessment::Supported && self.evidence_ids.is_empty() {
            return Err(Error::invalid(
                "review decision",
                "supported requires at least one recorded evidence link",
            ));
        }
        required_text(&self.rationale, "review rationale")?;
        required_text(&self.reviewer, "reviewer")?;
        Ok(())
    }
}
