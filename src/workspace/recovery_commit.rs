use std::fs;
use std::io;
use std::path::Path;

use crate::{Error, Result};

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

fn discard_identical(record: PendingRecord, result: &mut RecoveryResult) -> Result<()> {
    if read_bounded(&record.target)? != record.bytes
        || injected_storage_failure("recovery target conflict")
    {
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
    sync_directory(parent)?;
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
    let publication = if injected_storage_failure("hard link recovered record") {
        Err(io::Error::other("injected storage failure"))
    } else {
        fs::hard_link(&record.path, &record.target)
    };
    map_io(publication, "publish recovered record", &record.target)?;
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
