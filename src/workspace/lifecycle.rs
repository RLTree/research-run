use std::fs;
use std::io;
use std::path::Path;

use crate::domain::{CanonicalRecord, ProjectManifest, ReviewAuthority};
use crate::{Error, Result};

use super::path_safety::{absolute_path, create_directory_chain, reject_symlink_chain};
use super::sshsig::parse_authority_key;
use super::write_lock::WorkspaceWriteLock;
use super::{STATE_DIRECTORY, Workspace, injected_storage_failure};

impl Workspace {
    pub fn initialize(root: &Path, name: &str) -> Result<Self> {
        Self::initialize_inner(root, name, None, None)
    }

    pub fn initialize_with_review_authority(
        root: &Path,
        name: &str,
        authority: &ReviewAuthority,
    ) -> Result<Self> {
        authority.validate()?;
        parse_authority_key(authority)?;
        Self::initialize_inner(root, name, Some(authority), None)
    }

    pub(super) fn initialize_for_inventory(
        root: &Path,
        name: &str,
        authority: Option<&ReviewAuthority>,
        workspace_id: &str,
    ) -> Result<Self> {
        if let Some(authority) = authority {
            authority.validate()?;
            parse_authority_key(authority)?;
        }
        Self::initialize_inner(root, name, authority, Some(workspace_id))
    }

    fn initialize_inner(
        root: &Path,
        name: &str,
        authority: Option<&ReviewAuthority>,
        workspace_id: Option<&str>,
    ) -> Result<Self> {
        let mut manifest = ProjectManifest::new(name)?;
        if let Some(workspace_id) = workspace_id {
            manifest.workspace_id = workspace_id.to_owned();
        }
        if let Some(authority) = authority {
            manifest.anchor_review_authority(&authority.id, &authority.fingerprint);
        }
        manifest.validate()?;
        let root = absolute_path(root)?;
        reject_symlink_chain(&root)?;
        create_directory_chain(&root)?;
        let workspace = Self {
            state: root.join(STATE_DIRECTORY),
            root,
        };
        create_directory_chain(&workspace.state)?;
        let _write_lock = WorkspaceWriteLock::acquire(&workspace.state)?;
        let manifest_path = workspace.state.join("manifest.json");
        if manifest_path.is_file() {
            let existing = workspace.read_manifest()?;
            let expected_anchor = authority.map(|value| (&value.id, &value.fingerprint));
            let existing_anchor = existing
                .review_authority_id
                .as_ref()
                .zip(existing.review_authority_fingerprint.as_ref());
            let workspace_id_conflicts =
                workspace_id.is_some_and(|expected| existing.workspace_id.as_str() != expected);
            if existing.name != manifest.name
                || existing_anchor != expected_anchor
                || workspace_id_conflicts
            {
                return Err(Error::Conflict(
                    "workspace identity or review authority conflicts with the existing manifest"
                        .to_owned(),
                ));
            }
        }
        for directory in [
            "sources",
            "claims",
            "evidence",
            "experiments",
            "reviews",
            "review-authorities",
            "inventories",
            "knowledge",
            "relationships",
            "migrations",
        ] {
            let path = workspace.state.join(directory);
            create_directory_chain(&path)?;
        }
        if manifest_path.is_file() {
            if let Some(authority) = authority {
                workspace.publish_record("review-authorities", authority)?;
            }
            workspace.verify_initialized_authority(authority)?;
            return Ok(workspace);
        }
        workspace.publish_value(&manifest_path, &manifest)?;
        if let Some(authority) = authority {
            workspace.publish_record("review-authorities", authority)?;
        }
        workspace.verify_initialized_authority(authority)?;
        Ok(workspace)
    }

    pub(super) fn verify_initialized_authority(
        &self,
        expected: Option<&ReviewAuthority>,
    ) -> Result<()> {
        let snapshot = self.load_snapshot()?;
        match (expected, snapshot.review_authorities.as_slice()) {
            (Some(expected), [actual]) if actual == expected => Ok(()),
            (None, []) => Ok(()),
            (Some(_), _) => Err(Error::Conflict(
                "initialized workspace does not contain exactly the anchored review authority"
                    .to_owned(),
            )),
            (None, _) => Err(Error::Conflict(
                "unanchored workspace contains an unexpected review authority".to_owned(),
            )),
        }
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
