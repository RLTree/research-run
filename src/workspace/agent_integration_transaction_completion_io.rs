use std::ffi::{OsStr, OsString};
use std::fs::File;
#[cfg(unix)]
use std::io::Read;
use std::path::Path;

#[cfg(unix)]
use rustix::fs::Dir;
#[cfg(unix)]
use rustix::fs::{AtFlags, Mode, OFlags, openat, unlinkat};

use crate::{Error, Result};

#[cfg(unix)]
use super::super::storage::map_io;

pub(super) struct Leaf {
    pub(super) bytes: Vec<u8>,
    pub(super) mode: u32,
    pub(super) links: u64,
    pub(super) device: u64,
    pub(super) inode: u64,
}

#[cfg(unix)]
pub(super) fn read_leaf(directory: &File, name: &OsStr, maximum: u64, path: &Path) -> Result<Leaf> {
    let descriptor = openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| {
        Error::io(
            "open project instruction transaction leaf",
            path,
            error.into(),
        )
    })?;
    let mut file = File::from(descriptor);
    let metadata = map_io(
        file.metadata(),
        "inspect project instruction transaction leaf",
        path,
    )?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(Error::invalid(
            "project instruction transaction leaf",
            "must be one bounded regular file",
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    map_io(
        Read::by_ref(&mut file)
            .take(maximum.saturating_add(1))
            .read_to_end(&mut bytes),
        "read project instruction transaction leaf",
        path,
    )?;
    if bytes.len() as u64 > maximum {
        return Err(Error::Budget(
            "project instruction transaction leaf exceeds its budget".to_owned(),
        ));
    }
    let (mode, links, device, inode) = unix_identity(&metadata)?;
    Ok(Leaf {
        bytes,
        mode,
        links,
        device,
        inode,
    })
}

#[cfg(not(unix))]
pub(super) fn read_leaf(_: &File, _: &OsStr, _: u64, _: &Path) -> Result<Leaf> {
    Err(Error::invalid(
        "project instruction transaction leaf",
        "instruction transaction I/O requires Unix descriptor-relative filesystem support",
    ))
}

#[cfg(unix)]
pub(super) fn list_names(directory: &File, path: &Path) -> Result<Vec<OsString>> {
    use std::os::unix::ffi::OsStrExt;

    let entries = Dir::read_from(directory)
        .map_err(|error| Error::io("inspect committed instruction cleanup", path, error.into()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            Error::io("inspect committed instruction cleanup", path, error.into())
        })?;
        let bytes = entry.file_name().to_bytes();
        if matches!(bytes, b"." | b"..") {
            continue;
        }
        names.push(OsStr::from_bytes(bytes).to_os_string());
    }
    Ok(names)
}

#[cfg(not(unix))]
pub(super) fn list_names(_: &File, _: &Path) -> Result<Vec<OsString>> {
    Err(Error::invalid(
        "instruction completion directory",
        "instruction completion requires Unix directory enumeration",
    ))
}

#[cfg(unix)]
pub(super) fn sync_leaf(
    directory: &File,
    name: &OsStr,
    point: &'static str,
    path: &Path,
) -> Result<()> {
    let descriptor = openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| Error::io("open project instruction leaf for sync", path, error.into()))?;
    let file = File::from(descriptor);
    if !map_io(file.metadata(), "inspect instruction leaf for sync", path)?.is_file() {
        return Err(Error::invalid(
            "project instruction leaf",
            "durability sync requires a regular file",
        ));
    }
    sync_handle(&file, point, path)
}

#[cfg(not(unix))]
pub(super) fn sync_leaf(_: &File, _: &OsStr, _: &'static str, _: &Path) -> Result<()> {
    Err(Error::invalid(
        "project instruction leaf",
        "instruction durability requires Unix descriptor-relative filesystem support",
    ))
}

pub(super) fn sync_handle(file: &File, point: &'static str, path: &Path) -> Result<()> {
    if super::super::injected_storage_failure(point) {
        return Err(injected(point, path));
    }
    file.sync_all()
        .map_err(|error| Error::io(point, path, error))
}

#[cfg(unix)]
pub(super) fn unlink_file(directory: &File, name: &OsStr, action: &'static str) -> Result<()> {
    unlinkat(directory, name, AtFlags::empty())
        .map_err(|error| Error::io(action, Path::new(name), error.into()))
}

#[cfg(not(unix))]
pub(super) fn unlink_file(_: &File, _: &OsStr, _: &'static str) -> Result<()> {
    Err(Error::invalid(
        "project instruction cleanup",
        "instruction cleanup requires Unix descriptor-relative filesystem support",
    ))
}

#[cfg(unix)]
pub(super) fn unlink_directory(
    root: &File,
    name: &OsStr,
    path: &Path,
    action: &'static str,
) -> Result<()> {
    unlinkat(root, name, AtFlags::REMOVEDIR).map_err(|error| Error::io(action, path, error.into()))
}

#[cfg(not(unix))]
pub(super) fn unlink_directory(_: &File, _: &OsStr, _: &Path, _: &'static str) -> Result<()> {
    Err(Error::invalid(
        "project instruction cleanup",
        "instruction cleanup requires Unix descriptor-relative filesystem support",
    ))
}

pub(super) fn leaf<'a>(path: &'a Path, context: &str) -> Result<&'a OsStr> {
    path.file_name()
        .ok_or_else(|| Error::invalid(context, "missing leaf name"))
}

#[cfg(unix)]
fn unix_identity(metadata: &std::fs::Metadata) -> Result<(u32, u64, u64, u64)> {
    use std::os::unix::fs::MetadataExt;

    Ok((
        metadata.mode(),
        metadata.nlink(),
        metadata.dev(),
        metadata.ino(),
    ))
}

fn injected(action: &'static str, path: &Path) -> Error {
    Error::io(
        action,
        path,
        std::io::Error::other("injected storage failure"),
    )
}
