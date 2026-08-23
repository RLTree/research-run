use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

#[path = "agent_integration_atomic_exchange.rs"]
pub(super) mod atomic_exchange;
#[path = "agent_integration_transaction_cleanup.rs"]
pub(super) mod cleanup;
#[path = "agent_integration_transaction_cleanup_effects.rs"]
mod cleanup_effects;
#[path = "agent_integration_transaction_completion_io.rs"]
mod completion_io;
#[path = "agent_integration_transaction_completion_validation.rs"]
mod completion_validation;
#[path = "agent_integration_exchange_directories.rs"]
pub(super) mod directories;
#[path = "agent_integration_transaction_receipt.rs"]
mod receipt;
#[path = "agent_integration_transaction_receipt_io.rs"]
pub(super) mod receipt_io;
#[path = "agent_integration_transaction_witnesses.rs"]
pub(super) mod witnesses;

use super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::publication::pending_path;
use super::storage::{map_io, read_bounded_with_limit, sync_directory};
use atomic_exchange::{ensure_exchange_platform, exchange};
use cleanup::{complete_publication, sync_exchanged_state};
use cleanup_effects::{ambiguous, open_recovered_handles};
use directories::create_transaction_directory;
use witnesses::{
    TRANSACTION_VERSION, WitnessPaths, stage_witnesses, sync_witnesses, verify_witnesses,
};

pub(super) fn publish_append(
    target: &Path,
    content: &[u8],
    expected: &[u8],
    plan_sha256: &str,
) -> Result<bool> {
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
    exchange_and_finish(
        target,
        &transaction,
        expected,
        content,
        plan_sha256,
        TRANSACTION_VERSION,
    )?;
    Ok(true)
}

pub(super) fn exchange_and_finish(
    target: &Path,
    transaction: &Path,
    original: &[u8],
    reviewed: &[u8],
    plan_sha256: &str,
    transaction_version: &[u8],
) -> Result<()> {
    let paths = WitnessPaths::new(transaction);
    sync_witnesses(target, transaction, &paths).map_err(|error| ambiguous(target, error))?;
    let handles = match exchange(target, transaction) {
        Ok(handles) => handles,
        Err(error @ Error::AmbiguousEffect(_)) => return Err(error),
        Err(error) => return Err(abort_transaction(target, transaction, error)),
    };
    sync_after_exchange(target, transaction, &handles)?;
    verify_post_exchange(target, transaction, original, reviewed, transaction_version)?;
    complete_publication(
        target,
        transaction,
        &handles,
        plan_sha256,
        original,
        reviewed,
        transaction_version,
    )
}

pub(super) fn finish_verified_exchange(
    target: &Path,
    transaction: &Path,
    original: &[u8],
    reviewed: &[u8],
    plan_sha256: &str,
    transaction_version: &[u8],
) -> Result<()> {
    let handles =
        open_recovered_handles(target, transaction).map_err(|error| ambiguous(target, error))?;
    sync_after_exchange(target, transaction, &handles)?;
    verify_post_exchange(target, transaction, original, reviewed, transaction_version)?;
    complete_publication(
        target,
        transaction,
        &handles,
        plan_sha256,
        original,
        reviewed,
        transaction_version,
    )
}

pub(in crate::workspace) fn create_transaction(transaction: &Path) -> Result<()> {
    create_transaction_directory(transaction)
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

fn sync_after_exchange(
    target: &Path,
    transaction: &Path,
    handles: &directories::ExchangeHandles,
) -> Result<()> {
    sync_exchanged_state(target, transaction, handles).map_err(|error| ambiguous(target, error))
}

fn verify_post_exchange(
    target: &Path,
    transaction: &Path,
    original: &[u8],
    reviewed: &[u8],
    transaction_version: &[u8],
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
    verify_witnesses(
        target,
        &paths,
        original,
        reviewed,
        original,
        transaction_version,
    )
    .map_err(|error| ambiguous(target, error))
}

pub(super) fn cleanup_staging_transaction(target: &Path, transaction: &Path) -> Result<()> {
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
    match cleanup_staging_transaction(target, transaction) {
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

#[cfg(all(coverage, test))]
#[path = "tests/agent_integration_transaction_coverage.rs"]
mod coverage_tests;
