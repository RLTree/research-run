use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan, digest};
use crate::{Error, Result};

#[path = "agent_integration_pending.rs"]
pub(super) mod pending;

use super::agent_integration::private_staging::write_private_pending;
use super::agent_integration_recovery::plan_transition_matches;
use super::agent_integration_transaction::{cleanup::recover_completion, publish_append};
use super::agent_integration_transaction_recovery::recover_transaction;
use super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::path_safety::reject_symlink_chain;
use super::pending_cleanup::PendingCleanup;
use super::publication::pending_path;
use super::storage::{read_bounded_with_limit, sync_directory};
#[cfg(test)]
use pending::default_instruction_scan_budget;
use pending::{ensure_no_instruction_pending as ensure_no_pending, instruction_pending};

#[cfg(test)]
pub(super) fn publish_instruction(
    target: &Path,
    content: &[u8],
    operation: AgentIntegrationOperation,
    expected_source: &[u8],
    plan_sha256: &str,
) -> Result<bool> {
    publish_instruction_with_budget(
        target,
        content,
        operation,
        expected_source,
        plan_sha256,
        default_instruction_scan_budget()?,
    )
}

pub(super) fn publish_instruction_with_budget(
    target: &Path,
    content: &[u8],
    operation: AgentIntegrationOperation,
    expected_source: &[u8],
    plan_sha256: &str,
    scan_budget: usize,
) -> Result<bool> {
    ensure_instruction_budget(content)?;
    if operation == AgentIntegrationOperation::NoOp {
        return Ok(false);
    }
    reject_symlink_chain(target)?;
    ensure_no_pending(target, scan_budget)?;
    if operation == AgentIntegrationOperation::Append {
        return publish_append(target, content, expected_source, plan_sha256);
    }
    let temporary = pending_path(target);
    let mut cleanup = PendingCleanup::new(temporary.clone());
    let parent = target.parent().expect("root instruction has a parent");
    if let Err(error) = write_private_pending(parent, &temporary, content) {
        return Err(cleanup.after_failure(error));
    }
    let publication = create(target, &temporary, content);
    let changed = match publication {
        Ok(changed) => changed,
        Err(error) => return Err(cleanup.after_failure(error)),
    };
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

#[cfg(all(coverage, test))]
pub(super) fn ensure_no_instruction_pending(target: &Path) -> Result<()> {
    ensure_no_pending(target, default_instruction_scan_budget()?)
}

#[cfg(all(coverage, test))]
pub(super) fn recover_instruction_pending(
    target: &Path,
    plan: &AgentIntegrationPlan,
) -> Result<()> {
    recover_instruction_pending_with_budget(target, plan, default_instruction_scan_budget()?)
}

pub(super) fn recover_instruction_pending_with_budget(
    target: &Path,
    plan: &AgentIntegrationPlan,
    scan_budget: usize,
) -> Result<()> {
    let pending = instruction_pending(target, scan_budget).map_err(|error| {
        Error::AmbiguousEffect(format!(
            "interrupted project instruction publication could not be inspected at {}: {error}",
            target.display()
        ))
    })?;
    let Some(path) = pending.first() else {
        return Ok(());
    };
    let completions: Vec<&PathBuf> = pending
        .iter()
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "done")
        })
        .collect();
    let transactions: Vec<&PathBuf> = pending
        .iter()
        .filter(|path| path.extension().is_some_and(|extension| extension == "txn"))
        .collect();
    if completions.len() == 1
        && transactions.len() <= 1
        && pending.len() == completions.len() + transactions.len()
    {
        return recover_completion(
            completions[0],
            transactions.first().map(|path| path.as_path()),
            target,
            plan,
        );
    }
    if pending.len() != 1 || !completions.is_empty() {
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

#[cfg(all(coverage, test))]
#[path = "tests/agent_integration_publication_coverage.rs"]
mod coverage_tests;
