use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

use super::path_safety::{interrupted_target_name, reject_symlink_chain};
use super::storage::{ReadBudget, map_io, read_bounded_with_budget_and_limit};
use super::{MAX_RECORDS_PER_KIND, Workspace, injected_storage_failure, record_byte_limit};

pub(super) struct PendingRecord {
    pub(super) path: PathBuf,
    pub(super) target: PathBuf,
    pub(super) bytes: Vec<u8>,
}

impl Workspace {
    pub(super) fn collect_recovery_pending(&self, directory: &Path) -> Result<Vec<PendingRecord>> {
        let pending = collect_pending_records(directory)?;
        enforce_recovery_budget(self, directory, &pending)?;
        Ok(pending)
    }
}

fn collect_pending_records(directory: &Path) -> Result<Vec<PendingRecord>> {
    reject_symlink_chain(directory)?;
    inject_pending_symlink_race(directory);
    let mut pending = Vec::new();
    let mut budget = ReadBudget::default();
    let maximum = directory
        .file_name()
        .and_then(OsStr::to_str)
        .map(record_byte_limit)
        .unwrap_or(super::MAX_RECORD_BYTES);
    for entry in map_io(
        fs::read_dir(directory),
        "read recovery directory",
        directory,
    )? {
        let entry = map_io(entry, "read recovery entry", directory)?;
        let path = entry.path();
        let entry_name = entry.file_name();
        let Some(target_name) = interrupted_target_name(&entry_name) else {
            continue;
        };
        reject_symlink_chain(&path)?;
        pending.push(PendingRecord {
            target: directory.join(target_name),
            bytes: read_bounded_with_budget_and_limit(&path, &mut budget, maximum)?,
            path,
        });
        if pending.len() > MAX_RECORDS_PER_KIND || injected_storage_failure("pending record count")
        {
            return Err(Error::Budget(format!(
                "{} exceeds the {MAX_RECORDS_PER_KIND} pending record budget",
                directory.display()
            )));
        }
    }
    pending.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(pending)
}

#[cfg(all(coverage, unix))]
fn inject_pending_symlink_race(directory: &Path) {
    use std::os::unix::fs::symlink;

    if std::env::var("RESEARCH_RUN_COVERAGE_FAULT").as_deref() == Ok("pending record symlink race")
    {
        let destination = directory.join("manifest.race-destination");
        fs::write(&destination, b"{}").expect("write injected pending destination");
        symlink(destination, directory.join(".manifest.json.11.1.tmp"))
            .expect("create injected pending symlink");
    }
}

#[cfg(not(all(coverage, unix)))]
fn inject_pending_symlink_race(_directory: &Path) {}

fn enforce_recovery_budget(
    workspace: &Workspace,
    directory: &Path,
    pending: &[PendingRecord],
) -> Result<()> {
    let mut canonical_count = 0;
    for entry in map_io(
        fs::read_dir(directory),
        "count recovery directory",
        directory,
    )? {
        let entry = map_io(entry, "count recovery entry", directory)?;
        canonical_count += usize::from(entry.path().extension() == Some(OsStr::new("json")));
    }
    let new_targets = pending
        .iter()
        .map(|record| &record.target)
        .filter(|target| !target.exists())
        .collect::<BTreeSet<_>>()
        .len();
    if directory != workspace.state
        && (canonical_count.saturating_add(new_targets) > MAX_RECORDS_PER_KIND
            || injected_storage_failure("recovery record count"))
    {
        return Err(Error::Budget(format!(
            "recovery would exceed the {MAX_RECORDS_PER_KIND} record budget in {}",
            directory.display()
        )));
    }
    Ok(())
}
