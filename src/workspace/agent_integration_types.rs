use serde::{Deserialize, Serialize};

pub(super) const MAX_INSTRUCTION_BYTES: u64 = 1_048_576;
const MAX_VALIDATION_RECEIPT_TEXT_CHARACTERS: usize = 65_536;
const INSPECTION_UNAVAILABLE_DIAGNOSTIC: &str = "Agent integration inspection is unavailable. Run 'research-run agent-integration status' locally for details and preserve any pending publication evidence.";

// Draft 2020-12 string lengths count decoded characters. For serde-decoded
// strings, a bounded Rust scalar-value count matches the executable schema.
pub(super) fn validation_receipt_text_matches_schema(value: &str) -> bool {
    matches!(
        value
            .chars()
            .take(MAX_VALIDATION_RECEIPT_TEXT_CHARACTERS + 1)
            .count(),
        1..=MAX_VALIDATION_RECEIPT_TEXT_CHARACTERS
    )
}

pub(super) enum InstructionFile {
    Missing,
    Present(Vec<u8>),
}

impl InstructionFile {
    pub(super) fn bytes(&self) -> &[u8] {
        match self {
            Self::Missing => &[],
            Self::Present(bytes) => bytes,
        }
    }

    pub(super) fn exists(&self) -> bool {
        matches!(self, Self::Present(_))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentIntegrationStatus {
    pub protocol_installed: bool,
    pub instruction_contract_installed: bool,
    pub agent_integration_ready: bool,
    pub ready_scope: String,
    pub instruction_path: String,
    pub current_session_loaded: String,
    pub fresh_session_required: bool,
    pub diagnostic: String,
}

impl AgentIntegrationStatus {
    pub(super) fn inspection_unavailable(protocol_installed: bool) -> Self {
        Self {
            protocol_installed,
            instruction_contract_installed: false,
            agent_integration_ready: false,
            ready_scope: "unavailable".to_owned(),
            instruction_path: "unavailable".to_owned(),
            current_session_loaded: "unverified".to_owned(),
            fresh_session_required: false,
            diagnostic: INSPECTION_UNAVAILABLE_DIAGNOSTIC.to_owned(),
        }
    }

    pub fn is_consistent(&self) -> bool {
        if self.current_session_loaded != "unverified"
            || !validation_receipt_text_matches_schema(&self.diagnostic)
        {
            return false;
        }
        if self.agent_integration_ready {
            return self.protocol_installed
                && self.instruction_contract_installed
                && self.ready_scope == "new-agent-run"
                && matches!(
                    self.instruction_path.as_str(),
                    "AGENTS.md" | "AGENTS.override.md"
                )
                && self.fresh_session_required;
        }
        if self.ready_scope == "unavailable" {
            return !self.instruction_contract_installed
                && self.instruction_path == "unavailable"
                && !self.fresh_session_required;
        }
        self.protocol_installed
            && self.ready_scope == "new-agent-run"
            && !self.instruction_contract_installed
            && matches!(
                self.instruction_path.as_str(),
                "AGENTS.md" | "AGENTS.override.md"
            )
            && !self.fresh_session_required
    }
}

#[derive(Debug, Serialize)]
pub struct AgentIntegrationApplyResult {
    pub kind: &'static str,
    pub plan_sha256: String,
    pub instruction_path: String,
    pub changed: bool,
    pub protocol_installed: bool,
    pub instruction_contract_installed: bool,
    pub ready_for_new_agent_run: bool,
    pub current_session_loaded: &'static str,
    pub fresh_session_required: bool,
    pub next_action: &'static str,
}
