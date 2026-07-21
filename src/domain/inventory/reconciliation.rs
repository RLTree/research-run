use serde::{Deserialize, Serialize};

use crate::domain::validation::required_text;
use crate::{Result, domain::validate_workspace_locator};

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
    pub(super) fn validate(&self) -> Result<()> {
        if let Some(path) = &self.before {
            validate_workspace_locator(path)?;
        }
        if let Some(path) = &self.after {
            validate_workspace_locator(path)?;
        }
        required_text(&self.detail, "reconciliation detail")?;
        Ok(())
    }
}
