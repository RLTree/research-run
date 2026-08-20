use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan, FORMAT_VERSION, digest};
use crate::{Error, Result};

use super::agent_integration_content::{
    InstructionAssessment, assess_instruction, compose_instruction, managed_block,
};
use super::agent_integration_publication::publish_instruction;
use super::agent_integration_types::{InstructionFile, MAX_INSTRUCTION_BYTES};
use super::path_safety::reject_symlink_chain;
use super::storage::{parse_json, read_bounded_with_limit};
use super::write_lock::WorkspaceWriteLock;
use super::{
    AgentIntegrationApplyResult, AgentIntegrationStatus, CONTRIBUTION_PROTOCOL_DIRECTORY, Snapshot,
    Workspace,
};

impl Workspace {
    pub fn plan_agent_integration(root: &Path) -> Result<AgentIntegrationPlan> {
        let workspace = Self::for_recovery(root)?;
        let snapshot = workspace.load_snapshot()?;
        workspace.build_agent_integration_plan(&snapshot)
    }

    pub fn read_agent_integration_plan(path: &Path) -> Result<AgentIntegrationPlan> {
        let bytes = read_bounded_with_limit(path, MAX_INSTRUCTION_BYTES)?;
        let plan: AgentIntegrationPlan = parse_json(&bytes, path)?;
        plan.validate()?;
        Ok(plan)
    }

    pub fn apply_agent_integration(
        root: &Path,
        plan: AgentIntegrationPlan,
    ) -> Result<AgentIntegrationApplyResult> {
        plan.validate()?;
        let workspace = Self::for_recovery(root)?;
        let _write_lock = WorkspaceWriteLock::acquire(&workspace.state)?;
        let snapshot = workspace.load_snapshot()?;
        let current = workspace.build_agent_integration_plan(&snapshot)?;
        if current == plan {
            inject_instruction_drift(&workspace.root.join(&plan.instruction_path));
            let instruction =
                read_optional_instruction(&workspace.root.join(&plan.instruction_path))?;
            let content =
                compose_instruction(instruction.bytes(), &plan.managed_block, plan.operation)?;
            if digest(&content) != plan.prospective_sha256 {
                return Err(Error::Conflict(
                    "agent integration prospective content drifted before apply".to_owned(),
                ));
            }
            let changed = publish_instruction(
                &workspace.root.join(&plan.instruction_path),
                &content,
                plan.operation,
            )?;
            return Ok(apply_result(&plan, changed));
        }
        if identical_retry(&plan, &current) {
            return Ok(apply_result(&plan, false));
        }
        Err(Error::Conflict(
            "agent integration plan drifted; generate a new plan before apply".to_owned(),
        ))
    }

    pub fn agent_integration_status(&self) -> Result<AgentIntegrationStatus> {
        let snapshot = self.load_snapshot()?;
        self.agent_integration_status_from_snapshot(&snapshot)
    }

    pub fn agent_integration_onboarding_status(&self) -> AgentIntegrationStatus {
        self.agent_integration_status().unwrap_or_else(|error| {
            AgentIntegrationStatus::unavailable(
                format!("Agent integration inspection failed: {error}"),
                true,
            )
        })
    }

    pub(super) fn agent_integration_status_from_snapshot(
        &self,
        snapshot: &Snapshot,
    ) -> Result<AgentIntegrationStatus> {
        let path = self.active_instruction_path()?;
        let (manifest_sha, protocol_sha) = self.authority_digests()?;
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
            ready_scope: "new-agent-run",
            instruction_path: relative_name(&path),
            current_session_loaded: "unverified",
            fresh_session_required: installed,
            diagnostic,
        })
    }

    fn build_agent_integration_plan(&self, snapshot: &Snapshot) -> Result<AgentIntegrationPlan> {
        let path = self.active_instruction_path()?;
        let (manifest_sha, protocol_sha) = self.authority_digests()?;
        let block = managed_block(snapshot, &manifest_sha, &protocol_sha);
        let current = read_optional_instruction(&path)?;
        let assessment = assess_instruction(current.bytes(), current.exists(), &block)?;
        let operation = match assessment {
            InstructionAssessment::Missing => AgentIntegrationOperation::Create,
            InstructionAssessment::Unmanaged => AgentIntegrationOperation::Append,
            InstructionAssessment::Integrated => AgentIntegrationOperation::NoOp,
            InstructionAssessment::Conflict(reason) => return Err(Error::Conflict(reason)),
        };
        let prospective = compose_instruction(current.bytes(), &block, operation)?;
        Ok(AgentIntegrationPlan {
            schema_version: FORMAT_VERSION,
            kind: "agent-integration-plan".to_owned(),
            project_id: snapshot.manifest.project_id.clone(),
            workspace_id: snapshot.manifest.workspace_id.clone(),
            manifest_sha256: manifest_sha,
            protocol_sha256: protocol_sha,
            instruction_path: relative_name(&path),
            instruction_sha256: current.exists().then(|| digest(current.bytes())),
            instruction_bytes: current.bytes().len() as u64,
            managed_block_sha256: digest(block.as_bytes()),
            managed_block: block,
            prospective_sha256: digest(&prospective),
            operation,
            plan_sha256: String::new(),
        }
        .seal())
    }

    fn active_instruction_path(&self) -> Result<PathBuf> {
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

    fn authority_digests(&self) -> Result<(String, String)> {
        let manifest = authority_digest(
            &self.state.join("manifest.json"),
            "read agent integration manifest digest",
        )?;
        let protocol = authority_digest(
            &self
                .state
                .join(CONTRIBUTION_PROTOCOL_DIRECTORY)
                .join("agent-contribution.json"),
            "read agent integration protocol digest",
        )?;
        Ok((manifest, protocol))
    }
}

fn read_optional_instruction(path: &Path) -> Result<InstructionFile> {
    reject_symlink_chain(path)?;
    let metadata = if super::injected_storage_failure("inspect agent instruction") {
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    } else {
        fs::symlink_metadata(path)
    };
    match metadata {
        Ok(metadata) if metadata.is_file() => {
            read_bounded_with_limit(path, MAX_INSTRUCTION_BYTES).map(InstructionFile::Present)
        }
        Ok(_) => Err(Error::invalid(
            "project instructions",
            format!("{} is not a regular file", path.display()),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(InstructionFile::Missing),
        Err(error) => Err(Error::io("inspect project instructions", path, error)),
    }
}

fn authority_digest(path: &Path, injection_point: &'static str) -> Result<String> {
    if super::injected_storage_failure(injection_point) {
        return Err(Error::io(
            injection_point,
            path,
            std::io::Error::other("injected storage failure"),
        ));
    }
    read_bounded_with_limit(path, MAX_INSTRUCTION_BYTES).map(|bytes| digest(&bytes))
}

#[cfg(any(test, coverage))]
fn inject_instruction_drift(path: &Path) {
    if super::take_storage_failure("agent instruction drift after plan") {
        fs::write(path, b"injected instruction drift").expect("write injected instruction drift");
    }
}

#[cfg(not(any(test, coverage)))]
fn inject_instruction_drift(_path: &Path) {}

fn identical_retry(previous: &AgentIntegrationPlan, current: &AgentIntegrationPlan) -> bool {
    current.operation == AgentIntegrationOperation::NoOp
        && previous.project_id == current.project_id
        && previous.workspace_id == current.workspace_id
        && previous.manifest_sha256 == current.manifest_sha256
        && previous.protocol_sha256 == current.protocol_sha256
        && previous.instruction_path == current.instruction_path
        && previous.managed_block_sha256 == current.managed_block_sha256
        && previous.prospective_sha256 == current.prospective_sha256
}

fn relative_name(path: &Path) -> String {
    path.file_name()
        .expect("active instruction paths have a filename")
        .to_string_lossy()
        .into_owned()
}

fn apply_result(plan: &AgentIntegrationPlan, changed: bool) -> AgentIntegrationApplyResult {
    AgentIntegrationApplyResult {
        kind: "agent-integration-apply",
        plan_sha256: plan.plan_sha256.clone(),
        instruction_path: plan.instruction_path.clone(),
        changed,
        protocol_installed: true,
        instruction_contract_installed: true,
        ready_for_new_agent_run: true,
        current_session_loaded: "unverified",
        fresh_session_required: true,
        next_action: "Start a fresh agent run/session before claiming the project instruction contract was loaded.",
    }
}
