use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::InventoryLimits;
use crate::{Error, Result};

use super::super::Snapshot;
use super::super::path_safety::reject_symlink_chain;
use super::super::storage::map_io;

const ROOT_ENTRY_ALLOWANCE: u64 = 256;

#[cfg(test)]
pub(super) fn default_instruction_scan_budget() -> Result<usize> {
    scan_budget_from_max_entries(0)
}

pub(in crate::workspace) fn instruction_scan_budget(snapshot: &Snapshot) -> Result<usize> {
    let latest_maximum = snapshot
        .inventories
        .iter()
        .max_by(|left, right| (&left.observed_at, &left.id).cmp(&(&right.observed_at, &right.id)))
        .and_then(|inventory| inventory.policy.as_ref())
        .map(|policy| policy.limits.max_entries)
        .unwrap_or(0);
    scan_budget_from_max_entries(latest_maximum)
}

fn scan_budget_from_max_entries(latest_maximum: u64) -> Result<usize> {
    let material_entries = InventoryLimits::default().max_entries.max(latest_maximum);
    let total = material_entries
        .checked_add(ROOT_ENTRY_ALLOWANCE)
        .ok_or_else(|| {
            Error::Budget("project instruction root-entry budget overflowed".to_owned())
        })?;
    usize::try_from(total).map_err(|_| {
        Error::Budget("project instruction root-entry budget does not fit this platform".to_owned())
    })
}

pub(in crate::workspace) fn ensure_no_instruction_pending(
    target: &Path,
    budget: usize,
) -> Result<()> {
    if let Some(path) = instruction_pending(target, budget)?.first() {
        return Err(Error::AmbiguousEffect(format!(
            "interrupted project instruction publication found at {}; retry the reviewed agent integration plan",
            path.display()
        )));
    }
    Ok(())
}

pub(super) fn instruction_pending(target: &Path, budget: usize) -> Result<Vec<PathBuf>> {
    reject_symlink_chain(target)?;
    let parent = target
        .parent()
        .expect("root instruction paths have a parent");
    let target_name = target
        .file_name()
        .and_then(OsStr::to_str)
        .expect("root instruction names are UTF-8");
    let prefix = format!(".{target_name}.");
    let entries = map_io(
        fs::read_dir(parent),
        "inspect pending project instructions",
        parent,
    )?;
    let mut visited = 0usize;
    let mut pending = Vec::with_capacity(2);
    for entry in entries {
        visited = visited.checked_add(1).ok_or_else(|| {
            Error::Budget("project instruction root-entry count overflowed".to_owned())
        })?;
        if visited > budget
            || super::super::injected_storage_failure("project instruction root entry budget")
        {
            return Err(Error::Budget(format!(
                "project instruction pending scan exceeded the {budget} root-entry budget; reduce root fan-out or apply a reviewed inventory policy with a larger max_entries"
            )));
        }
        let entry = map_io(entry, "inspect pending project instruction", parent)?;
        let name = entry.file_name();
        let Some(kind) = instruction_artifact_kind(&name, &prefix) else {
            continue;
        };
        if pending.len() == 2 {
            return Err(Error::AmbiguousEffect(format!(
                "multiple interrupted project instruction publications exist for {}",
                target.display()
            )));
        }
        let path = entry.path();
        reject_symlink_chain(&path)?;
        let metadata = map_io(
            fs::symlink_metadata(&path),
            "inspect pending project instruction path",
            &path,
        )?;
        let expected_shape = match kind {
            ArtifactKind::Transaction => metadata.is_dir(),
            ArtifactKind::Pending | ArtifactKind::Completion => metadata.is_file(),
        };
        if !expected_shape {
            return Err(Error::invalid(
                "pending project instruction",
                format!("{} has an invalid transaction shape", path.display()),
            ));
        }
        pending.push(path);
    }
    pending.sort();
    Ok(pending)
}

#[derive(Clone, Copy)]
enum ArtifactKind {
    Pending,
    Transaction,
    Completion,
}

fn instruction_artifact_kind(name: &OsStr, prefix: &str) -> Option<ArtifactKind> {
    let name = name.to_str()?;
    for (suffix, kind) in [
        (".tmp", ArtifactKind::Pending),
        (".txn", ArtifactKind::Transaction),
        (".done", ArtifactKind::Completion),
    ] {
        let Some(body) = name
            .strip_prefix(prefix)
            .and_then(|name| name.strip_suffix(suffix))
        else {
            continue;
        };
        let Some((process, sequence)) = body.split_once('.') else {
            continue;
        };
        if !process.is_empty()
            && !sequence.is_empty()
            && !sequence.contains('.')
            && process.parse::<u32>().is_ok()
            && sequence.parse::<u64>().is_ok()
        {
            return Some(kind);
        }
    }
    None
}

#[cfg(test)]
#[path = "tests/agent_integration_pending.rs"]
mod tests;
