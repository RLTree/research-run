use std::fs::{self, File};
use std::path::Path;

use crate::{Error, Result};

use super::super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::super::path_safety::reject_symlink_chain;
use super::super::publication::write_pending;
use super::super::storage::{map_io, read_bounded_with_limit};
#[cfg(not(unix))]
use super::atomic_exchange::ensure_exchange_platform;
use super::sync_named_directory;

pub(in crate::workspace) const TRANSACTION_VERSION: &[u8] =
    b"research-run-agent-integration-transaction-v2\n";

pub(in crate::workspace) struct WitnessPaths {
    pub(in crate::workspace) version: std::path::PathBuf,
    pub(in crate::workspace) original: std::path::PathBuf,
    pub(in crate::workspace) reviewed: std::path::PathBuf,
    pub(in crate::workspace) exchange: std::path::PathBuf,
}

impl WitnessPaths {
    pub(in crate::workspace) fn new(transaction: &Path) -> Self {
        Self {
            version: transaction.join("version"),
            original: transaction.join("original"),
            reviewed: transaction.join("reviewed"),
            exchange: transaction.join("exchange"),
        }
    }
}

pub(in crate::workspace) fn stage_witnesses(
    target: &Path,
    transaction: &Path,
    paths: &WitnessPaths,
    original: &[u8],
    reviewed: &[u8],
) -> Result<()> {
    if super::super::injected_storage_failure("inspect project instruction permissions") {
        return Err(Error::io(
            "inspect project instruction permissions",
            target,
            std::io::Error::other("injected storage failure"),
        ));
    }
    let metadata = map_io(
        fs::symlink_metadata(target),
        "inspect project instruction metadata",
        target,
    )?;
    if !metadata.is_file() {
        return Err(Error::invalid(
            "project instructions",
            format!("{} is not a regular file", target.display()),
        ));
    }
    write_pending(&paths.version, TRANSACTION_VERSION)?;
    write_pending(&paths.original, original)?;
    write_pending(&paths.reviewed, reviewed)?;
    write_pending(&paths.exchange, reviewed)?;
    preserve_mode(&metadata, paths)?;
    verify_witnesses(target, paths, original, reviewed, reviewed)?;
    sync_witnesses(target, transaction, paths)
}

pub(in crate::workspace) fn sync_witnesses(
    target: &Path,
    transaction: &Path,
    paths: &WitnessPaths,
) -> Result<()> {
    for path in [
        &paths.version,
        &paths.original,
        &paths.reviewed,
        &paths.exchange,
    ] {
        let file = map_io(File::open(path), "open project instruction witness", path)?;
        map_io(
            file.sync_all(),
            "sync transaction witness before exchange",
            path,
        )?;
    }
    sync_named_directory(
        transaction,
        "sync project instruction transaction before exchange",
    )?;
    sync_named_directory(
        target.parent().expect("instructions have a parent"),
        "sync project instruction root before exchange",
    )
}

#[cfg(unix)]
fn preserve_mode(metadata: &fs::Metadata, paths: &WitnessPaths) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let permissions = fs::Permissions::from_mode(metadata.mode());
    for path in [&paths.original, &paths.reviewed, &paths.exchange] {
        if super::super::injected_storage_failure("preserve project instruction Unix mode") {
            return Err(Error::io(
                "preserve project instruction Unix mode",
                path,
                std::io::Error::other("injected storage failure"),
            ));
        }
        map_io(
            fs::set_permissions(path, permissions.clone()),
            "preserve project instruction Unix mode",
            path,
        )?;
        let file = map_io(File::open(path), "open project instruction witness", path)?;
        map_io(file.sync_all(), "sync project instruction witness", path)?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn preserve_mode(_: &fs::Metadata, _: &WitnessPaths) -> Result<()> {
    ensure_exchange_platform()
}

pub(in crate::workspace) fn verify_witnesses(
    target: &Path,
    paths: &WitnessPaths,
    original: &[u8],
    reviewed: &[u8],
    exchange_bytes: &[u8],
) -> Result<()> {
    if read_bounded_with_limit(&paths.version, MAX_INSTRUCTION_BYTES)? != TRANSACTION_VERSION
        || read_bounded_with_limit(&paths.original, MAX_INSTRUCTION_BYTES)? != original
        || read_bounded_with_limit(&paths.reviewed, MAX_INSTRUCTION_BYTES)? != reviewed
        || read_bounded_with_limit(&paths.exchange, MAX_INSTRUCTION_BYTES)? != exchange_bytes
    {
        return Err(Error::AmbiguousEffect(
            "project instruction transaction witness bytes changed".to_owned(),
        ));
    }
    verify_independent_files(target, paths)
}

#[cfg(unix)]
fn verify_independent_files(target: &Path, paths: &WitnessPaths) -> Result<()> {
    use std::os::unix::fs::MetadataExt;

    let target_metadata = map_io(
        fs::symlink_metadata(target),
        "inspect project instruction mode",
        target,
    )?;
    for path in [&paths.original, &paths.reviewed, &paths.exchange] {
        reject_symlink_chain(path)?;
        let file = map_io(
            fs::symlink_metadata(path),
            "inspect transaction witness",
            path,
        )?;
        if !file.is_file() || file.nlink() != 1 || file.mode() != target_metadata.mode() {
            return Err(Error::AmbiguousEffect(format!(
                "project instruction witness is not independent with the reviewed Unix mode: {}",
                path.display()
            )));
        }
    }
    // Distinct existing paths cannot share an inode while each has link count 1.
    Ok(())
}

#[cfg(not(unix))]
fn verify_independent_files(_: &Path, _: &WitnessPaths) -> Result<()> {
    ensure_exchange_platform()
}

#[cfg(all(coverage, test))]
#[path = "tests/agent_integration_transaction_witnesses_coverage.rs"]
mod coverage_tests;
