use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::{ContributionProtocol, MAX_LIST_ITEMS, digest, is_lowercase_sha256};

use super::{
    AgentIntegrationStatus, agent_integration_types::validation_receipt_text_matches_schema,
    publication::canonical_json_bytes,
};

pub(super) const MAX_VALIDATION_ERRORS: usize = MAX_LIST_ITEMS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ValidationAuthority {
    pub project_id: String,
    pub workspace_id: String,
    pub manifest_sha256: String,
    pub protocol_sha256: String,
    pub instruction_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ValidationResult {
    pub schema_version: u32,
    pub authority: Option<ValidationAuthority>,
    pub valid: bool,
    pub errors: Vec<String>,
    pub counts: BTreeMap<String, usize>,
    pub agent_integration: AgentIntegrationStatus,
}

pub(super) fn bound_validation_errors(mut errors: Vec<String>) -> Vec<String> {
    if errors.len() > MAX_VALIDATION_ERRORS {
        let omitted = errors.len() - (MAX_VALIDATION_ERRORS - 1);
        errors.truncate(MAX_VALIDATION_ERRORS - 1);
        errors.push(format!(
            "validation error budget exceeded: {omitted} additional diagnostics omitted"
        ));
    }
    errors
}

impl ValidationResult {
    pub(super) fn is_handoff_receipt_for(
        &self,
        project_id: &str,
        workspace_id: Option<&str>,
        contribution_protocol: Option<&ContributionProtocol>,
    ) -> bool {
        let Some(authority) = &self.authority else {
            return false;
        };
        let Some(contribution_protocol) = contribution_protocol else {
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
            && self.errors.len() <= MAX_VALIDATION_ERRORS
            && self
                .errors
                .iter()
                .all(|error| validation_receipt_text_matches_schema(error))
            && authority.project_id == project_id
            && workspace_id == Some(authority.workspace_id.as_str())
            && (authority.workspace_id.is_empty() || is_lowercase_sha256(&authority.workspace_id))
            && is_lowercase_sha256(&authority.manifest_sha256)
            && authority.protocol_sha256 == digest(&canonical_json_bytes(contribution_protocol))
            && authority
                .instruction_sha256
                .as_deref()
                .is_none_or(is_lowercase_sha256)
            && self.counts.len() == count_keys.len()
            && count_keys.iter().all(|key| self.counts.contains_key(*key))
            && self.counts["contribution-protocol"] == 1
            && self.counts.values().all(|count| *count <= 10_000)
            && self.agent_integration.is_consistent()
            && (!self.agent_integration.agent_integration_ready
                || authority.instruction_sha256.is_some())
    }
}
