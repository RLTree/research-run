use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::path::Path;

use rustix::fs::Dir;

use crate::{Error, Result};

use super::super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::cleanup_effects::{ambiguous, remove_transaction};
use super::completion_io::{read_leaf, sync_handle, unlink_file};
use super::directories::{ExchangeHandles, verify_relative_directory_anchor};

const STAGING_WITNESSES: [&str; 4] = ["exchange", "original", "reviewed", "version"];

pub(in crate::workspace) fn cleanup_staging_transaction(
    target: &Path,
    transaction: &Path,
    handles: &ExchangeHandles,
) -> Result<()> {
    verify_relative_directory_anchor(&handles.root, transaction, &handles.transaction)
        .map_err(|error| ambiguous(target, error))?;
    let names = inspect_staging_witnesses(transaction, &handles.transaction)
        .map_err(|error| ambiguous(target, error))?;
    if super::super::injected_storage_failure("remove abandoned pending record") {
        return Err(ambiguous(
            target,
            injected("remove abandoned pending record", transaction),
        ));
    }
    for name in names {
        unlink_file(
            &handles.transaction,
            &name,
            "remove project instruction staging witness",
        )
        .map_err(|error| ambiguous(target, error))?;
    }
    sync_handle(
        &handles.transaction,
        "sync project instruction staging cleanup",
        transaction,
    )
    .map_err(|error| ambiguous(target, error))?;
    verify_relative_directory_anchor(&handles.root, transaction, &handles.transaction)
        .map_err(|error| ambiguous(target, error))?;
    remove_transaction(&handles.root, transaction).map_err(|error| ambiguous(target, error))?;
    sync_handle(&handles.root, "sync abandoned pending directory", target)
        .map_err(|error| ambiguous(target, error))
}

pub(super) fn abort_transaction(
    target: &Path,
    transaction: &Path,
    handles: &ExchangeHandles,
    original: Error,
) -> Error {
    match cleanup_staging_transaction(target, transaction, handles) {
        Ok(()) => original,
        Err(cleanup) => Error::AmbiguousEffect(format!(
            "project instruction staging failed and cleanup was not durable: {original}; {cleanup}"
        )),
    }
}

fn inspect_staging_witnesses(transaction: &Path, directory: &File) -> Result<Vec<OsString>> {
    use std::os::unix::ffi::OsStrExt;

    let entries = Dir::read_from(directory).map_err(|error| {
        Error::io(
            "inspect project instruction staging cleanup",
            transaction,
            error.into(),
        )
    })?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            Error::io(
                "inspect project instruction staging cleanup",
                transaction,
                error.into(),
            )
        })?;
        let name = OsStr::from_bytes(entry.file_name().to_bytes());
        if matches!(name.as_bytes(), b"." | b"..") {
            continue;
        }
        if names.len() == STAGING_WITNESSES.len()
            || !STAGING_WITNESSES.iter().any(|allowed| name == *allowed)
        {
            return Err(Error::AmbiguousEffect(
                "project instruction staging cleanup found an unexpected witness".to_owned(),
            ));
        }
        let leaf = read_leaf(
            directory,
            name,
            MAX_INSTRUCTION_BYTES,
            &transaction.join(name),
        )?;
        if leaf.links != 1 {
            return Err(Error::Conflict(
                "project instruction staging witness is not independent".to_owned(),
            ));
        }
        names.push(name.to_os_string());
    }
    names.sort();
    Ok(names)
}

fn injected(action: &'static str, path: &Path) -> Error {
    Error::io(
        action,
        path,
        std::io::Error::other("injected cleanup failure"),
    )
}
