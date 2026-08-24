use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;

use crate::domain::digest;
use crate::{Error, Result};

use super::super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::completion_io::{Leaf, list_names, read_leaf};
use super::receipt::{CompletionReceipt, MAX_COMPLETION_RECEIPT_BYTES};
use super::receipt_io::COMPLETION_WITNESS;
use super::witnesses::{PREVIOUS_TRANSACTION_VERSION, TRANSACTION_VERSION};

const WITNESS_NAMES: [&str; 5] = [
    "version",
    "exchange",
    "original",
    "reviewed",
    COMPLETION_WITNESS,
];

pub(super) fn read_receipt(root: &File, receipt_path: &Path) -> Result<(CompletionReceipt, Leaf)> {
    let leaf = read_leaf(
        root,
        receipt_path
            .file_name()
            .ok_or_else(|| Error::invalid("completion receipt", "missing leaf name"))?,
        MAX_COMPLETION_RECEIPT_BYTES,
        receipt_path,
    )?;
    if !matches!(leaf.links, 1 | 2) || leaf.mode & 0o777 != 0o600 {
        return Err(Error::Conflict(
            "completion receipt is not one private independent regular file".to_owned(),
        ));
    }
    let receipt = CompletionReceipt::parse(&leaf.bytes, receipt_path)?;
    Ok((receipt, leaf))
}

pub(super) fn validate_canonical(
    target: &Path,
    root: &File,
    receipt: &CompletionReceipt,
) -> Result<()> {
    let name = target
        .file_name()
        .ok_or_else(|| Error::invalid("project instructions", "missing leaf name"))?;
    let leaf = read_leaf(root, name, MAX_INSTRUCTION_BYTES, target)?;
    if leaf.links != 1
        || leaf.bytes.len() as u64 != receipt.reviewed_bytes
        || digest(&leaf.bytes) != receipt.reviewed_sha256
        || leaf.mode != receipt.unix_mode
    {
        return Err(Error::Conflict(
            "canonical project instructions do not match completion receipt".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn validate_survivors(
    transaction: &Path,
    directory: &File,
    receipt: &CompletionReceipt,
    receipt_leaf: &Leaf,
) -> Result<Vec<String>> {
    let mut survivors = BTreeSet::new();
    for name in list_names(directory, transaction, &WITNESS_NAMES)? {
        let Some(name) = name.to_str() else {
            return Err(Error::Conflict(
                "completion transaction contains a non-UTF-8 name".to_owned(),
            ));
        };
        if !WITNESS_NAMES.contains(&name) {
            return Err(Error::Conflict(format!(
                "unexpected completion transaction child: {name}"
            )));
        }
        validate_survivor(directory, transaction, name, receipt, receipt_leaf)?;
        survivors.insert(name.to_owned());
    }
    let expected_links = u64::from(survivors.contains(COMPLETION_WITNESS)) + 1;
    if receipt_leaf.links != expected_links {
        return Err(Error::Conflict(
            "completion receipt has an unexpected hard-link identity".to_owned(),
        ));
    }
    Ok(survivors.into_iter().collect())
}

fn validate_survivor(
    directory: &File,
    transaction: &Path,
    name: &str,
    receipt: &CompletionReceipt,
    receipt_leaf: &Leaf,
) -> Result<()> {
    let path = transaction.join(name);
    let leaf = read_leaf(directory, OsStr::new(name), MAX_INSTRUCTION_BYTES, &path)?;
    if name == COMPLETION_WITNESS {
        return validate_completion_link(&path, &leaf, receipt_leaf);
    }
    if leaf.links != 1 {
        return Err(Error::Conflict(format!(
            "completion witness is not independent: {}",
            path.display()
        )));
    }
    if name != "version" && leaf.mode != receipt.unix_mode {
        return Err(Error::Conflict(format!(
            "completion witness mode changed: {}",
            path.display()
        )));
    }
    validate_survivor_bytes(&path, name, &leaf, receipt)
}

fn validate_completion_link(path: &Path, leaf: &Leaf, receipt: &Leaf) -> Result<()> {
    if leaf.links != 2
        || leaf.mode & 0o777 != 0o600
        || leaf.device != receipt.device
        || leaf.inode != receipt.inode
        || leaf.bytes != receipt.bytes
    {
        return Err(Error::Conflict(format!(
            "completion witness is not the published receipt link: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_survivor_bytes(
    path: &Path,
    name: &str,
    leaf: &Leaf,
    receipt: &CompletionReceipt,
) -> Result<()> {
    let expected = match name {
        "version" => return validate_version(&leaf.bytes, receipt.transaction_version),
        "reviewed" => (receipt.reviewed_bytes, receipt.reviewed_sha256.as_str()),
        "original" | "exchange" => (receipt.original_bytes, receipt.original_sha256.as_str()),
        _ => unreachable!("names were checked above"),
    };
    if leaf.bytes.len() as u64 != expected.0 || digest(&leaf.bytes) != expected.1 {
        return Err(Error::Conflict(format!(
            "completion witness bytes changed: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_version(bytes: &[u8], version: u32) -> Result<()> {
    let expected = match version {
        2 => PREVIOUS_TRANSACTION_VERSION,
        3 => TRANSACTION_VERSION,
        _ => {
            return Err(Error::Conflict(
                "unknown completion transaction version".to_owned(),
            ));
        }
    };
    if bytes != expected {
        return Err(Error::Conflict(
            "completion transaction version changed".to_owned(),
        ));
    }
    Ok(())
}
