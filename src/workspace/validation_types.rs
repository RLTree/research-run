use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::AgentIntegrationStatus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationAuthority {
    pub project_id: String,
    pub workspace_id: String,
    pub manifest_sha256: String,
    pub protocol_sha256: String,
    pub instruction_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationResult {
    pub schema_version: u32,
    pub authority: Option<ValidationAuthority>,
    pub valid: bool,
    pub errors: Vec<String>,
    pub counts: BTreeMap<String, usize>,
    pub agent_integration: AgentIntegrationStatus,
}

impl ValidationResult {
    pub(super) fn is_handoff_receipt_for(
        &self,
        project_id: &str,
        workspace_id: Option<&str>,
    ) -> bool {
        let Some(authority) = &self.authority else {
            return false;
        };
        let count_keys = [
            "claim",
            "contribution-protocol",
            "evidence",
            "experiment",
            "inventory",
            "knowledge",
            "migration",
            "relationship",
            "review",
            "review-authority",
            "source",
        ];
        self.schema_version == 2
            && self.valid == self.errors.is_empty()
            && authority.project_id == project_id
            && workspace_id == Some(authority.workspace_id.as_str())
            && (authority.workspace_id.is_empty() || is_digest(&authority.workspace_id))
            && is_digest(&authority.manifest_sha256)
            && is_digest(&authority.protocol_sha256)
            && authority
                .instruction_sha256
                .as_deref()
                .is_none_or(is_digest)
            && self.counts.len() == count_keys.len()
            && count_keys.iter().all(|key| self.counts.contains_key(*key))
            && self.counts["contribution-protocol"] == 1
            && self.counts.values().all(|count| *count <= 10_000)
            && self.agent_integration.is_consistent()
            && (!self.agent_integration.agent_integration_ready
                || authority.instruction_sha256.is_some())
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
