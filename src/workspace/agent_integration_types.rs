use serde::Serialize;

pub(super) const MAX_INSTRUCTION_BYTES: u64 = 1_048_576;

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

#[derive(Debug, Clone, Serialize)]
pub struct AgentIntegrationStatus {
    pub protocol_installed: bool,
    pub instruction_contract_installed: bool,
    pub agent_integration_ready: bool,
    pub ready_scope: &'static str,
    pub instruction_path: String,
    pub current_session_loaded: &'static str,
    pub fresh_session_required: bool,
    pub diagnostic: String,
}

impl AgentIntegrationStatus {
    pub(super) fn unavailable(diagnostic: String, protocol_installed: bool) -> Self {
        Self {
            protocol_installed,
            instruction_contract_installed: false,
            agent_integration_ready: false,
            ready_scope: "unavailable",
            instruction_path: "unavailable".to_owned(),
            current_session_loaded: "unverified",
            fresh_session_required: false,
            diagnostic,
        }
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
