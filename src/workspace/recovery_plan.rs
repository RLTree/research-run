use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

use super::path_safety::{interrupted_target_name, reject_symlink_chain};
use super::recovery::{RecordRecoveryKind, RecoveryKind};
use super::storage::{ReadBudget, map_io, read_bounded, read_bounded_with_budget, sync_directory};
use super::{MAX_RECORDS_PER_KIND, RecoveryResult, Workspace, injected_storage_failure};

struct PendingRecord {
    path: PathBuf,
    target: PathBuf,
    bytes: Vec<u8>,
}

impl Workspace {
    pub(super) fn recover_directory(
        &self,
        directory: &Path,
        kind: RecoveryKind,
        result: &mut RecoveryResult,
    ) -> Result<()> {
        let pending = collect_pending_records(directory)?;
        enforce_recovery_budget(self, directory, &pending)?;
        self.preflight_recovery(kind, &pending)?;
        commit_recovery(directory, pending, result)
    }

    fn preflight_recovery(&self, kind: RecoveryKind, pending: &[PendingRecord]) -> Result<()> {
        match kind {
            RecoveryKind::Review => {
                let authority = self.review_recovery_authority()?;
                let mut validate = |record: &PendingRecord| {
                    self.validate_pending_review(&record.target, &record.bytes, &authority)
                };
                self.preflight_records(pending, &mut validate)
            }
            RecoveryKind::Manifest => {
                self.preflight_non_review(RecordRecoveryKind::Manifest, pending)
            }
            RecoveryKind::Source => self.preflight_non_review(RecordRecoveryKind::Source, pending),
            RecoveryKind::Claim => self.preflight_non_review(RecordRecoveryKind::Claim, pending),
            RecoveryKind::Experiment => {
                self.preflight_non_review(RecordRecoveryKind::Experiment, pending)
            }
            RecoveryKind::Evidence => {
                self.preflight_non_review(RecordRecoveryKind::Evidence, pending)
            }
        }
    }

    fn preflight_non_review(
        &self,
        kind: RecordRecoveryKind,
        pending: &[PendingRecord],
    ) -> Result<()> {
        let mut validate = |record: &PendingRecord| {
            self.validate_pending_record(kind, &record.target, &record.bytes)
        };
        self.preflight_records(pending, &mut validate)
    }

    fn preflight_records(
        &self,
        pending: &[PendingRecord],
        validate: &mut dyn FnMut(&PendingRecord) -> Result<Option<String>>,
    ) -> Result<()> {
        let mut content_by_target: BTreeMap<&Path, usize> = BTreeMap::new();
        let mut pending_review_claims = BTreeSet::new();
        for (index, record) in pending.iter().enumerate() {
            if record.target.exists() && read_bounded(&record.target)? == record.bytes {
                content_by_target.insert(&record.target, index);
                continue;
            }
            let review_claim = validate(record)?;
            if let Some(existing) = content_by_target.get(record.target.as_path())
                && pending[*existing].bytes != record.bytes
            {
                return Err(Error::AmbiguousEffect(format!(
                    "conflicting pending publications target {}; inspect them before recovery",
                    record.target.display()
                )));
            }
            content_by_target.insert(&record.target, index);
            if let Some(review_claim) = review_claim
                && !pending_review_claims.insert(review_claim.clone())
            {
                return Err(Error::AmbiguousEffect(format!(
                    "multiple pending review decisions target claim {}; inspect them before recovery",
                    review_claim
                )));
            }
        }
        Ok(())
    }
}

// The explicit symlink error arm is a distinct recovery authority boundary and
// must remain independently visible to the authoritative region-coverage gate.
#[allow(clippy::question_mark)]
fn collect_pending_records(directory: &Path) -> Result<Vec<PendingRecord>> {
    reject_symlink_chain(directory)?;
    inject_pending_symlink_race(directory);
    let mut pending = Vec::new();
    let mut budget = ReadBudget::default();
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
        if let Err(error) = reject_symlink_chain(&path) {
            return Err(error);
        }
        pending.push(PendingRecord {
            target: directory.join(target_name),
            bytes: read_bounded_with_budget(&path, &mut budget)?,
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

fn commit_recovery(
    directory: &Path,
    pending: Vec<PendingRecord>,
    result: &mut RecoveryResult,
) -> Result<()> {
    for record in pending {
        if record.target.exists() {
            discard_identical(record, result)?;
        } else {
            publish_pending(directory, record, result)?;
        }
    }
    Ok(())
}

fn discard_identical(record: PendingRecord, result: &mut RecoveryResult) -> Result<()> {
    if read_bounded(&record.target)? != record.bytes
        || injected_storage_failure("recovery target conflict")
    {
        return Err(Error::AmbiguousEffect(format!(
            "recovery conflict for {}; inspect both files",
            record.target.display()
        )));
    }
    if injected_storage_failure("publish recovered record") {
        return Err(Error::io(
            "publish recovered record",
            &record.target,
            io::Error::other("injected storage failure"),
        ));
    }
    map_io(
        fs::remove_file(&record.path),
        "remove identical pending file",
        &record.path,
    )?;
    result
        .discarded_identical
        .push(record.target.display().to_string());
    Ok(())
}

fn publish_pending(
    directory: &Path,
    record: PendingRecord,
    result: &mut RecoveryResult,
) -> Result<()> {
    map_io(
        fs::hard_link(&record.path, &record.target),
        "publish recovered record",
        &record.target,
    )?;
    sync_directory(directory)?;
    map_io(
        fs::remove_file(&record.path),
        "remove recovered pending file",
        &record.path,
    )?;
    sync_directory(directory)?;
    result.recovered.push(record.target.display().to_string());
    Ok(())
}
