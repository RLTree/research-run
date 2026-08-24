use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::{Error, Result};

use super::super::agent_integration_transaction::directories::{open_directory, verify_anchor};
use super::super::storage::map_io;

pub(in crate::workspace) fn create_private_file(directory: &File, path: &Path) -> Result<File> {
    if super::super::injected_storage_failure("create pending record") {
        return Err(injected("create pending record", path));
    }
    let file = open_private_file(directory, path)?;
    verify_private_file(&file, path, 0o600)?;
    Ok(file)
}

pub(in crate::workspace) fn write_private_pending(
    parent: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<()> {
    let directory = open_directory(parent)?;
    verify_anchor(parent, &directory)?;
    write_private_pending_at(&directory, path, bytes)
}

fn write_private_pending_at(directory: &File, path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = create_private_file(directory, path)?;
    set_unix_mode(&file, path, 0o600)?;
    write_private_bytes(&mut file, path, bytes)?;
    verify_private_file(&file, path, 0o600)?;
    sync_private_file(&file, path)
}

pub(in crate::workspace) fn write_private_bytes(
    file: &mut File,
    path: &Path,
    bytes: &[u8],
) -> Result<()> {
    map_io(file.write_all(bytes), "write pending record", path)
}

pub(in crate::workspace) fn sync_private_file(file: &File, path: &Path) -> Result<()> {
    map_io(file.sync_all(), "sync pending record", path)
}

#[cfg(unix)]
pub(in crate::workspace) fn set_unix_mode(file: &File, path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let expected = mode & 0o7777;
    map_io(
        file.set_permissions(std::fs::Permissions::from_mode(expected)),
        "set private pending record mode",
        path,
    )?;
    let actual = map_io(file.metadata(), "inspect private pending record", path)?.mode() & 0o7777;
    if actual != expected {
        return Err(Error::Conflict(format!(
            "private pending record mode changed at {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
pub(in crate::workspace) fn set_unix_mode(_: &File, _: &Path, _: u32) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn open_private_file(directory: &File, path: &Path) -> Result<File> {
    use rustix::fs::{Mode, OFlags, openat};

    let name = leaf(path)?;
    openat(
        directory,
        name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )
    .map(File::from)
    .map_err(|error| Error::io("open pending record", path, error.into()))
}

#[cfg(not(unix))]
fn open_private_file(_: &File, path: &Path) -> Result<File> {
    map_io(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path),
        "open pending record",
        path,
    )
}

#[cfg(unix)]
fn verify_private_file(file: &File, path: &Path, ceiling: u32) -> Result<()> {
    use std::os::unix::fs::MetadataExt;

    let metadata = map_io(file.metadata(), "inspect private pending record", path)?;
    let permissions = metadata.mode() & 0o7777;
    if !metadata.is_file() || metadata.nlink() != 1 || permissions & !ceiling != 0 {
        return Err(Error::Conflict(format!(
            "private pending record has an unsafe identity or mode at {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn verify_private_file(file: &File, path: &Path, _: u32) -> Result<()> {
    if !map_io(file.metadata(), "inspect private pending record", path)?.is_file() {
        return Err(Error::Conflict(format!(
            "private pending record is not a file at {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn leaf(path: &Path) -> Result<&std::ffi::OsStr> {
    path.file_name()
        .ok_or_else(|| Error::invalid("private pending record", "missing leaf name"))
}

fn injected(action: &'static str, path: &Path) -> Error {
    Error::io(
        action,
        path,
        std::io::Error::other("injected storage failure"),
    )
}
