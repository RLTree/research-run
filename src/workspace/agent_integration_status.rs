use std::fs;
use std::path::PathBuf;

use crate::domain::digest;
use crate::{Error, Result};

use super::agent_integration::{read_optional_instruction, relative_name};
use super::agent_integration_content::{InstructionAssessment, assess_instruction, managed_block};
use super::agent_integration_publication::ensure_no_instruction_pending;
use super::path_safety::reject_symlink_chain;
use super::{AgentIntegrationStatus, Snapshot, ValidationAuthority, Workspace};

impl Workspace {
    pub fn agent_integration_status(&self) -> Result<AgentIntegrationStatus> {
        let snapshot = self.load_snapshot()?;
        self.agent_integration_status_from_snapshot(&snapshot)
    }

    pub fn agent_integration_onboarding_status(&self) -> AgentIntegrationStatus {
        match self.load_snapshot() {
            Ok(snapshot) => self
                .agent_integration_status_from_snapshot(&snapshot)
                .unwrap_or_else(|error| {
                    AgentIntegrationStatus::unavailable(
                        format!("Agent integration inspection failed: {error}"),
                        true,
                    )
                }),
            Err(error) => AgentIntegrationStatus::unavailable(
                format!("Agent integration inspection failed: {error}"),
                false,
            ),
        }
    }

    pub(super) fn agent_integration_status_from_snapshot(
        &self,
        snapshot: &Snapshot,
    ) -> Result<AgentIntegrationStatus> {
        let path = self.active_instruction_path()?;
        ensure_no_instruction_pending(&path)?;
        let (manifest_sha, protocol_sha) = self.agent_integration_authority_digests()?;
        let block = managed_block(snapshot, &manifest_sha, &protocol_sha);
        let bytes = read_optional_instruction(&path)?;
        let assessment = assess_instruction(bytes.bytes(), bytes.exists(), &block)?;
        let (installed, diagnostic) = match assessment {
            InstructionAssessment::Integrated => (
                true,
                "Project instruction contract is installed. Start a new agent run after any instruction change before claiming it was loaded.".to_owned(),
            ),
            InstructionAssessment::Missing => (
                false,
                "The active project instruction file is missing; run 'research-run agent-integration plan' and apply the reviewed plan.".to_owned(),
            ),
            InstructionAssessment::Unmanaged => (
                false,
                "The active project instruction file does not contain the Research Run contract; plan and apply integration without replacing existing instructions.".to_owned(),
            ),
            InstructionAssessment::Conflict(reason) => (false, reason),
        };
        Ok(AgentIntegrationStatus {
            protocol_installed: true,
            instruction_contract_installed: installed,
            agent_integration_ready: installed,
            ready_scope: "new-agent-run".to_owned(),
            instruction_path: relative_name(&path),
            current_session_loaded: "unverified".to_owned(),
            fresh_session_required: installed,
            diagnostic,
        })
    }

    pub(super) fn validation_authority(&self, snapshot: &Snapshot) -> Result<ValidationAuthority> {
        let (manifest_sha256, protocol_sha256) = self.agent_integration_authority_digests()?;
        let instruction = read_optional_instruction(&self.active_instruction_path()?)?;
        Ok(ValidationAuthority {
            project_id: snapshot.manifest.project_id.clone(),
            workspace_id: snapshot.manifest.workspace_id.clone(),
            manifest_sha256,
            protocol_sha256,
            instruction_sha256: instruction.exists().then(|| digest(instruction.bytes())),
        })
    }

    pub(super) fn active_instruction_path(&self) -> Result<PathBuf> {
        let override_path = self.root.join("AGENTS.override.md");
        let standard_path = self.root.join("AGENTS.md");
        reject_symlink_chain(&override_path)?;
        reject_symlink_chain(&standard_path)?;
        match fs::symlink_metadata(&override_path) {
            Ok(_) => Ok(override_path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(standard_path),
            Err(error) => Err(Error::io(
                "inspect project instruction override",
                &override_path,
                error,
            )),
        }
    }
}
