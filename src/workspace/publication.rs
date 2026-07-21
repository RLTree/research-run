use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use serde::Serialize;

use crate::domain::CanonicalRecord;
use crate::{Error, Result};

use super::path_safety::{ensure_no_pending_effect, reject_symlink_chain};
use super::pending_cleanup::PendingCleanup;
use super::storage::{map_io, read_bounded_with_limit, sync_directory};
#[cfg(any(test, coverage))]
use super::take_storage_failure;
use super::{
    MAX_RECORD_BYTES, TEMP_SEQUENCE, Workspace, injected_storage_failure, record_byte_limit,
};

impl Workspace {
    pub(super) fn publish_record<T: CanonicalRecord>(
        &self,
        directory: &str,
        record: &T,
    ) -> Result<bool> {
        let target = self
            .state
            .join(directory)
            .join(format!("{}.json", record.id()));
        self.publish_value_with_limit(&target, record, record_byte_limit(directory))
    }

    pub(super) fn publish_value(&self, target: &Path, value: &impl Serialize) -> Result<bool> {
        let content = canonical_json_bytes(value);
        self.publish_bytes(target, &content)
    }

    fn publish_value_with_limit(
        &self,
        target: &Path,
        value: &impl Serialize,
        maximum: u64,
    ) -> Result<bool> {
        let content = canonical_json_bytes(value);
        self.publish_bytes_with_limit(target, &content, maximum)
    }

    pub(super) fn publish_bytes(&self, target: &Path, content: &[u8]) -> Result<bool> {
        self.publish_bytes_with_limit(target, content, MAX_RECORD_BYTES)
    }

    fn publish_bytes_with_limit(
        &self,
        target: &Path,
        content: &[u8],
        maximum: u64,
    ) -> Result<bool> {
        if !publication_needed(target, content, maximum)? {
            return Ok(false);
        }
        let temporary = pending_path(target);
        let mut cleanup = PendingCleanup::new(temporary.clone());
        if let Err(error) = write_pending(&temporary, content) {
            return Err(cleanup.after_failure(error));
        }
        let created = match link_canonical(&temporary, target, content, maximum) {
            Ok(created) => created,
            Err(error) => return Err(cleanup.after_failure(error)),
        };
        if !created {
            cleanup.remove()?;
            return Ok(false);
        }
        finish_publication(&temporary, target)?;
        Ok(true)
    }
}

pub(super) fn canonical_json_bytes(value: &impl Serialize) -> Vec<u8> {
    let mut content = serde_json::to_vec_pretty(value).expect(
        "Research Run domain records contain only JSON-representable strings, numbers, lists, and enums",
    );
    content.push(b'\n');
    content
}

fn publication_needed(target: &Path, content: &[u8], maximum: u64) -> Result<bool> {
    inject_inspection_race(target, content);
    reject_symlink_chain(target)?;
    if content.len() as u64 > maximum {
        return Err(Error::Budget(format!(
            "record exceeds the {maximum} byte budget"
        )));
    }
    if !target.exists() {
        ensure_no_pending_effect(target)?;
        return Ok(true);
    }
    if read_bounded_with_limit(target, maximum)? == content {
        return Ok(false);
    }
    Err(Error::Conflict(format!(
        "record identity already exists with different content: {}",
        target.display()
    )))
}

#[cfg(all(any(test, coverage), unix))]
fn inject_inspection_race(target: &Path, content: &[u8]) {
    use std::os::unix::fs::symlink;

    if take_storage_failure("inspect publication symlink race") {
        let destination = target.with_extension("race-destination");
        fs::write(&destination, content).expect("write injected publication destination");
        symlink(destination, target).expect("create injected publication symlink");
    }
    if take_storage_failure("inspect publication pending race") {
        let name = target
            .file_name()
            .and_then(OsStr::to_str)
            .expect("canonical target name");
        let pending = target
            .parent()
            .expect("canonical target parent")
            .join(format!(".{name}.race.0.tmp"));
        fs::write(pending, content).expect("write injected pending publication");
    }
    if take_storage_failure("inspect publication unreadable race") {
        fs::create_dir(target).expect("create injected unreadable publication target");
    }
    if std::env::var("RESEARCH_RUN_COVERAGE_FAULT").as_deref() == Ok("inspect pending effect") {
        fs::write(
            target
                .parent()
                .expect("canonical target parent")
                .join("publication-entry"),
            b"entry",
        )
        .expect("write injected publication directory entry");
    }
}

#[cfg(not(all(any(test, coverage), unix)))]
fn inject_inspection_race(_target: &Path, _content: &[u8]) {}

pub(super) fn pending_path(target: &Path) -> PathBuf {
    let parent = target
        .parent()
        .expect("canonical publication targets are rooted in the workspace");
    let file_name = target
        .file_name()
        .and_then(OsStr::to_str)
        .expect("validated record identifiers produce UTF-8 filenames");
    let nonce = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(".{file_name}.{}.{}.tmp", std::process::id(), nonce))
}

pub(super) fn write_pending(path: &Path, content: &[u8]) -> Result<()> {
    if injected_storage_failure("create pending record") {
        return Err(Error::io(
            "create pending record",
            path,
            io::Error::other("injected storage failure"),
        ));
    }
    let mut file = map_io(
        OpenOptions::new().write(true).create_new(true).open(path),
        "open pending record",
        path,
    )?;
    map_io(file.write_all(content), "write pending record", path)?;
    map_io(file.sync_all(), "sync pending record", path)
}

fn link_canonical(temporary: &Path, target: &Path, content: &[u8], maximum: u64) -> Result<bool> {
    inject_pending_cleanup_shape(temporary);
    inject_publication_race(target, content);
    if injected_storage_failure("publish canonical record") {
        return Err(Error::io(
            "publish canonical record",
            target,
            io::Error::other("injected storage failure"),
        ));
    }
    let publication = if injected_storage_failure("hard link canonical record") {
        Err(io::Error::other("injected storage failure"))
    } else {
        fs::hard_link(temporary, target)
    };
    match publication {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if read_bounded_with_limit(target, maximum)? == content {
                Ok(false)
            } else {
                Err(Error::Conflict(format!(
                    "record identity appeared during publication: {}",
                    target.display()
                )))
            }
        }
        Err(error) => Err(Error::io("publish canonical record", target, error)),
    }
}

#[cfg(coverage)]
fn inject_pending_cleanup_shape(temporary: &Path) {
    if std::env::var("RESEARCH_RUN_COVERAGE_FAULT").as_deref()
        == Ok("abandoned pending path is directory")
    {
        fs::remove_file(temporary).expect("remove injected pending file");
        fs::create_dir(temporary).expect("create injected pending directory");
    }
}

#[cfg(not(coverage))]
fn inject_pending_cleanup_shape(_temporary: &Path) {}

fn finish_publication(temporary: &Path, target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .expect("pending_path proved the canonical target has a parent");
    if let Err(error) = sync_directory(parent) {
        return Err(Error::AmbiguousEffect(format!(
            "record may be published at {}; directory sync failed: {error}",
            target.display()
        )));
    }
    if let Err(error) = map_io(
        fs::remove_file(temporary),
        "remove pending record",
        temporary,
    ) {
        return Err(Error::AmbiguousEffect(format!(
            "record was published at {} but pending file cleanup failed: {error}",
            target.display()
        )));
    }
    sync_directory(parent)
}

#[cfg(any(test, coverage))]
fn inject_publication_race(target: &Path, content: &[u8]) {
    if take_storage_failure("publish identical race") {
        fs::write(target, content).expect("write injected identical publication");
    }
    if take_storage_failure("publish conflicting race") {
        fs::write(target, b"conflict").expect("write injected conflicting publication");
    }
    if take_storage_failure("publish unreadable race") {
        fs::create_dir(target).expect("create injected unreadable publication target");
    }
    if matches!(
        std::env::var("RESEARCH_RUN_COVERAGE_FAULT").as_deref(),
        Ok("remove abandoned pending record" | "sync abandoned pending directory")
    ) {
        fs::write(target, content).expect("write injected cleanup publication");
    }
}

#[cfg(not(any(test, coverage)))]
fn inject_publication_race(_target: &Path, _content: &[u8]) {}
