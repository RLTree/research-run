use serde::{Deserialize, Serialize};

use crate::{Error, Result};

use super::validation::{
    bounded_text, required_text, slug, validate_header, validate_id, validate_record_header,
    validate_workspace_locator,
};
use super::{CanonicalRecord, FORMAT_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub kind: String,
    pub project_id: String,
    pub name: String,
    pub declared_roots: Vec<String>,
}

impl ProjectManifest {
    pub fn new(name: &str) -> Result<Self> {
        let name = required_text(name, "project name")?;
        let project_id = slug(&name);
        Ok(Self {
            schema_version: FORMAT_VERSION,
            kind: "project-manifest".to_owned(),
            project_id,
            name,
            declared_roots: vec![".".to_owned()],
        })
    }

    pub fn validate(&self) -> Result<()> {
        validate_header(self.schema_version, &self.kind, "project-manifest")?;
        validate_id(&self.project_id, "project_id")?;
        required_text(&self.name, "project name")?;
        if self.declared_roots != ["."] {
            return Err(Error::invalid(
                "manifest",
                "declared_roots must be exactly [\".\"] in format v1",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Authorship {
    Human,
    Ai,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SourceProvenance {
    Human,
    Imported,
    Ai,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Stance {
    Supports,
    Limits,
    Contradicts,
    Context,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Positive,
    Negative,
    Ambiguous,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Assessment {
    Unsupported,
    Limited,
    Supported,
    Contradicted,
    Unreviewed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub citation: String,
    pub locator: String,
    pub provenance: SourceProvenance,
    pub notes: String,
}

impl CanonicalRecord for SourceRecord {
    const KIND: &'static str = "source";

    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self) -> Result<()> {
        validate_record_header(self.schema_version, &self.kind, Self::KIND, &self.id)?;
        required_text(&self.citation, "citation")?;
        required_text(&self.locator, "locator")?;
        bounded_text(&self.notes, "notes")?;
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimRecord {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub text: String,
    pub scope: String,
    pub owner: String,
    pub authorship: Authorship,
}

impl CanonicalRecord for ClaimRecord {
    const KIND: &'static str = "claim";

    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self) -> Result<()> {
        validate_record_header(self.schema_version, &self.kind, Self::KIND, &self.id)?;
        required_text(&self.text, "claim text")?;
        required_text(&self.scope, "claim scope")?;
        required_text(&self.owner, "claim owner")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceLink {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub claim_id: String,
    pub source_id: Option<String>,
    pub experiment_id: Option<String>,
    pub artifact: Option<String>,
    pub stance: Stance,
    pub specific_evidence: String,
    pub authorship: Authorship,
}

impl CanonicalRecord for EvidenceLink {
    const KIND: &'static str = "evidence";

    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self) -> Result<()> {
        validate_record_header(self.schema_version, &self.kind, Self::KIND, &self.id)?;
        validate_id(&self.claim_id, "claim_id")?;
        if let Some(source_id) = &self.source_id {
            validate_id(source_id, "source_id")?;
        }
        if let Some(experiment_id) = &self.experiment_id {
            validate_id(experiment_id, "experiment_id")?;
        }
        if let Some(artifact) = &self.artifact {
            validate_workspace_locator(artifact)?;
        }
        let references = usize::from(self.source_id.is_some())
            + usize::from(self.experiment_id.is_some())
            + usize::from(self.artifact.is_some());
        if references != 1 {
            return Err(Error::invalid(
                "evidence link",
                "exactly one source, experiment, or artifact reference is required",
            ));
        }
        required_text(&self.specific_evidence, "specific evidence")?;
        Ok(())
    }
}
