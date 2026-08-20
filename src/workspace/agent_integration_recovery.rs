use std::path::Path;

use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan, digest};
use crate::{Error, Result};

use super::agent_integration::{read_optional_instruction, relative_name};
use super::agent_integration_content::{compose_planned_instruction, managed_block};
use super::agent_integration_publication::ensure_instruction_budget;
use super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::storage::read_bounded_with_limit;
use super::{CONTRIBUTION_PROTOCOL_DIRECTORY, Snapshot, Workspace};

impl Workspace {
    pub(super) fn agent_integration_authority_digests(&self) -> Result<(String, String)> {
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

    pub(super) fn agent_integration_plan_has_current_authority(
        &self,
        snapshot: &Snapshot,
        path: &Path,
        plan: &AgentIntegrationPlan,
    ) -> Result<bool> {
        let (manifest_sha, protocol_sha) = self.agent_integration_authority_digests()?;
        let block = managed_block(snapshot, &manifest_sha, &protocol_sha);
        Ok(plan.project_id == snapshot.manifest.project_id
            && plan.workspace_id == snapshot.manifest.workspace_id
            && plan.manifest_sha256 == manifest_sha
            && plan.protocol_sha256 == protocol_sha
            && plan.instruction_path == relative_name(path)
            && plan.managed_block == block
            && plan.managed_block_sha256 == digest(block.as_bytes()))
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

pub(super) fn identical_retry(
    previous: &AgentIntegrationPlan,
    current: &AgentIntegrationPlan,
    instruction_path: &Path,
) -> Result<bool> {
    if current.operation != AgentIntegrationOperation::NoOp
        || !matches!(
            previous.operation,
            AgentIntegrationOperation::Create | AgentIntegrationOperation::Append
        )
        || previous.project_id != current.project_id
    {
        return Ok(false);
    }
    if previous.workspace_id != current.workspace_id
        || previous.manifest_sha256 != current.manifest_sha256
        || previous.protocol_sha256 != current.protocol_sha256
        || previous.instruction_path != current.instruction_path
        || previous.managed_block != current.managed_block
        || previous.managed_block_sha256 != current.managed_block_sha256
        || previous.prospective_sha256 != current.prospective_sha256
    {
        return Ok(false);
    }
    let installed = read_optional_instruction(instruction_path)?;
    if !installed.exists()
        || current.instruction_bytes != installed.bytes().len() as u64
        || current.instruction_sha256.as_deref() != Some(digest(installed.bytes()).as_str())
        || current.prospective_sha256 != digest(installed.bytes())
    {
        return Ok(false);
    }
    plan_transition_matches(previous, installed.bytes())
}

pub(super) fn plan_transition_matches(
    plan: &AgentIntegrationPlan,
    installed: &[u8],
) -> Result<bool> {
    ensure_instruction_budget(installed)?;
    if plan.operation == AgentIntegrationOperation::NoOp
        || digest(installed) != plan.prospective_sha256
    {
        return Ok(false);
    }
    let original = match plan.operation {
        AgentIntegrationOperation::Create
            if plan.instruction_sha256.is_none() && plan.instruction_bytes == 0 =>
        {
            &[][..]
        }
        AgentIntegrationOperation::Append => {
            let Ok(length) = usize::try_from(plan.instruction_bytes) else {
                return Ok(false);
            };
            let Some(original) = installed.get(..length) else {
                return Ok(false);
            };
            if plan.instruction_sha256.as_deref() != Some(digest(original).as_str()) {
                return Ok(false);
            }
            original
        }
        _ => return Ok(false),
    };
    Ok(compose_planned_instruction(original, &plan.managed_block, plan.operation)? == installed)
}
