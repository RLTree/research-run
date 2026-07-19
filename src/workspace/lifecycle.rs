use std::fs;
use std::io;
use std::path::Path;

use crate::domain::ProjectManifest;
use crate::{Error, Result};

use super::path_safety::{absolute_path, create_directory_chain, reject_symlink_chain};
use super::write_lock::WorkspaceWriteLock;
use super::{STATE_DIRECTORY, Workspace, injected_storage_failure};

impl Workspace {
    pub fn initialize(root: &Path, name: &str) -> Result<Self> {
        let manifest = ProjectManifest::new(name)?;
        let root = absolute_path(root)?;
        reject_symlink_chain(&root)?;
        create_directory_chain(&root)?;
        let workspace = Self {
            state: root.join(STATE_DIRECTORY),
            root,
        };
        create_directory_chain(&workspace.state)?;
        for directory in ["sources", "claims", "evidence", "experiments", "reviews"] {
            let path = workspace.state.join(directory);
            create_directory_chain(&path)?;
        }
        let _write_lock = WorkspaceWriteLock::acquire(&workspace.state)?;
        workspace.publish_value(&workspace.state.join("manifest.json"), &manifest)?;
        Ok(workspace)
    }

    pub fn discover(start: &Path) -> Result<Self> {
        let start = absolute_path(start)?;
        reject_symlink_chain(&start)?;
        for candidate in start.ancestors() {
            let state = candidate.join(STATE_DIRECTORY);
            reject_symlink_chain(&state)?;
            let metadata = if injected_storage_failure("inspect workspace boundary") {
                Err(io::Error::other("injected storage failure"))
            } else {
                fs::symlink_metadata(&state)
            };
            match metadata {
                Ok(_) => {
                    let manifest = state.join("manifest.json");
                    reject_symlink_chain(&manifest)?;
                    let workspace = Self {
                        root: candidate.to_path_buf(),
                        state,
                    };
                    workspace.read_manifest()?;
                    return Ok(workspace);
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(Error::io("inspect workspace boundary", state, error)),
            }
        }
        Err(Error::NotFound(
            "no Research Run workspace found; run 'research-run init' first".to_owned(),
        ))
    }

    pub fn for_recovery(root: &Path) -> Result<Self> {
        let root = absolute_path(root)?;
        reject_symlink_chain(&root)?;
        let state = root.join(STATE_DIRECTORY);
        reject_symlink_chain(&state)?;
        if !state.is_dir() {
            return Err(Error::NotFound(format!(
                "no partial or complete Research Run workspace at {}",
                root.display()
            )));
        }
        Ok(Self { root, state })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}
