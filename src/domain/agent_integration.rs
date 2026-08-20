use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Error, Result};

use super::validation::{required_text, validate_header, validate_hex_digest, validate_id};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AgentIntegrationOperation {
    Create,
    Append,
    NoOp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentIntegrationPlan {
    pub schema_version: u32,
    pub kind: String,
    pub project_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub workspace_id: String,
    pub manifest_sha256: String,
    pub protocol_sha256: String,
    pub instruction_path: String,
    pub instruction_sha256: Option<String>,
    pub instruction_bytes: u64,
    pub managed_block: String,
    pub managed_block_sha256: String,
    pub prospective_sha256: String,
    pub operation: AgentIntegrationOperation,
    pub plan_sha256: String,
}

impl AgentIntegrationPlan {
    pub fn seal(mut self) -> Self {
        self.plan_sha256.clear();
        self.plan_sha256 = digest(&serde_json::to_vec(&self).expect("plan is JSON representable"));
        self
    }

    pub fn validate(&self) -> Result<()> {
        validate_header(self.schema_version, &self.kind, "agent-integration-plan")?;
        validate_id(&self.project_id, "agent integration project_id")?;
        if !self.workspace_id.is_empty() {
            validate_hex_digest(&self.workspace_id, "agent integration workspace_id")?;
        }
        validate_hex_digest(&self.manifest_sha256, "agent integration manifest digest")?;
        validate_hex_digest(&self.protocol_sha256, "agent integration protocol digest")?;
        if !matches!(
            self.instruction_path.as_str(),
            "AGENTS.md" | "AGENTS.override.md"
        ) {
            return Err(Error::invalid(
                "agent integration instruction_path",
                "must be the active root AGENTS.md or AGENTS.override.md",
            ));
        }
        if let Some(digest) = &self.instruction_sha256 {
            validate_hex_digest(digest, "agent integration instruction digest")?;
        }
        match self.operation {
            AgentIntegrationOperation::Create if self.instruction_sha256.is_some() => {
                return Err(Error::invalid(
                    "agent integration operation",
                    "create requires a missing instruction file",
                ));
            }
            AgentIntegrationOperation::Append | AgentIntegrationOperation::NoOp
                if self.instruction_sha256.is_none() =>
            {
                return Err(Error::invalid(
                    "agent integration operation",
                    "append and no-op require an existing instruction file",
                ));
            }
            _ => {}
        }
        if self.operation == AgentIntegrationOperation::Create && self.instruction_bytes != 0 {
            return Err(Error::invalid(
                "agent integration instruction identity",
                "a missing instruction file must have zero bytes",
            ));
        }
        required_text(&self.managed_block, "agent integration managed block")?;
        validate_hex_digest(
            &self.managed_block_sha256,
            "agent integration managed block digest",
        )?;
        validate_hex_digest(
            &self.prospective_sha256,
            "agent integration prospective digest",
        )?;
        validate_hex_digest(&self.plan_sha256, "agent integration plan digest")?;
        if digest(self.managed_block.as_bytes()) != self.managed_block_sha256 {
            return Err(Error::invalid(
                "agent integration managed block",
                "digest does not match content",
            ));
        }
        let mut unsealed = self.clone();
        unsealed.plan_sha256.clear();
        if digest(&serde_json::to_vec(&unsealed).expect("plan is JSON representable"))
            != self.plan_sha256
        {
            return Err(Error::invalid(
                "agent integration plan",
                "plan_sha256 does not match the plan",
            ));
        }
        Ok(())
    }
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
