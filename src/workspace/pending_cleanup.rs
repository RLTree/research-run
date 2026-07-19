use std::fs;
use std::io;
use std::path::PathBuf;

use crate::{Error, Result};

use super::storage::sync_directory;

pub(super) struct PendingCleanup {
    path: PathBuf,
}

impl PendingCleanup {
    pub(super) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(super) fn remove(&mut self) -> Result<()> {
        if super::injected_storage_failure("remove abandoned pending record") {
            return Err(Error::io(
                "remove abandoned pending record",
                &self.path,
                io::Error::other("injected storage failure"),
            ));
        }
        match fs::remove_file(&self.path) {
            Ok(()) => {
                let parent = self
                    .path
                    .parent()
                    .expect("pending records always have a parent directory");
                if super::injected_storage_failure("sync abandoned pending directory") {
                    return Err(Error::io(
                        "sync abandoned pending directory",
                        parent,
                        io::Error::other("injected storage failure"),
                    ));
                }
                sync_directory(parent)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(Error::io(
                "remove abandoned pending record",
                &self.path,
                error,
            )),
        }
    }

    pub(super) fn after_failure(&mut self, original: Error) -> Error {
        match self.remove() {
            Ok(()) => original,
            Err(cleanup) => Error::AmbiguousEffect(format!(
                "publication failed and pending cleanup was not durable: {original}; {cleanup}"
            )),
        }
    }
}
