use crate::domain::{AgentIntegrationOperation, digest};
use crate::{Error, Result};

use super::Snapshot;

const START_MARKER: &str = "<!-- research-run:agent-integration:v1:start -->";
const END_MARKER: &str = "<!-- research-run:agent-integration:v1:end -->";
const INSTALLATION_MARKER: &str = "<!-- research-run:agent-integration-installation:v1 ";

pub(super) enum InstructionAssessment {
    Missing,
    Unmanaged,
    Integrated,
    Conflict(String),
}

pub(super) fn assess_instruction(
    bytes: &[u8],
    exists: bool,
    expected: &str,
) -> Result<InstructionAssessment> {
    if bytes.is_empty() {
        return Ok(if exists {
            InstructionAssessment::Unmanaged
        } else {
            InstructionAssessment::Missing
        });
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid("project instructions", "must be UTF-8"))?;
    let starts = text.matches(START_MARKER).count();
    let ends = text.matches(END_MARKER).count();
    if starts == 0 && ends == 0 {
        return Ok(InstructionAssessment::Unmanaged);
    }
    if starts != 1 || ends != 1 {
        return Ok(InstructionAssessment::Conflict(
            "Research Run managed instruction markers are missing or duplicated".to_owned(),
        ));
    }
    if text.contains(expected) && installed_transition_matches(bytes, expected)? {
        Ok(InstructionAssessment::Integrated)
    } else {
        Ok(InstructionAssessment::Conflict(
            "Research Run managed instruction content conflicts with the canonical workspace binding: content drifted or the installation transition is incomplete".to_owned(),
        ))
    }
}

pub(super) fn compose_instruction(
    current: &[u8],
    block: &str,
    operation: AgentIntegrationOperation,
) -> Result<Vec<u8>> {
    if operation == AgentIntegrationOperation::NoOp {
        return Ok(current.to_vec());
    }
    std::str::from_utf8(current)
        .map_err(|_| Error::invalid("project instructions", "must be UTF-8"))?;
    let mut content = current.to_vec();
    if !content.is_empty() {
        if !content.ends_with(b"\n") {
            content.push(b'\n');
        }
        content.push(b'\n');
    }
    content.extend_from_slice(block.as_bytes());
    Ok(content)
}

pub(super) fn compose_planned_instruction(
    current: &[u8],
    block: &str,
    operation: AgentIntegrationOperation,
) -> Result<Vec<u8>> {
    let mut content = compose_instruction(current, block, operation)?;
    if operation == AgentIntegrationOperation::NoOp {
        return Ok(content);
    }
    let original_sha256 = if operation == AgentIntegrationOperation::Create {
        "missing".to_owned()
    } else {
        digest(current)
    };
    content.extend_from_slice(
        format!(
            "<!-- research-run:agent-integration-installation:v1 operation={} instruction-bytes={} instruction-sha256={} -->\n",
            operation_name(operation),
            current.len(),
            original_sha256,
        )
        .as_bytes(),
    );
    Ok(content)
}

fn operation_name(operation: AgentIntegrationOperation) -> &'static str {
    match operation {
        AgentIntegrationOperation::Create => "create",
        AgentIntegrationOperation::Append => "append",
        AgentIntegrationOperation::NoOp => "no-op",
    }
}

fn installed_transition_matches(content: &[u8], block: &str) -> Result<bool> {
    if compose_planned_instruction(b"", block, AgentIntegrationOperation::Create)? == content {
        return Ok(true);
    }
    let text = std::str::from_utf8(content)
        .map_err(|_| Error::invalid("project instructions", "must be UTF-8"))?;
    let Some((_, marker)) = text.rsplit_once(INSTALLATION_MARKER) else {
        return Ok(false);
    };
    let Some(fields) = marker.strip_suffix(" -->\n") else {
        return Ok(false);
    };
    let mut fields = fields.split(' ');
    if fields.next() != Some("operation=append") {
        return Ok(false);
    }
    let Some(length) = fields
        .next()
        .and_then(|field| field.strip_prefix("instruction-bytes="))
        .and_then(|value| value.parse::<usize>().ok())
    else {
        return Ok(false);
    };
    let Some(expected_digest) = fields
        .next()
        .and_then(|field| field.strip_prefix("instruction-sha256="))
    else {
        return Ok(false);
    };
    if fields.next().is_some() {
        return Ok(false);
    }
    let Some(original) = content.get(..length) else {
        return Ok(false);
    };
    Ok(digest(original) == expected_digest
        && compose_planned_instruction(original, block, AgentIntegrationOperation::Append)?
            == content)
}

pub(super) fn managed_block(snapshot: &Snapshot, manifest_sha: &str, protocol_sha: &str) -> String {
    format!(
        "{START_MARKER}\n## Research Run contribution contract (managed)\n\nBinding: project `{}`, workspace `{}`, manifest SHA-256 `{manifest_sha}`, protocol SHA-256 `{protocol_sha}`.\n\n- Before material work, retrieve bounded context with `research-run context --query <scope>` and classify whether new material exists.\n- Record new human observations, corrections, and decisions with the supported typed Research Run CLI using human authorship. Record AI analyses with AI authorship. If there is no new material, do not write.\n- Never create or edit canonical `.research-run` files with generic filesystem tools.\n- Run `research-run validate --json` before handoff. Claim validation only when that actual CLI invocation produced a successful receipt.\n- Complete material work with a Research Run v2 handoff containing bounded context and the contribution protocol. Preserve v1 handoff inspection compatibility.\n- Only an explicit signed human `ReviewDecision` may promote a claim assessment.\n- After this managed block changes, start a fresh agent run/session before claiming the instruction contract was loaded.\n{END_MARKER}\n",
        snapshot.manifest.project_id,
        if snapshot.manifest.workspace_id.is_empty() {
            "legacy-unanchored"
        } else {
            snapshot.manifest.workspace_id.as_str()
        }
    )
}
