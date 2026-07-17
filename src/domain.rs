use std::collections::BTreeSet;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_TEXT_BYTES: usize = 65_536;
pub const MAX_LIST_ITEMS: usize = 256;
pub const MAX_ARTIFACT_POINTERS: usize = 128;

pub trait CanonicalRecord: Serialize + Sized {
    const KIND: &'static str;

    fn id(&self) -> &str;
    fn validate(&self) -> Result<()>;
}

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
        validate_id(&project_id, "project_id")?;
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
        if self.decision == Assessment::Unreviewed {
            return Err(Error::invalid(
                "review decision",
                "human reviews cannot set the unreviewed assessment",
            ));
        }
        required_text(&self.rationale, "review rationale")?;
        required_text(&self.reviewer, "reviewer")?;
        Ok(())
    }
}

fn validate_header(version: u32, kind: &str, expected_kind: &str) -> Result<()> {
    if version != FORMAT_VERSION {
        return Err(Error::invalid(
            "schema_version",
            format!("expected {FORMAT_VERSION}, found {version}"),
        ));
    }
    if kind != expected_kind {
        return Err(Error::invalid(
            "record kind",
            format!("expected {expected_kind}"),
        ));
    }
    Ok(())
}

fn validate_record_header(version: u32, kind: &str, expected: &str, id: &str) -> Result<()> {
    validate_header(version, kind, expected)?;
    validate_id(id, "id")
}

pub fn validate_id(value: &str, field: &str) -> Result<()> {
    if value.is_empty() || value.len() > 64 {
        return Err(Error::invalid(field, "must contain 1 to 64 bytes"));
    }
    let mut bytes = value.bytes();
    if !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(Error::invalid(
            field,
            "must start with a lowercase ASCII letter and contain only lowercase letters, digits, or '-'",
        ));
    }
    Ok(())
}

pub fn required_text(value: &str, field: &str) -> Result<String> {
    bounded_text(value, field)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Error::invalid(field, "must not be empty"));
    }
    Ok(trimmed.to_owned())
}

fn bounded_text(value: &str, field: &str) -> Result<()> {
    if value.len() > MAX_TEXT_BYTES {
        return Err(Error::Budget(format!(
            "{field} exceeds the {MAX_TEXT_BYTES} byte text budget"
        )));
    }
    Ok(())
}

fn validate_text_list(values: &[String], field: &str, required: bool) -> Result<()> {
    if required && values.is_empty() {
        return Err(Error::invalid(field, "must contain at least one item"));
    }
    if values.len() > MAX_LIST_ITEMS {
        return Err(Error::Budget(format!(
            "{field} exceeds the {MAX_LIST_ITEMS} item budget"
        )));
    }
    for value in values {
        required_text(value, field)?;
    }
    Ok(())
}

pub fn validate_workspace_locator(value: &str) -> Result<()> {
    let path = Path::new(value);
    if path.is_absolute() || value.is_empty() {
        return Err(Error::invalid(
            "workspace locator",
            "must be a non-empty relative path",
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(Error::invalid(
            "workspace locator",
            "parent traversal and absolute paths are forbidden",
        ));
    }
    Ok(())
}

pub fn ensure_unique_ids<'a>(ids: impl Iterator<Item = &'a str>, kind: &str) -> Result<()> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(Error::invalid(kind, format!("duplicate identity {id}")));
        }
    }
    Ok(())
}

fn slug(name: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('-');
            }
            result.push(character.to_ascii_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }
    if result.is_empty() {
        result.push_str("project");
    } else if !result.as_bytes()[0].is_ascii_lowercase() {
        result.insert_str(0, "project-");
    }
    result.truncate(64);
    while result.ends_with('-') {
        result.pop();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{ProjectManifest, validate_id, validate_workspace_locator};

    #[test]
    fn identifiers_and_paths_fail_closed() {
        assert!(validate_id("claim-one", "id").is_ok());
        assert!(validate_id("../claim", "id").is_err());
        assert!(validate_workspace_locator("artifacts/summary.txt").is_ok());
        assert!(validate_workspace_locator("../secret").is_err());
        assert!(validate_workspace_locator("/tmp/secret").is_err());
    }

    #[test]
    fn manifest_identity_is_deterministic() {
        let manifest = ProjectManifest::new("Synthetic Assay").expect("manifest");
        assert_eq!(manifest.project_id, "synthetic-assay");
    }
}
