use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

use super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::pending_cleanup::PendingCleanup;
use super::publication::{pending_path, write_pending};
use super::storage::{map_io, read_bounded_with_limit, sync_directory};

pub(super) fn publish_append(target: &Path, content: &[u8], expected: &[u8]) -> Result<bool> {
    let transaction = transaction_path(target);
    create_transaction(&transaction)?;
    let planned = transaction.join("planned");
    if let Err(error) = write_pending(&planned, content) {
        return Err(abort_transaction(&transaction, error));
    }
    inject_concurrent_source_change(target);
    let source = transaction.join("source");
    if let Err(error) = withdraw_source(target, &source) {
        return Err(abort_transaction(&transaction, error));
    }
    let observed = match read_bounded_with_limit(&source, MAX_INSTRUCTION_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => return Err(ambiguous(target, error)),
    };
    if observed != expected {
        restore_source(target, &source, &transaction)?;
        return Err(Error::Conflict(format!(
            "project instruction file changed during append publication: {}",
            target.display()
        )));
    }
    if let Err(error) = copy_permissions(&source, &planned) {
        return Err(restore_after_failure(target, &source, &transaction, error));
    }
    link_planned(target, &planned)?;
    verify_published(target, &source, content, expected)?;
    sync_directory(target.parent().expect("instructions have a parent"))
        .map_err(|error| ambiguous(target, error))?;
    cleanup_after_publication(target, &transaction)?;
    Ok(true)
}

fn create_transaction(transaction: &Path) -> Result<()> {
    map_io(
        fs::create_dir(transaction),
        "create project instruction transaction",
        transaction,
    )?;
    sync_directory(transaction.parent().expect("transaction has a parent"))
}

fn transaction_path(target: &Path) -> PathBuf {
    let pending = pending_path(target);
    let name = pending
        .file_name()
        .and_then(|name| name.to_str())
        .expect("instruction paths are UTF-8")
        .strip_suffix(".tmp")
        .expect("pending paths end in .tmp");
    pending.with_file_name(format!("{name}.txn"))
}

fn withdraw_source(target: &Path, source: &Path) -> Result<()> {
    if super::injected_storage_failure("replace project instructions") {
        return Err(Error::io(
            "preserve project instructions",
            target,
            io::Error::other("injected storage failure"),
        ));
    }
    map_io(
        fs::rename(target, source),
        "preserve project instructions",
        target,
    )
}

pub(super) fn link_planned(target: &Path, planned: &Path) -> Result<()> {
    inject_concurrent_target_claim(target);
    match fs::hard_link(planned, target) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            Err(Error::AmbiguousEffect(format!(
                "project instruction file appeared while the reviewed source was preserved: {}",
                target.display()
            )))
        }
        Err(error) => Err(ambiguous(
            target,
            Error::io("publish project instructions", target, error),
        )),
    }
}

fn verify_published(target: &Path, source: &Path, content: &[u8], expected: &[u8]) -> Result<()> {
    let installed = read_bounded_with_limit(target, MAX_INSTRUCTION_BYTES)
        .map_err(|error| ambiguous(target, error))?;
    let preserved = read_bounded_with_limit(source, MAX_INSTRUCTION_BYTES)
        .map_err(|error| ambiguous(target, error))?;
    if installed != content || preserved != expected {
        return Err(Error::AmbiguousEffect(format!(
            "project instruction bytes changed during atomic append publication: {}",
            target.display()
        )));
    }
    Ok(())
}

fn restore_after_failure(
    target: &Path,
    source: &Path,
    transaction: &Path,
    original: Error,
) -> Error {
    match restore_source(target, source, transaction) {
        Ok(()) => original,
        Err(error) => ambiguous(target, error),
    }
}

pub(super) fn restore_source(target: &Path, source: &Path, transaction: &Path) -> Result<()> {
    match fs::hard_link(source, target) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(Error::AmbiguousEffect(format!(
                "project instruction file appeared while preserved bytes awaited restoration: {}",
                target.display()
            )));
        }
        Err(error) => {
            return Err(ambiguous(
                target,
                Error::io("restore project instructions", target, error),
            ));
        }
    }
    sync_directory(target.parent().expect("instructions have a parent"))
        .map_err(|error| ambiguous(target, error))?;
    cleanup_after_publication(target, transaction)
}

fn copy_permissions(source: &Path, planned: &Path) -> Result<()> {
    let permissions = map_io(
        fs::metadata(source),
        "inspect project instruction permissions",
        source,
    )?
    .permissions();
    map_io(
        fs::set_permissions(planned, permissions),
        "preserve project instruction permissions",
        planned,
    )
}

pub(super) fn cleanup_after_publication(target: &Path, transaction: &Path) -> Result<()> {
    for path in [transaction.join("planned"), transaction.join("source")] {
        let mut cleanup = PendingCleanup::new(path);
        cleanup.remove().map_err(|error| ambiguous(target, error))?;
    }
    match fs::remove_dir(transaction) {
        Ok(()) => sync_directory(transaction.parent().expect("transaction has a parent"))
            .map_err(|error| ambiguous(target, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ambiguous(
            target,
            Error::io("remove project instruction transaction", transaction, error),
        )),
    }
}

fn abort_transaction(transaction: &Path, original: Error) -> Error {
    let target = transaction.with_extension("instruction");
    match cleanup_after_publication(&target, transaction) {
        Ok(()) => original,
        Err(cleanup) => Error::AmbiguousEffect(format!(
            "project instruction staging failed and cleanup was not durable: {original}; {cleanup}"
        )),
    }
}

pub(super) fn ambiguous(target: &Path, error: Error) -> Error {
    Error::AmbiguousEffect(format!(
        "project instruction publication may have changed {}; inspect and retry the reviewed plan: {error}",
        target.display()
    ))
}

#[cfg(any(test, coverage))]
fn inject_concurrent_source_change(target: &Path) {
    if super::take_storage_failure("agent instruction changed before append publication") {
        fs::write(target, b"concurrent instruction bytes")
            .expect("write injected concurrent project instructions");
    }
}

#[cfg(not(any(test, coverage)))]
fn inject_concurrent_source_change(_target: &Path) {}

#[cfg(any(test, coverage))]
fn inject_concurrent_target_claim(target: &Path) {
    if super::take_storage_failure("agent instruction appeared during append publication") {
        fs::write(target, b"concurrent target claimant")
            .expect("write injected concurrent target claimant");
    }
}

#[cfg(not(any(test, coverage)))]
fn inject_concurrent_target_claim(_target: &Path) {}
