use serde::{Deserialize, Serialize};

use crate::domain::{digest, validate_id, validate_timestamp};
use crate::{Error, Result};

use super::{ContextBundle, ValidationResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoffValidationReceipt {
    pub result: ValidationResult,
    pub result_sha256: String,
    pub context_sha256: String,
}

impl HandoffValidationReceipt {
    pub(super) fn new(result: ValidationResult, context: &ContextBundle) -> Self {
        let result_sha256 = digest(&serde_json::to_vec(&result).expect("validation is JSON"));
        let context_sha256 = digest(&serde_json::to_vec(context).expect("context is JSON"));
        Self {
            result,
            result_sha256,
            context_sha256,
        }
    }

    fn validate(&self, context: &ContextBundle) -> Result<()> {
        if !self.result.is_handoff_receipt_for(
            &context.project_id,
            context.workspace_id.as_deref(),
            context.contribution_protocol.as_ref(),
        ) {
            return Err(Error::invalid(
                "handoff validation receipt",
                "result is invalid, contradictory, or bound to another project",
            ));
        }
        let actual = digest(&serde_json::to_vec(&self.result).expect("validation is JSON"));
        if actual != self.result_sha256 {
            return Err(Error::invalid(
                "handoff validation receipt",
                "result_sha256 does not match the embedded result",
            ));
        }
        let context_sha256 = digest(&serde_json::to_vec(context).expect("context is JSON"));
        if context_sha256 != self.context_sha256 {
            return Err(Error::invalid(
                "handoff validation receipt",
                "context_sha256 does not match the embedded context",
            ));
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation: Option<HandoffValidationReceipt>,
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
        if self.schema_version == 2
            && self
                .context
                .workspace_id
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(Error::invalid(
                "handoff",
                "version 2 requires a non-empty immutable workspace identity",
            ));
        }
        if self.schema_version == 2 {
            self.validation
                .as_ref()
                .ok_or_else(|| {
                    Error::invalid("handoff", "version 2 requires a validation receipt")
                })?
                .validate(&self.context)?;
        }
        validate_id(&self.id, "handoff id")?;
        validate_timestamp(&self.generated_at)?;
        self.context.validate()
    }
}
