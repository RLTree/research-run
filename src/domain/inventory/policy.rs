use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

use super::MaterialClass;
use crate::domain::experiments::{ArtifactLocatorType, ArtifactPointer};
use crate::domain::validation::{required_text, validate_header, validate_workspace_locator};

pub const DEFAULT_MAX_ENTRIES: u64 = 20_000;
pub const DEFAULT_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const DEFAULT_MAX_TOTAL_BYTES: u64 = 8 * 1024 * 1024 * 1024;
pub const LEGACY_MAX_ENTRIES: u64 = 2_048;
pub const LEGACY_MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const LEGACY_MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InventoryLimits {
    pub max_entries: u64,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
}

impl InventoryLimits {
    pub const fn legacy() -> Self {
        Self {
            max_entries: LEGACY_MAX_ENTRIES,
            max_file_bytes: LEGACY_MAX_FILE_BYTES,
            max_total_bytes: LEGACY_MAX_TOTAL_BYTES,
        }
    }

    pub fn validate(self) -> Result<()> {
        if self.max_entries == 0 || self.max_file_bytes == 0 || self.max_total_bytes == 0 {
            return Err(Error::invalid(
                "inventory limits",
                "budgets must be positive",
            ));
        }
        if self.max_file_bytes > self.max_total_bytes {
            return Err(Error::invalid(
                "inventory limits",
                "max_file_bytes cannot exceed max_total_bytes",
            ));
        }
        usize::try_from(self.max_entries).map_err(|_| {
            Error::Budget("inventory max_entries does not fit this platform".to_owned())
        })?;
        Ok(())
    }
}

impl Default for InventoryLimits {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_ENTRIES,
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InventoryPolicy {
    pub schema_version: u32,
    pub kind: String,
    pub limits: InventoryLimits,
    pub boundaries: Vec<InventoryBoundary>,
}

impl Default for InventoryPolicy {
    fn default() -> Self {
        Self {
            schema_version: 1,
            kind: "inventory-policy".to_owned(),
            limits: InventoryLimits::default(),
            boundaries: Vec::new(),
        }
    }
}

impl InventoryPolicy {
    pub fn canonicalized(mut self) -> Result<Self> {
        self.boundaries.sort_by(boundary_order);
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<()> {
        validate_header(self.schema_version, &self.kind, "inventory-policy")?;
        self.limits.validate()?;
        if u64::try_from(self.boundaries.len()).unwrap_or(u64::MAX) > self.limits.max_entries {
            return Err(Error::Budget(
                "inventory boundaries exceed the configured entry budget".to_owned(),
            ));
        }
        for boundary in &self.boundaries {
            boundary.validate()?;
        }
        for pair in self.boundaries.windows(2) {
            if boundary_order(&pair[0], &pair[1]) != Ordering::Less {
                return Err(Error::invalid(
                    "inventory boundaries",
                    "must be unique and sorted by canonical path",
                ));
            }
            if path_contains(pair[0].path(), pair[1].path()) {
                return Err(Error::invalid(
                    "inventory boundaries",
                    "declarations cannot overlap by ancestry",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "boundary_kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum InventoryBoundary {
    ReferenceOnly {
        path: String,
        class: MaterialClass,
        rationale: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reference: Option<DeclaredReference>,
    },
    ChildWorkspace {
        path: String,
        rationale: String,
    },
}

impl InventoryBoundary {
    pub fn path(&self) -> &str {
        match self {
            Self::ReferenceOnly { path, .. } | Self::ChildWorkspace { path, .. } => path,
        }
    }

    fn validate(&self) -> Result<()> {
        validate_boundary_path(self.path())?;
        match self {
            Self::ReferenceOnly {
                rationale,
                reference,
                ..
            } => {
                required_text(rationale, "reference-only rationale")?;
                if let Some(reference) = reference {
                    reference.validate()?;
                }
            }
            Self::ChildWorkspace { rationale, .. } => {
                required_text(rationale, "child-workspace rationale")?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum DeclaredReferenceKind {
    Provenance,
    Manifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeclaredReference {
    pub reference_kind: DeclaredReferenceKind,
    pub locator_type: ArtifactLocatorType,
    pub locator: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

impl DeclaredReference {
    pub fn validate(&self) -> Result<()> {
        ArtifactPointer {
            locator_type: self.locator_type,
            locator: self.locator.clone(),
            description: self.description.clone(),
            digest: self.sha256.clone(),
        }
        .validate()?;
        if let Some(digest) = &self.sha256 {
            crate::domain::validation::validate_hex_digest(digest, "declared reference sha256")?;
        }
        Ok(())
    }
}

fn validate_boundary_path(path: &str) -> Result<()> {
    validate_workspace_locator(path)?;
    if path
        .split('/')
        .any(|component| matches!(component, ".git" | ".research-run"))
    {
        return Err(Error::invalid(
            "inventory boundary path",
            "cannot name Git or Research Run control paths",
        ));
    }
    Ok(())
}

fn boundary_order(left: &InventoryBoundary, right: &InventoryBoundary) -> Ordering {
    left.path().cmp(right.path())
}

fn path_contains(parent: &str, child: &str) -> bool {
    child
        .strip_prefix(parent)
        .is_some_and(|suffix| suffix.starts_with('/'))
}
