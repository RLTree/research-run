use std::fs::{self, File};
use std::path::Path;

use crate::{Error, Result};

#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::super::path_safety::reject_symlink_chain;
use super::super::storage::{map_io, same_file_identity};
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use super::atomic_exchange::ensure_exchange_platform;

pub(in crate::workspace) struct ExchangeHandles {
    pub(in crate::workspace) root: File,
    pub(in crate::workspace) transaction: File,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(in crate::workspace) fn open_exchange_handles(
    target: &Path,
    transaction: &Path,
) -> Result<ExchangeHandles> {
    let root = target.parent().expect("instructions have a parent");
    reject_symlink_chain(root)?;
    reject_symlink_chain(transaction)?;
    let root_handle = open_directory(root)?;
    verify_anchor(root, &root_handle)?;
    open_exchange_handles_from_root(target, transaction, root_handle)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(in crate::workspace) fn open_exchange_handles(
    _target: &Path,
    _transaction: &Path,
) -> Result<ExchangeHandles> {
    ensure_exchange_platform()?;
    unreachable!("unsupported platforms fail before opening exchange handles")
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(in crate::workspace) fn open_exchange_handles_from_root(
    target: &Path,
    transaction: &Path,
    root_handle: File,
) -> Result<ExchangeHandles> {
    let transaction_handle = open_directory_at(&root_handle, transaction)?;
    verify_anchor(transaction, &transaction_handle)?;
    let root = target.parent().expect("instructions have a parent");
    verify_anchor(root, &root_handle)?;
    ensure_same_device(root, &root_handle, transaction, &transaction_handle)?;
    Ok(ExchangeHandles {
        root: root_handle,
        transaction: transaction_handle,
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(in crate::workspace) fn open_exchange_handles_from_root(
    _target: &Path,
    _transaction: &Path,
    _root_handle: File,
) -> Result<ExchangeHandles> {
    ensure_exchange_platform()?;
    unreachable!("unsupported platforms fail before opening exchange handles")
}

#[cfg(unix)]
pub(in crate::workspace) fn open_directory(path: &Path) -> Result<File> {
    use rustix::fs::{Mode, OFlags, open};

    let descriptor = open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| {
        Error::io(
            "open project instruction directory without following symlinks",
            path,
            error.into(),
        )
    })?;
    let file = File::from(descriptor);
    let metadata = map_io(
        file.metadata(),
        "inspect project instruction directory",
        path,
    )?;
    if !metadata.is_dir() {
        return Err(Error::invalid(
            "project instruction directory",
            format!("{} is not a directory", path.display()),
        ));
    }
    Ok(file)
}

#[cfg(not(unix))]
pub(in crate::workspace) fn open_directory(_: &Path) -> Result<File> {
    ensure_exchange_platform()?;
    unreachable!("unsupported platforms fail before opening directories")
}

pub(in crate::workspace) fn verify_anchor(path: &Path, handle: &File) -> Result<()> {
    let current = map_io(
        fs::symlink_metadata(path),
        "inspect project instruction directory anchor",
        path,
    )?;
    let opened = map_io(
        handle.metadata(),
        "inspect opened project instruction directory",
        path,
    )?;
    if !current.is_dir() || !same_file_identity(&current, &opened) {
        return Err(Error::AmbiguousEffect(format!(
            "project instruction directory identity changed at {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(unix)]
pub(in crate::workspace) fn verify_relative_directory_anchor(
    root: &File,
    path: &Path,
    handle: &File,
) -> Result<()> {
    let current = open_directory_at(root, path)?;
    let current_metadata = map_io(
        current.metadata(),
        "inspect current project instruction transaction directory",
        path,
    )?;
    let opened_metadata = map_io(
        handle.metadata(),
        "inspect held project instruction transaction directory",
        path,
    )?;
    if !same_file_identity(&current_metadata, &opened_metadata) {
        return Err(Error::AmbiguousEffect(format!(
            "project instruction transaction identity changed at {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
pub(in crate::workspace) fn verify_relative_directory_anchor(
    _: &File,
    _: &Path,
    _: &File,
) -> Result<()> {
    ensure_exchange_platform()
}

#[cfg(unix)]
fn open_directory_at(root: &File, path: &Path) -> Result<File> {
    use rustix::fs::{Mode, OFlags, openat};

    let name = path.file_name().ok_or_else(|| {
        Error::invalid(
            "project instruction directory",
            "transaction path must have one leaf name",
        )
    })?;
    let descriptor = openat(
        root,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| {
        Error::io(
            "open project instruction transaction without following symlinks",
            path,
            error.into(),
        )
    })?;
    let file = File::from(descriptor);
    if !map_io(
        file.metadata(),
        "inspect project instruction transaction directory",
        path,
    )?
    .is_dir()
    {
        return Err(Error::invalid(
            "project instruction transaction",
            "must be a directory",
        ));
    }
    Ok(file)
}

#[cfg(unix)]
pub(in crate::workspace) fn ensure_same_device(
    root: &Path,
    root_handle: &File,
    transaction: &Path,
    transaction_handle: &File,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;

    let root_metadata = map_io(root_handle.metadata(), "inspect project directory", root)?;
    let transaction_metadata = map_io(
        transaction_handle.metadata(),
        "inspect project instruction transaction",
        transaction,
    )?;
    if root_metadata.dev() != transaction_metadata.dev() {
        return Err(Error::invalid(
            "project instruction atomic exchange",
            "canonical and exchange entries are on different devices",
        ));
    }
    Ok(())
}
