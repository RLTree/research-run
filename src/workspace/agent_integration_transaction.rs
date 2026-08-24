#[cfg(any(test, coverage))]
use std::fs;
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
#[path = "agent_integration_transaction_directory_creation.rs"]
pub(super) mod directory_creation;
#[path = "agent_integration_transaction_receipt.rs"]
mod receipt;
#[path = "agent_integration_transaction_receipt_io.rs"]
pub(super) mod receipt_io;
#[path = "agent_integration_transaction_staging_cleanup.rs"]
mod staging_cleanup;
#[path = "agent_integration_transaction_witnesses.rs"]
pub(super) mod witnesses;

use super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::publication::pending_path;
use super::storage::read_bounded_with_limit;
use atomic_exchange::{ensure_exchange_platform, exchange_with_handles};
use cleanup::{complete_publication, sync_exchanged_state};
use cleanup_effects::{ambiguous, open_recovered_handles};
use directories::{ExchangeHandles, open_exchange_handles};
use directory_creation::create_transaction_directory;
use staging_cleanup::abort_transaction;
#[cfg(test)]
pub(in crate::workspace) use staging_cleanup::cleanup_staging_transaction;
use witnesses::{
    TRANSACTION_VERSION, WitnessPaths, inspect_independent_append_source, stage_witnesses,
    sync_witnesses, verify_witnesses,
};

pub(super) fn publish_append(
    target: &Path,
    content: &[u8],
    expected: &[u8],
    plan_sha256: &str,
) -> Result<bool> {
    ensure_exchange_platform()?;
    let source_metadata = inspect_independent_append_source(target)?;
    let transaction = transaction_path(target);
    create_transaction(&transaction)?;
    let handles =
        open_exchange_handles(target, &transaction).map_err(|error| ambiguous(target, error))?;
    let paths = WitnessPaths::new(&transaction);
    if let Err(error) = stage_witnesses(
        target,
        &transaction,
        &handles,
        &paths,
        &source_metadata,
        expected,
        content,
    ) {
        return Err(abort_transaction(target, &transaction, &handles, error));
    }
    inject_concurrent_source_change(target);
    let current = match read_bounded_with_limit(target, MAX_INSTRUCTION_BYTES) {
        Ok(current) => current,
        Err(error) => return Err(abort_transaction(target, &transaction, &handles, error)),
    };
    if current != expected {
        return Err(abort_transaction(
            target,
            &transaction,
            &handles,
            Error::Conflict(format!(
                "project instruction file changed during append publication: {}",
                target.display()
            )),
        ));
    }
    exchange_and_finish_with_handles(
        target,
        &transaction,
        handles,
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
    let handles =
        open_exchange_handles(target, transaction).map_err(|error| ambiguous(target, error))?;
    exchange_and_finish_with_handles(
        target,
        transaction,
        handles,
        original,
        reviewed,
        plan_sha256,
        transaction_version,
    )
}

fn exchange_and_finish_with_handles(
    target: &Path,
    transaction: &Path,
    handles: ExchangeHandles,
    original: &[u8],
    reviewed: &[u8],
    plan_sha256: &str,
    transaction_version: &[u8],
) -> Result<()> {
    let paths = WitnessPaths::new(transaction);
    sync_witnesses(target, transaction, &handles, &paths)
        .map_err(|error| ambiguous(target, error))?;
    match exchange_with_handles(target, transaction, &handles) {
        Ok(()) => {}
        Err(error @ Error::AmbiguousEffect(_)) => return Err(error),
        Err(error) => return Err(abort_transaction(target, transaction, &handles, error)),
    }
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
