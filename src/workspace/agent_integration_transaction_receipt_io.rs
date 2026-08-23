use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use rustix::fs::{AtFlags, Mode, OFlags, linkat, openat};
use rustix::io::Errno;

use crate::{Error, Result};

use super::super::storage::map_io;
use super::completion_io::{leaf, read_leaf, sync_handle, sync_leaf, unlink_file};
use super::receipt::MAX_COMPLETION_RECEIPT_BYTES;

pub(super) const COMPLETION_WITNESS: &str = "completion";

pub(super) fn stage_receipt(
    transaction: &File,
    transaction_path: &Path,
    bytes: &[u8],
) -> Result<()> {
    let path = transaction_path.join(COMPLETION_WITNESS);
    match open_staged_receipt(transaction) {
        Ok(file) => write_staged_receipt(file, &path, bytes)?,
        Err(Errno::EXIST) => {
            repair_or_reuse_staged_receipt(transaction, transaction_path, &path, bytes)?
        }
        Err(error) => {
            return Err(Error::io(
                "create project instruction completion receipt",
                &path,
                error.into(),
            ));
        }
    }
    sync_handle(
        transaction,
        "sync project instruction transaction after completion staging",
        transaction_path,
    )
}

pub(super) fn publish_receipt(transaction: &File, root: &File, receipt_path: &Path) -> Result<()> {
    if super::super::injected_storage_failure("publish project instruction completion receipt") {
        return Err(injected(
            "publish project instruction completion receipt",
            receipt_path,
        ));
    }
    linkat(
        transaction,
        COMPLETION_WITNESS,
        root,
        leaf(receipt_path, "completion receipt")?,
        AtFlags::empty(),
    )
    .map_err(|error| {
        Error::io(
            "publish project instruction completion receipt",
            receipt_path,
            error.into(),
        )
    })?;
    sync_leaf(
        root,
        leaf(receipt_path, "completion receipt")?,
        "sync published project instruction completion receipt",
        receipt_path,
    )
}

fn open_staged_receipt(transaction: &File) -> std::result::Result<File, Errno> {
    if super::super::injected_storage_failure("create project instruction completion receipt") {
        return Err(Errno::IO);
    }
    openat(
        transaction,
        COMPLETION_WITNESS,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map(File::from)
}

fn write_staged_receipt(mut file: File, path: &Path, bytes: &[u8]) -> Result<()> {
    if super::super::injected_storage_failure("write project instruction completion receipt") {
        return Err(injected(
            "write project instruction completion receipt",
            path,
        ));
    }
    map_io(
        file.write_all(bytes),
        "write project instruction completion receipt",
        path,
    )?;
    set_receipt_mode(&file, path)?;
    if super::super::injected_storage_failure("sync project instruction completion receipt") {
        return Err(injected(
            "sync project instruction completion receipt",
            path,
        ));
    }
    map_io(
        file.sync_all(),
        "sync project instruction completion receipt",
        path,
    )
}

fn repair_or_reuse_staged_receipt(
    transaction: &File,
    transaction_path: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<()> {
    let existing = read_leaf(
        transaction,
        OsStr::new(COMPLETION_WITNESS),
        MAX_COMPLETION_RECEIPT_BYTES,
        path,
    )?;
    if existing.bytes == bytes && existing.mode & 0o777 == 0o600 && existing.links == 1 {
        return sync_leaf(
            transaction,
            OsStr::new(COMPLETION_WITNESS),
            "sync project instruction completion receipt",
            path,
        );
    }
    if existing.links != 1 {
        return Err(Error::Conflict(
            "staged completion receipt is not one independent file".to_owned(),
        ));
    }
    unlink_file(
        transaction,
        OsStr::new(COMPLETION_WITNESS),
        "remove incomplete project instruction completion receipt",
    )?;
    sync_handle(
        transaction,
        "sync project instruction transaction after receipt repair",
        transaction_path,
    )?;
    let file = open_staged_receipt(transaction).map_err(|error| {
        Error::io(
            "recreate project instruction completion receipt",
            path,
            error.into(),
        )
    })?;
    write_staged_receipt(file, path, bytes)
}

#[cfg(unix)]
fn set_receipt_mode(file: &File, path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    map_io(
        file.set_permissions(std::fs::Permissions::from_mode(0o600)),
        "set project instruction completion receipt mode",
        path,
    )
}

#[cfg(not(unix))]
fn set_receipt_mode(_: &File, _: &Path) -> Result<()> {
    Err(Error::invalid(
        "completion receipt mode",
        "instruction completion requires Unix mode metadata",
    ))
}

fn injected(action: &'static str, path: &Path) -> Error {
    Error::io(
        action,
        path,
        std::io::Error::other("injected storage failure"),
    )
}
