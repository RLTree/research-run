use std::fs::{self, File, OpenOptions};
use std::path::Path;

use crate::{Error, Result};

use super::injected_storage_failure;
use super::path_safety::reject_symlink_chain;
use super::storage::{map_io, same_file_identity};

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
