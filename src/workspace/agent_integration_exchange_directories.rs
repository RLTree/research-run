use std::fs::{self, File};
use std::path::Path;

use crate::{Error, Result};

use super::super::path_safety::reject_symlink_chain;
use super::super::storage::sync_directory;
use super::super::storage::{map_io, same_file_identity};
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use super::atomic_exchange::ensure_exchange_platform;

pub(in crate::workspace) struct ExchangeHandles {
    pub(in crate::workspace) root: File,
    pub(in crate::workspace) transaction: File,
}

#[cfg(unix)]
pub(in crate::workspace) fn create_transaction_directory(path: &Path) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};

    let parent = path.parent().expect("transaction has a parent");
    let parent_metadata = map_io(
        fs::symlink_metadata(parent),
        "inspect project instruction transaction parent",
        parent,
    )?;
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    map_io(
        builder.create(path),
        "create project instruction transaction",
        path,
    )?;
    let metadata = map_io(
        fs::symlink_metadata(path),
        "inspect project instruction transaction permissions",
        path,
    )?;
    if !transaction_directory_permissions_are_private(
        parent_metadata.mode(),
        parent_metadata.gid(),
        metadata.mode(),
        metadata.gid(),
        metadata.is_dir(),
    ) {
        return Err(Error::Conflict(format!(
            "project instruction transaction permissions or inherited identity are invalid: {}",
            path.display()
        )));
    }
    sync_directory(parent)
}

#[cfg(unix)]
pub(in crate::workspace) fn transaction_directory_permissions_are_private(
    parent_mode: u32,
    parent_gid: u32,
    transaction_mode: u32,
    transaction_gid: u32,
    is_directory: bool,
) -> bool {
    if !is_directory || transaction_mode & 0o077 != 0 {
        return false;
    }
    let special = transaction_mode & 0o7000;
    // Linux mkdir inherits S_ISGID from a setgid parent without granting access.
    #[cfg(target_os = "linux")]
    let inherited_setgid =
        special == 0o2000 && parent_mode & 0o2000 != 0 && transaction_gid == parent_gid;
    #[cfg(not(target_os = "linux"))]
    let inherited_setgid = {
        let _ = (parent_mode, parent_gid, transaction_gid);
        false
    };
    special == 0 || inherited_setgid
}

#[cfg(not(unix))]
pub(in crate::workspace) fn create_transaction_directory(_: &Path) -> Result<()> {
    ensure_exchange_platform()
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
    ensure_exchange_platform()
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
    ensure_exchange_platform()
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

#[cfg(not(unix))]
fn open_directory_at(_: &File, _: &Path) -> Result<File> {
    ensure_exchange_platform()?;
    unreachable!("unsupported platforms fail before opening directories")
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

#[cfg(not(unix))]
pub(in crate::workspace) fn ensure_same_device(
    _: &Path,
    _: &File,
    _: &Path,
    _: &File,
) -> Result<()> {
    ensure_exchange_platform()
}
