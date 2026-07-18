use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use serde::Serialize;

use crate::domain::CanonicalRecord;
use crate::{Error, Result};

use super::path_safety::{ensure_no_pending_effect, reject_symlink_chain};
use super::storage::{map_io, read_bounded, same_file_identity, sync_directory};
#[cfg(any(test, coverage))]
use super::take_storage_failure;
use super::{MAX_RECORD_BYTES, TEMP_SEQUENCE, Workspace, injected_storage_failure};

pub(super) struct WorkspaceWriteLock {
    file: File,
}

impl WorkspaceWriteLock {
    pub(super) fn acquire(state: &Path) -> Result<Self> {
        let path = state.join("write.lock");
        reject_symlink_chain(&path)?;
        let file = map_io(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path),
            "open workspace write lock",
            &path,
        )?;
        map_io(
            file.try_lock().map_err(Into::into),
            "acquire workspace write lock",
            &path,
        )?;
        let opened = map_io(file.metadata(), "inspect workspace write lock", &path)?;
        let current = map_io(fs::metadata(&path), "reinspect workspace write lock", &path)?;
        if !same_file_identity(&opened, &current) || injected_storage_failure("lock identity") {
            return Err(Error::AmbiguousEffect(
                "workspace write lock identity changed during acquisition".to_owned(),
            ));
        }
        Ok(Self { file })
    }
}

impl Drop for WorkspaceWriteLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

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
        self.publish_value(&target, record)
    }

    pub(super) fn publish_value(&self, target: &Path, value: &impl Serialize) -> Result<bool> {
        let mut content = serde_json::to_vec_pretty(value).expect(
            "Research Run domain records contain only JSON-representable strings, numbers, lists, and enums",
        );
        content.push(b'\n');
        self.publish_bytes(target, &content)
    }

    pub(super) fn publish_bytes(&self, target: &Path, content: &[u8]) -> Result<bool> {
        if let Some(created) = inspect_publication(target, content)? {
            return Ok(created);
        }
        let temporary = pending_path(target);
        let mut cleanup = PendingCleanup::new(temporary.clone());
        write_pending(&temporary, content)?;
        if !link_canonical(&temporary, target, content)? {
            return Ok(false);
        }
        finish_publication(&temporary, target, &mut cleanup)?;
        cleanup.disarm();
        Ok(true)
    }
}

fn inspect_publication(target: &Path, content: &[u8]) -> Result<Option<bool>> {
    reject_symlink_chain(target)?;
    if content.len() as u64 > MAX_RECORD_BYTES {
        return Err(Error::Budget(format!(
            "record exceeds the {MAX_RECORD_BYTES} byte budget"
        )));
    }
    if !target.exists() {
        ensure_no_pending_effect(target)?;
        return Ok(None);
    }
    if read_bounded(target)? == content {
        return Ok(Some(false));
    }
    Err(Error::Conflict(format!(
        "record identity already exists with different content: {}",
        target.display()
    )))
}

fn pending_path(target: &Path) -> PathBuf {
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

fn write_pending(path: &Path, content: &[u8]) -> Result<()> {
    let mut file = map_io(
        OpenOptions::new().write(true).create_new(true).open(path),
        "create pending record",
        path,
    )?;
    map_io(file.write_all(content), "write pending record", path)?;
    map_io(file.sync_all(), "sync pending record", path)
}

fn link_canonical(temporary: &Path, target: &Path, content: &[u8]) -> Result<bool> {
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
            if read_bounded(target)? == content {
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

fn finish_publication(temporary: &Path, target: &Path, cleanup: &mut PendingCleanup) -> Result<()> {
    let parent = target
        .parent()
        .expect("pending_path proved the canonical target has a parent");
    if let Err(error) = sync_directory(parent) {
        cleanup.preserve();
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
        cleanup.preserve();
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
}

#[cfg(not(any(test, coverage)))]
fn inject_publication_race(_target: &Path, _content: &[u8]) {}

pub(super) struct PendingCleanup {
    path: PathBuf,
    remove: bool,
}

impl PendingCleanup {
    pub(super) fn new(path: PathBuf) -> Self {
        Self { path, remove: true }
    }

    pub(super) fn preserve(&mut self) {
        self.remove = false;
    }

    pub(super) fn disarm(&mut self) {
        self.remove = false;
    }
}

impl Drop for PendingCleanup {
    fn drop(&mut self) {
        if self.remove {
            let _ = fs::remove_file(&self.path);
        }
    }
}
