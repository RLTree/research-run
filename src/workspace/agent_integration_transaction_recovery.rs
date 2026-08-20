use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::domain::{AgentIntegrationPlan, digest};
use crate::{Error, Result};

use super::agent_integration_recovery::plan_transition_matches;
use super::agent_integration_transaction::{
    ambiguous, cleanup_after_publication, link_planned, restore_source,
};
use super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::path_safety::reject_symlink_chain;
use super::storage::{map_io, read_bounded_with_limit, sync_directory};

pub(super) fn recover_transaction(
    transaction: &Path,
    target: &Path,
    plan: &AgentIntegrationPlan,
) -> Result<()> {
    let (planned, source) = inspect_transaction(transaction)?;
    let target_bytes = read_optional(target)?;
    let planned_bytes = planned
        .as_ref()
        .map(|path| read_bounded_with_limit(path, MAX_INSTRUCTION_BYTES))
        .transpose()?;
    if let Some(bytes) = planned_bytes.as_deref()
        && !plan_transition_matches(plan, bytes)?
    {
        return Err(Error::Conflict(format!(
            "interrupted project instruction content does not match the reviewed plan: {}",
            transaction.display()
        )));
    }
    let source_bytes = source
        .as_ref()
        .map(|path| read_bounded_with_limit(path, MAX_INSTRUCTION_BYTES))
        .transpose()?;
    recover_state(
        transaction,
        target,
        plan,
        target_bytes.as_deref(),
        planned_bytes.as_deref(),
        source_bytes.as_deref(),
    )
}

fn recover_state(
    transaction: &Path,
    target: &Path,
    plan: &AgentIntegrationPlan,
    target_bytes: Option<&[u8]>,
    planned: Option<&[u8]>,
    source: Option<&[u8]>,
) -> Result<()> {
    let source_matches = source.is_none_or(|bytes| source_matches_plan(plan, bytes));
    let target_is_planned = planned.is_some_and(|bytes| target_bytes == Some(bytes))
        || target_bytes.is_some_and(|bytes| digest(bytes) == plan.prospective_sha256);
    if !source_matches {
        if target_bytes.is_none() {
            restore_source(target, &transaction.join("source"), transaction)?;
            return Err(Error::Conflict(
                "concurrent project instruction bytes were restored; generate a new plan"
                    .to_owned(),
            ));
        }
        return Err(Error::AmbiguousEffect(format!(
            "interrupted append contains concurrent instruction bytes at {}",
            transaction.display()
        )));
    }
    match (source, planned, target_bytes, target_is_planned) {
        (Some(_), Some(_), None, _) => {
            link_planned(target, &transaction.join("planned"))?;
            sync_directory(target.parent().expect("instructions have a parent"))
                .map_err(|error| ambiguous(target, error))?;
        }
        (Some(_), _, Some(_), true) | (None, _, Some(_), true) => {}
        (Some(_), None, None, _) => {
            restore_source(target, &transaction.join("source"), transaction)?;
            return Err(Error::Conflict(
                "interrupted append was rolled back; retry the reviewed plan".to_owned(),
            ));
        }
        (Some(preserved), None, Some(current), false) if preserved == current => {}
        (None, Some(_), Some(current), false) if source_matches_plan(plan, current) => {}
        (None, None, Some(current), false) if source_matches_plan(plan, current) => {}
        (None, Some(_), None, _) | (None, None, None, _) => {}
        _ => {
            return Err(Error::AmbiguousEffect(format!(
                "project instruction state conflicts with interrupted publication at {}",
                transaction.display()
            )));
        }
    }
    cleanup_after_publication(target, transaction)
}

fn inspect_transaction(transaction: &Path) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
    reject_symlink_chain(transaction)?;
    let metadata = map_io(
        fs::symlink_metadata(transaction),
        "inspect project instruction transaction",
        transaction,
    )?;
    if !metadata.is_dir() {
        return Err(Error::invalid(
            "project instruction transaction",
            format!("{} is not a directory", transaction.display()),
        ));
    }
    let mut planned = None;
    let mut source = None;
    for entry in map_io(
        fs::read_dir(transaction),
        "inspect project instruction transaction",
        transaction,
    )? {
        let entry = map_io(
            entry,
            "inspect project instruction transaction",
            transaction,
        )?;
        let path = entry.path();
        reject_symlink_chain(&path)?;
        let file = map_io(
            fs::symlink_metadata(&path),
            "inspect transaction file",
            &path,
        )?;
        if !file.is_file() {
            return Err(Error::invalid(
                "project instruction transaction",
                format!("{} is not a regular file", path.display()),
            ));
        }
        match entry.file_name().to_str() {
            Some("planned") if planned.is_none() => planned = Some(path),
            Some("source") if source.is_none() => source = Some(path),
            _ => {
                return Err(Error::AmbiguousEffect(format!(
                    "unexpected project instruction transaction content at {}",
                    path.display()
                )));
            }
        }
    }
    Ok((planned, source))
}

fn read_optional(target: &Path) -> Result<Option<Vec<u8>>> {
    reject_symlink_chain(target)?;
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.is_file() => {
            read_bounded_with_limit(target, MAX_INSTRUCTION_BYTES).map(Some)
        }
        Ok(_) => Err(Error::invalid(
            "project instructions",
            format!("{} is not a regular file", target.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Error::io("inspect project instructions", target, error)),
    }
}

fn source_matches_plan(plan: &AgentIntegrationPlan, bytes: &[u8]) -> bool {
    bytes.len() as u64 == plan.instruction_bytes
        && plan.instruction_sha256.as_deref() == Some(digest(bytes).as_str())
}
