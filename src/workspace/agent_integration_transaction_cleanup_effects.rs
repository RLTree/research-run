use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;

use crate::{Error, Result};

use super::super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::completion_io::{
    leaf, read_leaf, sync_handle, sync_leaf, unlink_directory, unlink_file,
};
use super::directories::{ExchangeHandles, open_exchange_handles};
use super::witnesses::{PREVIOUS_TRANSACTION_VERSION, TRANSACTION_VERSION};

pub(super) fn remove_receipt(target: &Path, path: &Path, root: &File) -> Result<()> {
    if super::super::injected_storage_failure("remove project instruction completion receipt") {
        return Err(terminal_error(
            target,
            injected("remove project instruction completion receipt", path),
        ));
    }
    unlink_file(
        root,
        leaf(path, "completion receipt")?,
        "remove project instruction completion receipt",
    )
    .map_err(|error| terminal_error(target, error))?;
    sync_handle(
        root,
        "sync project instruction root after receipt cleanup",
        target,
    )
    .map_err(|error| terminal_error(target, error))
}

pub(super) fn make_receipt_durable(target: &Path, receipt: &Path, root: &File) -> Result<()> {
    sync_leaf(
        root,
        leaf(receipt, "completion receipt")?,
        "sync recovered project instruction completion receipt",
        receipt,
    )?;
    sync_handle(
        root,
        "sync project instruction root before recovered cleanup",
        target,
    )
}

pub(super) fn remove_witness(transaction: &File, name: &str) -> Result<()> {
    if super::super::injected_storage_failure("remove project instruction witness") {
        return Err(injected(
            "remove project instruction witness",
            Path::new(name),
        ));
    }
    unlink_file(
        transaction,
        OsStr::new(name),
        "remove project instruction witness",
    )
}

pub(super) fn remove_transaction(root: &File, transaction: &Path) -> Result<()> {
    if super::super::injected_storage_failure("remove project instruction transaction") {
        return Err(injected(
            "remove project instruction transaction",
            transaction,
        ));
    }
    unlink_directory(
        root,
        leaf(transaction, "instruction transaction")?,
        transaction,
        "remove project instruction transaction",
    )
}

pub(super) fn canonical_mode(target: &Path, handles: &ExchangeHandles) -> Result<u32> {
    if super::super::injected_storage_failure(
        "inspect installed project instruction after exchange",
    ) {
        return Err(injected(
            "inspect installed project instruction after exchange",
            target,
        ));
    }
    let receipt = read_leaf(
        &handles.root,
        leaf(target, "project instructions")?,
        MAX_INSTRUCTION_BYTES,
        target,
    )?;
    if receipt.links != 1 {
        return Err(Error::Conflict(
            "installed project instructions are not one independent file".to_owned(),
        ));
    }
    Ok(receipt.mode)
}

pub(super) fn open_recovered_handles(target: &Path, transaction: &Path) -> Result<ExchangeHandles> {
    if super::super::injected_storage_failure(
        "open recovered project instruction transaction after exchange",
    ) {
        return Err(injected(
            "open recovered project instruction transaction after exchange",
            transaction,
        ));
    }
    open_exchange_handles(target, transaction)
}

pub(super) fn transaction_version_number(version: &[u8]) -> Result<u32> {
    match version {
        PREVIOUS_TRANSACTION_VERSION => Ok(2),
        TRANSACTION_VERSION => Ok(3),
        _ => Err(Error::AmbiguousEffect(
            "project instruction transaction version is unknown".to_owned(),
        )),
    }
}

pub(super) fn pre_terminal(target: &Path, error: Error) -> Error {
    Error::AmbiguousEffect(format!(
        "project instruction publication may have changed {}; full transaction evidence remains: {error}",
        target.display()
    ))
}

pub(super) fn ambiguous(target: &Path, error: Error) -> Error {
    Error::AmbiguousEffect(format!(
        "project instruction publication may have changed {}; retained transaction evidence: {error}",
        target.display()
    ))
}

pub(super) fn terminal_error(target: &Path, error: Error) -> Error {
    Error::AmbiguousEffect(format!(
        "project instruction publication is committed at {}; plan-bound completion cleanup remains: {error}",
        target.display()
    ))
}

pub(super) fn untrusted_state(target: &Path, error: Error) -> Error {
    Error::AmbiguousEffect(format!(
        "project instruction completion state at {} is untrusted and retained: {error}",
        target.display()
    ))
}

fn injected(action: &'static str, path: &Path) -> Error {
    Error::io(
        action,
        path,
        std::io::Error::other("injected storage failure"),
    )
}
