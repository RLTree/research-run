use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

#[path = "agent_integration_atomic_exchange.rs"]
mod atomic_exchange;
#[path = "agent_integration_transaction_witnesses.rs"]
pub(super) mod witnesses;

use super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::publication::pending_path;
use super::storage::{map_io, read_bounded_with_limit, sync_directory};
use atomic_exchange::{ensure_exchange_platform, exchange};
use witnesses::{WitnessPaths, stage_witnesses, sync_witnesses, verify_witnesses};

pub(super) fn publish_append(target: &Path, content: &[u8], expected: &[u8]) -> Result<bool> {
    ensure_exchange_platform()?;
    let transaction = transaction_path(target);
    create_transaction(&transaction)?;
    let paths = WitnessPaths::new(&transaction);
    if let Err(error) = stage_witnesses(target, &transaction, &paths, expected, content) {
        return Err(abort_transaction(target, &transaction, error));
    }
    inject_concurrent_source_change(target);
    let current = match read_bounded_with_limit(target, MAX_INSTRUCTION_BYTES) {
        Ok(current) => current,
        Err(error) => return Err(abort_transaction(target, &transaction, error)),
    };
    if current != expected {
        return Err(abort_transaction(
            target,
            &transaction,
            Error::Conflict(format!(
                "project instruction file changed during append publication: {}",
                target.display()
            )),
        ));
    }
    exchange_and_finish(target, &transaction, expected, content)?;
    Ok(true)
}

pub(super) fn exchange_and_finish(
    target: &Path,
    transaction: &Path,
    original: &[u8],
    reviewed: &[u8],
) -> Result<()> {
    let paths = WitnessPaths::new(transaction);
    sync_witnesses(target, transaction, &paths).map_err(|error| ambiguous(target, error))?;
    match exchange(target, transaction) {
        Ok(()) => {}
        Err(error @ Error::AmbiguousEffect(_)) => return Err(error),
        Err(error) => return Err(abort_transaction(target, transaction, error)),
    }
    sync_after_exchange(target, transaction)?;
    verify_post_exchange(target, transaction, original, reviewed)?;
    cleanup_after_publication(target, transaction)
}

pub(super) fn finish_verified_exchange(
    target: &Path,
    transaction: &Path,
    original: &[u8],
    reviewed: &[u8],
) -> Result<()> {
    sync_after_exchange(target, transaction)?;
    verify_post_exchange(target, transaction, original, reviewed)?;
    cleanup_after_publication(target, transaction)
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

fn sync_after_exchange(target: &Path, transaction: &Path) -> Result<()> {
    sync_named_directory(
        transaction,
        "sync project instruction transaction after exchange",
    )
    .map_err(|error| ambiguous(target, error))?;
    sync_named_directory(
        target.parent().expect("instructions have a parent"),
        "sync project instruction root after exchange",
    )
    .map_err(|error| ambiguous(target, error))
}

fn verify_post_exchange(
    target: &Path,
    transaction: &Path,
    original: &[u8],
    reviewed: &[u8],
) -> Result<()> {
    let paths = WitnessPaths::new(transaction);
    inject_post_exchange_change(target);
    let installed = read_bounded_with_limit(target, MAX_INSTRUCTION_BYTES)
        .map_err(|error| ambiguous(target, error))?;
    if installed != reviewed {
        return Err(Error::AmbiguousEffect(format!(
            "project instruction bytes changed during atomic exchange: {}",
            target.display()
        )));
    }
    verify_witnesses(target, &paths, original, reviewed, original)
        .map_err(|error| ambiguous(target, error))
}

pub(super) fn cleanup_after_publication(target: &Path, transaction: &Path) -> Result<()> {
    if super::injected_storage_failure("remove abandoned pending record")
        || super::injected_storage_failure("sync abandoned pending directory")
    {
        return Err(ambiguous(
            target,
            Error::io(
                "clean project instruction transaction",
                transaction,
                io::Error::other("injected cleanup failure"),
            ),
        ));
    }
    let paths = WitnessPaths::new(transaction);
    for path in [
        paths.version,
        paths.exchange,
        paths.original,
        paths.reviewed,
    ] {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(ambiguous(
                    target,
                    Error::io("remove project instruction witness", path, error),
                ));
            }
        }
    }
    sync_directory(transaction).map_err(|error| ambiguous(target, error))?;
    map_io(
        fs::remove_dir(transaction),
        "remove project instruction transaction",
        transaction,
    )
    .map_err(|error| ambiguous(target, error))?;
    sync_directory(transaction.parent().expect("transaction has a parent"))
        .map_err(|error| ambiguous(target, error))
}

fn abort_transaction(target: &Path, transaction: &Path, original: Error) -> Error {
    match cleanup_after_publication(target, transaction) {
        Ok(()) => original,
        Err(cleanup) => Error::AmbiguousEffect(format!(
            "project instruction staging failed and cleanup was not durable: {original}; {cleanup}"
        )),
    }
}

fn sync_named_directory(path: &Path, point: &'static str) -> Result<()> {
    if super::injected_storage_failure(point) {
        return Err(Error::io(
            "sync project instruction directory",
            path,
            io::Error::other("injected storage failure"),
        ));
    }
    sync_directory(path)
}

pub(super) fn ambiguous(target: &Path, error: Error) -> Error {
    Error::AmbiguousEffect(format!(
        "project instruction publication may have changed {}; retained transaction evidence: {error}",
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
fn inject_post_exchange_change(target: &Path) {
    if super::take_storage_failure("agent instruction changed after atomic exchange") {
        fs::write(target, b"post-exchange instruction bytes")
            .expect("write injected post-exchange project instructions");
    }
}

#[cfg(not(any(test, coverage)))]
fn inject_post_exchange_change(_target: &Path) {}

#[cfg(test)]
#[path = "tests/agent_integration_atomic_exchange.rs"]
mod tests;

#[cfg(all(coverage, test))]
#[path = "tests/agent_integration_transaction_coverage.rs"]
mod coverage_tests;
