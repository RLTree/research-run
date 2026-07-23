use serde::{Deserialize, Serialize};

use crate::domain::validation::{
    required_text, validate_header, validate_hex_digest, validate_id, validate_timestamp,
};
use crate::domain::{CanonicalRecord, ReviewAuthority};
use crate::{Error, Result};

use super::{InventoryEntry, InventoryLimits, InventoryPolicy, ReconciliationChange};

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InventoryPlan {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub project_name: String,
    pub project_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub workspace_id: String,
    pub observed_at: String,
    pub previous_snapshot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_authority: Option<ReviewAuthority>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub without_review_authority: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<InventoryPolicy>,
    pub entries: Vec<InventoryEntry>,
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
        if self.review_authority.is_some()
            || self.without_review_authority
            || !self.workspace_id.is_empty()
        {
            validate_hex_digest(&self.workspace_id, "inventory plan workspace_id")?;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<InventoryPolicy>,
    pub entries: Vec<InventoryEntry>,
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
            policy: plan.policy,
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
    fn policy(&self) -> Option<&InventoryPolicy>;
    fn entries(&self) -> &[InventoryEntry];
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
            fn policy(&self) -> Option<&InventoryPolicy> {
                self.policy.as_ref()
            }
            fn entries(&self) -> &[InventoryEntry] {
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
    validate_identity(value)?;
    let limits = resolved_limits(value.policy())?;
    let maximum = usize::try_from(limits.max_entries).map_err(|_| {
        Error::Budget("inventory max_entries does not fit this platform".to_owned())
    })?;
    if value.entries().len() > maximum {
        return Err(Error::Budget(format!(
            "inventory exceeds the {} entry budget",
            limits.max_entries
        )));
    }
    super::validation::validate_entries(value.policy(), value.entries())?;
    validate_changes(value.changes(), maximum)
}

fn validate_identity(value: &impl InventoryFields) -> Result<()> {
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
    Ok(())
}

fn resolved_limits(policy: Option<&InventoryPolicy>) -> Result<InventoryLimits> {
    match policy {
        Some(policy) => {
            policy.validate()?;
            Ok(policy.limits)
        }
        None => Ok(InventoryLimits::legacy()),
    }
}

fn validate_changes(changes: &[ReconciliationChange], maximum: usize) -> Result<()> {
    let maximum = maximum
        .checked_mul(2)
        .ok_or_else(|| Error::Budget("inventory change budget overflowed".to_owned()))?;
    if changes.len() > maximum {
        return Err(Error::Budget(format!(
            "reconciliation exceeds the {maximum} change budget"
        )));
    }
    for change in changes {
        change.validate()?;
    }
    Ok(())
}
