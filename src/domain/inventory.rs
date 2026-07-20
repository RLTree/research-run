use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

use super::validation::{
    required_text, validate_header, validate_hex_digest, validate_id, validate_timestamp,
};
use super::{CanonicalRecord, MAX_INVENTORY_ENTRIES, ReviewAuthority};

fn is_false(value: &bool) -> bool {
    !*value
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct MaterialEntry {
    pub path: String,
    pub class: MaterialClass,
    pub bytes: u64,
    pub sha256: String,
}

impl MaterialEntry {
    fn validate(&self) -> Result<()> {
        super::validate_workspace_locator(&self.path)?;
        validate_hex_digest(&self.sha256, "material sha256")
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum ReconciliationKind {
    Added,
    Changed,
    Moved,
    Missing,
    Duplicate,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationChange {
    pub kind: ReconciliationKind,
    pub before: Option<String>,
    pub after: Option<String>,
    pub detail: String,
}

impl ReconciliationChange {
    fn validate(&self) -> Result<()> {
        if let Some(path) = &self.before {
            super::validate_workspace_locator(path)?;
        }
        if let Some(path) = &self.after {
            super::validate_workspace_locator(path)?;
        }
        required_text(&self.detail, "reconciliation detail")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InventoryPlan {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub project_name: String,
    pub project_id: String,
    pub observed_at: String,
    pub previous_snapshot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_authority: Option<ReviewAuthority>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub without_review_authority: bool,
    pub entries: Vec<MaterialEntry>,
    pub changes: Vec<ReconciliationChange>,
}

impl InventoryPlan {
    pub fn validate(&self) -> Result<()> {
        validate_header(self.schema_version, &self.kind, "inventory-plan")?;
        if self.review_authority.is_some() && self.without_review_authority {
            return Err(Error::invalid(
                "inventory plan review authority",
                "authority and unanchored opt-out are mutually exclusive",
            ));
        }
        if let Some(authority) = &self.review_authority {
            authority.validate()?;
        }
        validate_common(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InventorySnapshot {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub project_name: String,
    pub project_id: String,
    pub observed_at: String,
    pub previous_snapshot_id: Option<String>,
    pub entries: Vec<MaterialEntry>,
    pub changes: Vec<ReconciliationChange>,
}

impl From<InventoryPlan> for InventorySnapshot {
    fn from(plan: InventoryPlan) -> Self {
        Self {
            schema_version: plan.schema_version,
            kind: "inventory".to_owned(),
            id: plan.id,
            project_name: plan.project_name,
            project_id: plan.project_id,
            observed_at: plan.observed_at,
            previous_snapshot_id: plan.previous_snapshot_id,
            entries: plan.entries,
            changes: plan.changes,
        }
    }
}

impl CanonicalRecord for InventorySnapshot {
    const KIND: &'static str = "inventory";

    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self) -> Result<()> {
        validate_header(self.schema_version, &self.kind, Self::KIND)?;
        validate_common(self)
    }
}

trait InventoryFields {
    fn id(&self) -> &str;
    fn project_name(&self) -> &str;
    fn project_id(&self) -> &str;
    fn observed_at(&self) -> &str;
    fn previous_snapshot_id(&self) -> Option<&str>;
    fn entries(&self) -> &[MaterialEntry];
    fn changes(&self) -> &[ReconciliationChange];
}

macro_rules! inventory_fields {
    ($type:ty) => {
        impl InventoryFields for $type {
            fn id(&self) -> &str {
                &self.id
            }
            fn project_name(&self) -> &str {
                &self.project_name
            }
            fn project_id(&self) -> &str {
                &self.project_id
            }
            fn observed_at(&self) -> &str {
                &self.observed_at
            }
            fn previous_snapshot_id(&self) -> Option<&str> {
                self.previous_snapshot_id.as_deref()
            }
            fn entries(&self) -> &[MaterialEntry] {
                &self.entries
            }
            fn changes(&self) -> &[ReconciliationChange] {
                &self.changes
            }
        }
    };
}

inventory_fields!(InventoryPlan);
inventory_fields!(InventorySnapshot);

fn validate_common(value: &impl InventoryFields) -> Result<()> {
    validate_id(value.id(), "inventory id")?;
    required_text(value.project_name(), "project name")?;
    validate_id(value.project_id(), "project_id")?;
    validate_timestamp(value.observed_at())?;
    if let Some(previous) = value.previous_snapshot_id() {
        validate_id(previous, "previous_snapshot_id")?;
        if previous == value.id() {
            return Err(Error::invalid(
                "inventory",
                "cannot reference itself as previous",
            ));
        }
    }
    if value.entries().len() > MAX_INVENTORY_ENTRIES {
        return Err(Error::Budget(format!(
            "inventory exceeds the {MAX_INVENTORY_ENTRIES} entry budget"
        )));
    }
    let mut paths = BTreeSet::new();
    for entry in value.entries() {
        entry.validate()?;
        if !paths.insert(&entry.path) {
            return Err(Error::invalid("inventory", "contains duplicate paths"));
        }
    }
    if value.changes().len() > MAX_INVENTORY_ENTRIES * 2 {
        return Err(Error::Budget(format!(
            "reconciliation exceeds the {} change budget",
            MAX_INVENTORY_ENTRIES * 2
        )));
    }
    for change in value.changes() {
        change.validate()?;
    }
    Ok(())
}
