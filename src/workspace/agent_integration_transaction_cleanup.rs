use std::ffi::OsStr;
use std::path::Path;

use crate::domain::AgentIntegrationPlan;
use crate::{Error, Result};

use super::cleanup_effects::{
    canonical_mode, make_receipt_durable, pre_terminal, remove_receipt, remove_transaction,
    remove_witness, terminal_error, transaction_version_number, untrusted_state,
};
use super::completion_io::{Leaf, leaf, sync_handle, sync_leaf};
use super::completion_validation::{read_receipt, validate_canonical, validate_survivors};
use super::directories::{
    ExchangeHandles, open_directory, open_exchange_handles_from_root, verify_anchor,
};
use super::receipt::{CompletionReceipt, completion_path};
use super::receipt_io::{publish_receipt, stage_receipt};

pub(super) fn sync_exchanged_state(
    target: &Path,
    transaction: &Path,
    handles: &ExchangeHandles,
) -> Result<()> {
    sync_leaf(
        &handles.root,
        leaf(target, "project instructions")?,
        "sync installed project instructions after exchange",
        target,
    )?;
    sync_leaf(
        &handles.transaction,
        OsStr::new("exchange"),
        "sync exchanged project instruction witness after exchange",
        &transaction.join("exchange"),
    )?;
    sync_handle(
        &handles.transaction,
        "sync project instruction transaction after exchange",
        transaction,
    )?;
    sync_handle(
        &handles.root,
        "sync project instruction root after exchange",
        target,
    )
}

pub(super) fn complete_publication(
    target: &Path,
    transaction: &Path,
    handles: &ExchangeHandles,
    plan_sha256: &str,
    original: &[u8],
    reviewed: &[u8],
    transaction_version: &[u8],
) -> Result<()> {
    let receipt = CompletionReceipt::new(
        target,
        transaction,
        transaction_version_number(transaction_version)
            .map_err(|error| pre_terminal(target, error))?,
        plan_sha256,
        original,
        reviewed,
        canonical_mode(target, handles).map_err(|error| pre_terminal(target, error))?,
    )
    .map_err(|error| pre_terminal(target, error))?;
    let receipt_path = completion_path(transaction).map_err(|error| pre_terminal(target, error))?;
    let receipt_bytes = receipt.bytes();
    stage_receipt(&handles.transaction, transaction, &receipt_bytes)
        .map_err(|error| pre_terminal(target, error))?;
    publish_receipt(&handles.transaction, &handles.root, &receipt_path)
        .map_err(|error| pre_terminal(target, error))?;
    sync_handle(
        &handles.root,
        "sync project instruction root after completion receipt",
        target,
    )
    .map_err(|error| pre_terminal(target, error))?;
    let (published, receipt_leaf) = read_receipt(&handles.root, &receipt_path)
        .map_err(|error| terminal_error(target, error))?;
    published
        .validate_pair(&receipt_path, Some(transaction))
        .map_err(|error| terminal_error(target, error))?;
    if receipt_leaf.bytes != receipt_bytes {
        return Err(terminal_error(
            target,
            Error::Conflict("published completion receipt bytes changed".to_owned()),
        ));
    }
    cleanup_with_transaction(
        target,
        transaction,
        &receipt_path,
        &receipt,
        &receipt_leaf,
        handles,
    )
}

pub(in crate::workspace) fn recover_completion(
    receipt_path: &Path,
    transaction: Option<&Path>,
    target: &Path,
    plan: &AgentIntegrationPlan,
) -> Result<()> {
    let root_path = target.parent().expect("instructions have a parent");
    let root =
        (if super::super::injected_storage_failure("open committed project instruction root") {
            Err(Error::io(
                "open committed project instruction root",
                root_path,
                std::io::Error::other("injected storage failure"),
            ))
        } else {
            open_directory(root_path)
        })
        .map_err(|error| untrusted_state(target, error))?;
    verify_anchor(root_path, &root).map_err(|error| untrusted_state(target, error))?;
    let (receipt, receipt_leaf) =
        read_receipt(&root, receipt_path).map_err(|error| untrusted_state(target, error))?;
    receipt
        .validate_pair(receipt_path, transaction)
        .and_then(|()| receipt.validate_for_plan(plan))
        .map_err(|error| untrusted_state(target, error))?;
    match transaction {
        Some(transaction) => {
            let handles = open_exchange_handles_from_root(target, transaction, root)
                .map_err(|error| untrusted_state(target, error))?;
            validate_canonical(target, &handles.root, &receipt)
                .map_err(|error| untrusted_state(target, error))?;
            validate_survivors(transaction, &handles.transaction, &receipt, &receipt_leaf)
                .map_err(|error| untrusted_state(target, error))?;
            make_receipt_durable(target, receipt_path, &handles.root)
                .map_err(|error| untrusted_state(target, error))?;
            cleanup_with_transaction(
                target,
                transaction,
                receipt_path,
                &receipt,
                &receipt_leaf,
                &handles,
            )
        }
        None => {
            validate_canonical(target, &root, &receipt)
                .map_err(|error| untrusted_state(target, error))?;
            if receipt_leaf.links != 1 {
                return Err(untrusted_state(
                    target,
                    Error::Conflict(
                        "receipt-only completion has an unexpected hard link".to_owned(),
                    ),
                ));
            }
            make_receipt_durable(target, receipt_path, &root)
                .map_err(|error| untrusted_state(target, error))?;
            remove_receipt(target, receipt_path, &root)
        }
    }
}

fn cleanup_with_transaction(
    target: &Path,
    transaction: &Path,
    receipt_path: &Path,
    receipt: &CompletionReceipt,
    receipt_leaf: &Leaf,
    handles: &ExchangeHandles,
) -> Result<()> {
    validate_canonical(target, &handles.root, receipt)
        .map_err(|error| terminal_error(target, error))?;
    let survivors = validate_survivors(transaction, &handles.transaction, receipt, receipt_leaf)
        .map_err(|error| terminal_error(target, error))?;
    for name in survivors {
        remove_witness(&handles.transaction, &name)
            .map_err(|error| terminal_error(target, error))?;
    }
    sync_handle(
        &handles.transaction,
        "sync project instruction transaction during cleanup",
        transaction,
    )
    .map_err(|error| terminal_error(target, error))?;
    remove_transaction(&handles.root, transaction)
        .map_err(|error| terminal_error(target, error))?;
    sync_handle(
        &handles.root,
        "sync project instruction root after cleanup",
        target,
    )
    .map_err(|error| terminal_error(target, error))?;
    remove_receipt(target, receipt_path, &handles.root)
}

#[cfg(test)]
#[path = "tests/agent_integration_cleanup.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/agent_integration_cleanup_recovery.rs"]
mod recovery_tests;

#[cfg(test)]
#[path = "tests/agent_integration_cleanup_security.rs"]
mod security_tests;

#[cfg(test)]
#[path = "tests/agent_integration_cleanup_special_files.rs"]
mod special_file_tests;
