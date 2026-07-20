use std::fs;
use std::io;
use std::path::Path;

use crate::{Error, Result};

use super::pending_cleanup::PendingCleanup;
use super::publication::{pending_path, write_pending};
use super::recovery_plan::PendingRecord;
use super::storage::{map_io, read_bounded, sync_directory};
use super::{RecoveryResult, injected_storage_failure};

pub(super) fn commit_recovery(
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

pub(super) fn discard_identical(record: PendingRecord, result: &mut RecoveryResult) -> Result<()> {
    let target = read_bounded(&record.target)?;
    let target_matches = target.iter().eq(record.bytes.iter());
    if !target_matches || injected_storage_failure("recovery target conflict") {
        return Err(Error::AmbiguousEffect(format!(
            "recovery conflict for {}; inspect both files",
            record.target.display()
        )));
    }
    fail_before_publication(&record)?;
    map_io(
        fs::remove_file(&record.path),
        "remove identical pending file",
        &record.path,
    )?;
    let parent = record
        .path
        .parent()
        .expect("pending recovery records always have a parent directory");
    sync_after_mutation(
        parent,
        format!(
            "identical pending record was removed from {}",
            parent.display()
        ),
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
    fail_before_publication(&record)?;
    let canonical_pending = pending_path(&record.target);
    let mut canonical_cleanup = PendingCleanup::new(canonical_pending.clone());
    if let Err(error) = write_pending(&canonical_pending, &record.bytes) {
        return Err(canonical_cleanup.after_failure(error));
    }
    let publication = if injected_storage_failure("hard link recovered record") {
        Err(io::Error::other("injected storage failure"))
    } else {
        fs::hard_link(&canonical_pending, &record.target)
    };
    if let Err(error) = map_io(publication, "publish recovered record", &record.target) {
        return Err(canonical_cleanup.after_failure(error));
    }
    sync_after_mutation(
        directory,
        format!(
            "recovered record may be published at {}",
            record.target.display()
        ),
    )?;
    if let Err(error) = canonical_cleanup.remove() {
        return Err(Error::AmbiguousEffect(format!(
            "recovered record was published at {} but canonical pending cleanup failed: {error}",
            record.target.display()
        )));
    }
    if let Err(error) = map_io(
        fs::remove_file(&record.path),
        "remove recovered pending file",
        &record.path,
    ) {
        return Err(Error::AmbiguousEffect(format!(
            "recovered record was published at {} but original pending cleanup failed: {error}",
            record.target.display()
        )));
    }
    sync_after_mutation(
        directory,
        format!(
            "recovered record was published but pending cleanup may not be durable at {}",
            record.target.display()
        ),
    )?;
    result.recovered.push(record.target.display().to_string());
    Ok(())
}

fn fail_before_publication(record: &PendingRecord) -> Result<()> {
    if injected_storage_failure("publish recovered record") {
        return Err(Error::io(
            "publish recovered record",
            &record.target,
            io::Error::other("injected storage failure"),
        ));
    }
    Ok(())
}

fn sync_after_mutation(directory: &Path, effect: String) -> Result<()> {
    sync_directory(directory).map_err(|source| {
        Error::AmbiguousEffect(format!("{effect}; directory sync failed: {source}"))
    })
}
