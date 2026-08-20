use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan, digest};
use crate::{Error, Result};

use super::agent_integration_recovery::plan_transition_matches;
use super::agent_integration_transaction::publish_append;
use super::agent_integration_transaction_recovery::recover_transaction;
use super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::path_safety::reject_symlink_chain;
use super::pending_cleanup::PendingCleanup;
use super::publication::{pending_path, write_pending};
use super::storage::{map_io, read_bounded_with_limit, sync_directory};

pub(super) fn publish_instruction(
    target: &Path,
    content: &[u8],
    operation: AgentIntegrationOperation,
    expected_source: &[u8],
) -> Result<bool> {
    ensure_instruction_budget(content)?;
    if operation == AgentIntegrationOperation::NoOp {
        return Ok(false);
    }
    reject_symlink_chain(target)?;
    ensure_no_instruction_pending(target)?;
    if operation == AgentIntegrationOperation::Append {
        return publish_append(target, content, expected_source);
    }
    let temporary = pending_path(target);
    let mut cleanup = PendingCleanup::new(temporary.clone());
    if let Err(error) = write_pending(&temporary, content) {
        return Err(cleanup.after_failure(error));
    }
    let publication = create(target, &temporary, content);
    let changed = match publication {
        Ok(changed) => changed,
        Err(error) => return Err(cleanup.after_failure(error)),
    };
    let parent = target.parent().expect("root instruction has a parent");
    sync_directory(parent).map_err(|error| {
        Error::AmbiguousEffect(format!(
            "project instructions may be installed at {}; directory sync failed: {error}",
            target.display()
        ))
    })?;
    cleanup.remove().map_err(|error| {
        Error::AmbiguousEffect(format!(
            "project instructions were published at {} but pending cleanup was not durable: {error}",
            target.display()
        ))
    })?;
    Ok(changed)
}

pub(super) fn ensure_no_instruction_pending(target: &Path) -> Result<()> {
    if let Some(path) = instruction_pending(target)?.first() {
        return Err(Error::AmbiguousEffect(format!(
            "interrupted project instruction publication found at {}; retry the reviewed agent integration plan",
            path.display()
        )));
    }
    Ok(())
}

pub(super) fn recover_instruction_pending(
    target: &Path,
    plan: &AgentIntegrationPlan,
) -> Result<()> {
    let pending = instruction_pending(target)?;
    let Some(path) = pending.first() else {
        return Ok(());
    };
    if pending.len() != 1 {
        return Err(Error::AmbiguousEffect(format!(
            "multiple interrupted project instruction publications exist for {}",
            target.display()
        )));
    }
    if path.is_dir() {
        return recover_transaction(path, target, plan);
    }
    let pending_bytes = read_bounded_with_limit(path, MAX_INSTRUCTION_BYTES)?;
    if !plan_transition_matches(plan, &pending_bytes)? {
        return Err(Error::Conflict(format!(
            "interrupted project instruction content does not match the reviewed plan: {}",
            path.display()
        )));
    }

    match read_optional_target(target)? {
        None if plan.operation == AgentIntegrationOperation::Create => {}
        Some(current) if current == pending_bytes => {}
        Some(current)
            if plan.operation == AgentIntegrationOperation::Append
                && current.len() as u64 == plan.instruction_bytes
                && plan.instruction_sha256.as_deref() == Some(digest(&current).as_str()) => {}
        _ => {
            return Err(Error::Conflict(format!(
                "project instruction state does not match the interrupted reviewed plan: {}",
                target.display()
            )));
        }
    }

    PendingCleanup::new(path.clone()).remove()
}

pub(super) fn ensure_instruction_budget(content: &[u8]) -> Result<()> {
    if content.len() as u64 > MAX_INSTRUCTION_BYTES {
        return Err(Error::Budget(format!(
            "project instructions exceed the {MAX_INSTRUCTION_BYTES} byte budget"
        )));
    }
    Ok(())
}

fn instruction_pending(target: &Path) -> Result<Vec<PathBuf>> {
    reject_symlink_chain(target)?;
    let parent = target
        .parent()
        .expect("root instruction paths have a parent");
    let target_name = target
        .file_name()
        .and_then(OsStr::to_str)
        .expect("root instruction names are UTF-8");
    let entries = map_io(
        fs::read_dir(parent),
        "inspect pending project instructions",
        parent,
    )?;
    let mut pending = Vec::new();
    for entry in entries {
        let entry = map_io(entry, "inspect pending project instruction", parent)?;
        let name = entry.file_name();
        if is_instruction_pending_name(&name, target_name)
            || is_instruction_transaction_name(&name, target_name)
        {
            let path = entry.path();
            reject_symlink_chain(&path)?;
            let metadata = map_io(
                fs::symlink_metadata(&path),
                "inspect pending project instruction path",
                &path,
            )?;
            let expected_shape = if is_instruction_transaction_name(&name, target_name) {
                metadata.is_dir()
            } else {
                metadata.is_file()
            };
            if !expected_shape {
                return Err(Error::invalid(
                    "pending project instruction",
                    format!("{} has an invalid transaction shape", path.display()),
                ));
            }
            pending.push(path);
        }
    }
    Ok(pending)
}

fn is_instruction_pending_name(name: &OsStr, target_name: &str) -> bool {
    is_instruction_artifact_name(name, target_name, ".tmp")
}

fn is_instruction_transaction_name(name: &OsStr, target_name: &str) -> bool {
    is_instruction_artifact_name(name, target_name, ".txn")
}

fn is_instruction_artifact_name(name: &OsStr, target_name: &str, suffix: &str) -> bool {
    let Some(name) = name.to_str() else {
        return false;
    };
    let Some(body) = name
        .strip_prefix(&format!(".{target_name}."))
        .and_then(|name| name.strip_suffix(suffix))
    else {
        return false;
    };
    let Some((process, sequence)) = body.split_once('.') else {
        return false;
    };
    !process.is_empty()
        && !sequence.is_empty()
        && !sequence.contains('.')
        && process.parse::<u32>().is_ok()
        && sequence.parse::<u64>().is_ok()
}

fn read_optional_target(target: &Path) -> Result<Option<Vec<u8>>> {
    reject_symlink_chain(target)?;
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.is_file() => {
            read_bounded_with_limit(target, MAX_INSTRUCTION_BYTES).map(Some)
        }
        Ok(_) => Err(Error::invalid(
            "project instructions",
            format!("{} is not a regular file", target.display()),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Error::io("inspect project instructions", target, error)),
    }
}

fn create(target: &Path, temporary: &Path, content: &[u8]) -> Result<bool> {
    let publication = if super::injected_storage_failure("create project instructions") {
        Err(std::io::Error::other("injected storage failure"))
    } else {
        fs::hard_link(temporary, target)
    };
    match publication {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_bounded_with_limit(target, MAX_INSTRUCTION_BYTES)? == content {
                Ok(false)
            } else {
                Err(Error::Conflict(format!(
                    "project instruction file appeared during apply: {}",
                    target.display()
                )))
            }
        }
        Err(error) => Err(Error::io("create project instructions", target, error)),
    }
}
