use serde::{Deserialize, Serialize};

use crate::{Error, Result};

use super::DeclaredReference;
use crate::domain::validate_workspace_locator;
use crate::domain::validation::{
    required_text, validate_hex_digest, validate_id, validate_timestamp,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum InventoryEntry {
    Indexed(MaterialEntry),
    Boundary(BoundaryEntry),
}

impl InventoryEntry {
    pub fn path(&self) -> &str {
        match self {
            Self::Indexed(entry) => &entry.path,
            Self::Boundary(entry) => entry.path(),
        }
    }

    pub fn indexed(&self) -> Option<&MaterialEntry> {
        match self {
            Self::Indexed(entry) => Some(entry),
            Self::Boundary(_) => None,
        }
    }

    pub(super) fn validate(&self) -> Result<()> {
        match self {
            Self::Indexed(entry) => entry.validate(),
            Self::Boundary(entry) => entry.validate(),
        }
    }
}

impl From<MaterialEntry> for InventoryEntry {
    fn from(value: MaterialEntry) -> Self {
        Self::Indexed(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "entry_kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum BoundaryEntry {
    ReferenceOnly {
        path: String,
        class: MaterialClass,
        declared_rationale: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        declared_reference: Option<DeclaredReference>,
        root_observation: RootObservation,
        content_status: InspectionStatus,
    },
    ChildWorkspace {
        path: String,
        declared_rationale: String,
        workspace_observation: ChildWorkspaceObservation,
        child_material_status: InspectionStatus,
    },
}

impl BoundaryEntry {
    pub fn path(&self) -> &str {
        match self {
            Self::ReferenceOnly { path, .. } | Self::ChildWorkspace { path, .. } => path,
        }
    }

    fn validate(&self) -> Result<()> {
        validate_workspace_locator(self.path())?;
        match self {
            Self::ReferenceOnly {
                declared_rationale,
                declared_reference,
                root_observation,
                content_status,
                ..
            } => {
                required_text(declared_rationale, "reference-only rationale")?;
                if let Some(reference) = declared_reference {
                    reference.validate()?;
                }
                root_observation.validate()?;
                content_status.validate()?;
            }
            Self::ChildWorkspace {
                declared_rationale,
                workspace_observation,
                child_material_status,
                ..
            } => {
                required_text(declared_rationale, "child-workspace rationale")?;
                workspace_observation.validate()?;
                child_material_status.validate()?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InspectionStatus {
    NotInspected,
}

impl InspectionStatus {
    fn validate(self) -> Result<()> {
        match self {
            Self::NotInspected => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RootNodeKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RootObservation {
    pub node_kind: RootNodeKind,
    pub bytes: Option<u64>,
}

impl RootObservation {
    fn validate(&self) -> Result<()> {
        if matches!(
            (self.node_kind, self.bytes),
            (RootNodeKind::File, None) | (RootNodeKind::Directory, Some(_))
        ) {
            return Err(Error::invalid(
                "reference-only root observation",
                "file roots require bytes and directory roots cannot summarize bytes",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChildWorkspaceObservation {
    pub manifest_path: String,
    pub project_name: String,
    pub project_id: String,
    pub workspace_id: String,
    pub manifest_sha256: String,
    pub latest_inventory: Option<ChildInventoryIdentity>,
}

impl ChildWorkspaceObservation {
    fn validate(&self) -> Result<()> {
        validate_workspace_locator(&self.manifest_path)?;
        required_text(&self.project_name, "child project name")?;
        validate_id(&self.project_id, "child project_id")?;
        validate_hex_digest(&self.workspace_id, "child workspace_id")?;
        validate_hex_digest(&self.manifest_sha256, "child manifest sha256")?;
        if let Some(inventory) = &self.latest_inventory {
            inventory.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChildInventoryIdentity {
    pub id: String,
    pub observed_at: String,
    pub sha256: String,
}

impl ChildInventoryIdentity {
    fn validate(&self) -> Result<()> {
        validate_id(&self.id, "child inventory id")?;
        validate_timestamp(&self.observed_at)?;
        validate_hex_digest(&self.sha256, "child inventory sha256")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MaterialEntry {
    pub path: String,
    pub class: MaterialClass,
    pub bytes: u64,
    pub sha256: String,
}

impl MaterialEntry {
    pub(super) fn validate(&self) -> Result<()> {
        validate_workspace_locator(&self.path)?;
        validate_hex_digest(&self.sha256, "material sha256")
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum MaterialClass {
    Note,
    Protocol,
    Source,
    Experiment,
    Observation,
    Analysis,
    Decision,
    Artifact,
    Plan,
    Presentation,
    Unknown,
}
