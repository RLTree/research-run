use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::Path;

use serde::de::DeserializeOwned;

use crate::{Error, Result};

use super::path_safety::reject_symlink_chain;
#[cfg(any(test, coverage))]
use super::take_storage_failure;
use super::{MAX_RECORD_BYTES, MAX_SNAPSHOT_BYTES, injected_storage_failure};

#[derive(Default)]
pub(super) struct ReadBudget {
    pub(super) bytes: u64,
}

impl ReadBudget {
    pub(super) fn consume(&mut self, path: &Path, bytes: u64) -> Result<()> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| Error::Budget("snapshot byte budget overflowed".to_owned()))?;
        if self.bytes > MAX_SNAPSHOT_BYTES || injected_storage_failure("snapshot byte budget") {
            return Err(Error::Budget(format!(
                "{} would exceed the {MAX_SNAPSHOT_BYTES} byte snapshot budget",
                path.display()
            )));
        }
        Ok(())
    }
}

pub(super) fn read_json_with_budget<T: DeserializeOwned>(
    path: &Path,
    budget: &mut ReadBudget,
) -> Result<T> {
    let bytes = read_bounded_with_budget(path, budget)?;
    parse_json(&bytes, path)
}

pub(super) fn read_json_with_limit<T: DeserializeOwned>(
    path: &Path,
    budget: &mut ReadBudget,
    maximum: u64,
) -> Result<T> {
    let bytes = read_bounded_with_budget_and_limit(path, budget, maximum)?;
    parse_json(&bytes, path)
}

pub(super) fn parse_json<T: DeserializeOwned>(bytes: &[u8], path: &Path) -> Result<T> {
    match serde_json::from_slice(bytes) {
        Ok(value) => Ok(value),
        Err(_) => Err(Error::MalformedJson {
            path: path.to_path_buf(),
        }),
    }
}

pub(super) fn map_io<T>(result: io::Result<T>, action: &'static str, path: &Path) -> Result<T> {
    #[cfg(any(test, coverage))]
    if take_storage_failure(action) {
        return Err(Error::io(
            action,
            path,
            io::Error::other("injected storage failure"),
        ));
    }
    match result {
        Ok(value) => Ok(value),
        Err(error) => Err(Error::io(action, path, error)),
    }
}

pub(super) fn read_bounded_with_budget(path: &Path, budget: &mut ReadBudget) -> Result<Vec<u8>> {
    read_bounded_with_budget_and_limit(path, budget, MAX_RECORD_BYTES)
}

pub(super) fn read_bounded_with_budget_and_limit(
    path: &Path,
    budget: &mut ReadBudget,
    maximum: u64,
) -> Result<Vec<u8>> {
    let bytes = read_bounded_with_limit(path, maximum)?;
    budget.consume(path, bytes.len() as u64)?;
    Ok(bytes)
}

pub(super) fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    read_bounded_with_limit(path, MAX_RECORD_BYTES)
}

pub(super) fn read_bounded_with_limit(path: &Path, maximum: u64) -> Result<Vec<u8>> {
    reject_symlink_chain(path)?;
    let metadata = map_io(fs::symlink_metadata(path), "inspect record", path)?;
    if !metadata.is_file() {
        return Err(Error::invalid(
            "record path",
            format!("{} is not a regular file", path.display()),
        ));
    }
    if metadata.len() > maximum {
        return Err(Error::Budget(format!(
            "{} exceeds the {maximum} byte record budget",
            path.display()
        )));
    }
    inject_record_open_race(path);
    let file = map_io(open_record(path), "open record", path)?;
    let opened_metadata = map_io(file.metadata(), "inspect opened record", path)?;
    if !opened_metadata.is_file() {
        return Err(Error::invalid(
            "record path",
            format!("must open as a regular file: {}", path.display()),
        ));
    }
    if !same_file_identity(&metadata, &opened_metadata)
        || injected_storage_failure("record opened identity")
    {
        return Err(Error::AmbiguousEffect(format!(
            "record identity changed while opening {}",
            path.display()
        )));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    map_io(
        file.take(maximum.saturating_add(1)).read_to_end(&mut bytes),
        "read record",
        path,
    )?;
    if bytes.len() as u64 > maximum || injected_storage_failure("record grew") {
        return Err(Error::Budget(format!(
            "{} grew beyond the record budget while reading",
            path.display()
        )));
    }
    inject_record_symlink_after_read(path, &bytes);
    reject_symlink_chain(path)?;
    let current_metadata = map_io(fs::metadata(path), "reinspect record", path)?;
    if !same_file_identity(&opened_metadata, &current_metadata)
        || injected_storage_failure("record identity")
    {
        return Err(Error::AmbiguousEffect(format!(
            "record identity changed while reading {}",
            path.display()
        )));
    }
    Ok(bytes)
}

fn open_record(path: &Path) -> io::Result<File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(path)
    }
    #[cfg(not(unix))]
    OpenOptions::new().read(true).open(path)
}

#[cfg(all(any(test, coverage), unix))]
fn inject_record_open_race(path: &Path) {
    if take_storage_failure("record open fifo race") {
        fs::remove_file(path).expect("remove record for injected FIFO race");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(path)
                .status()
                .expect("create injected record FIFO")
                .success()
        );
    }
}

#[cfg(not(all(any(test, coverage), unix)))]
fn inject_record_open_race(_path: &Path) {}

#[cfg(all(any(test, coverage), unix))]
fn inject_record_symlink_after_read(path: &Path, bytes: &[u8]) {
    use std::os::unix::fs::symlink;

    if take_storage_failure("record symlink after read") {
        let destination = path.with_extension("post-read");
        fs::write(&destination, bytes).expect("write injected record destination");
        fs::remove_file(path).expect("remove injected record pathname");
        symlink(destination, path).expect("create injected record symlink");
    }
}

#[cfg(not(all(any(test, coverage), unix)))]
fn inject_record_symlink_after_read(_path: &Path, _bytes: &[u8]) {}

#[cfg(unix)]
pub(super) fn same_file_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
pub(super) fn same_file_identity(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.len() == right.len() && left.modified().ok() == right.modified().ok()
}

pub(super) fn sync_directory(path: &Path) -> Result<()> {
    let directory = map_io(File::open(path), "open record directory for sync", path)?;
    map_io(directory.sync_all(), "sync record directory", path)
}
