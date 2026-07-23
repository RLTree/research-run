use std::fs;
use std::io;
use std::path::Path;

use crate::domain::{CanonicalRecord, ContributionProtocol, ProjectManifest, ReviewAuthority};
use crate::{Error, Result};

use super::path_safety::{absolute_path, create_directory_chain, reject_symlink_chain};
use super::sshsig::parse_authority_key;
use super::storage::ReadBudget;
use super::write_lock::WorkspaceWriteLock;
use super::{
    CONTRIBUTION_PROTOCOL_DIRECTORY, STATE_DIRECTORY, Workspace, injected_storage_failure,
};

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
            CONTRIBUTION_PROTOCOL_DIRECTORY,
        ] {
            let path = workspace.state.join(directory);
            create_directory_chain(&path)?;
        }
        if manifest_path.is_file() {
            workspace.stage_initialized_authority(authority)?;
            let snapshot = workspace.load_snapshot_for_activation()?;
            verify_initialized_authority_records(authority, &snapshot.review_authorities)?;
            workspace.install_contribution_protocol()?;
            workspace.verify_initialized_authority(authority)?;
            return Ok(workspace);
        }
        workspace.stage_initialized_authority(authority)?;
        workspace.install_contribution_protocol()?;
        workspace.publish_value(&manifest_path, &manifest)?;
        workspace.verify_initialized_authority(authority)?;
        Ok(workspace)
    }

    pub(super) fn install_contribution_protocol(&self) -> Result<bool> {
        self.publish_record(
            CONTRIBUTION_PROTOCOL_DIRECTORY,
            &ContributionProtocol::agent_v1(),
        )
    }

    fn stage_initialized_authority(&self, expected: Option<&ReviewAuthority>) -> Result<()> {
        let mut budget = ReadBudget::default();
        let existing: Vec<ReviewAuthority> =
            self.load_optional_records("review-authorities", false, &mut budget)?;
        match (expected, existing.as_slice()) {
            (Some(_), []) | (None, []) => {}
            (Some(expected), [actual]) if actual == expected => {}
            (Some(_), _) => return Err(initialized_authority_conflict(true)),
            (None, _) => return Err(initialized_authority_conflict(false)),
        }
        if let Some(expected) = expected {
            self.publish_record("review-authorities", expected)?;
        }
        let mut budget = ReadBudget::default();
        let staged: Vec<ReviewAuthority> =
            self.load_optional_records("review-authorities", false, &mut budget)?;
        verify_initialized_authority_records(expected, &staged)
    }

    pub(super) fn verify_initialized_authority(
        &self,
        expected: Option<&ReviewAuthority>,
    ) -> Result<()> {
        let snapshot = self.load_snapshot()?;
        verify_initialized_authority_records(expected, &snapshot.review_authorities)
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

fn verify_initialized_authority_records(
    expected: Option<&ReviewAuthority>,
    actual: &[ReviewAuthority],
) -> Result<()> {
    match (expected, actual) {
        (Some(expected), [actual]) if actual == expected => Ok(()),
        (None, []) => Ok(()),
        (Some(_), _) => Err(initialized_authority_conflict(true)),
        (None, _) => Err(initialized_authority_conflict(false)),
    }
}

fn initialized_authority_conflict(anchored: bool) -> Error {
    if anchored {
        Error::Conflict(
            "initialized workspace does not contain exactly the anchored review authority"
                .to_owned(),
        )
    } else {
        Error::Conflict("unanchored workspace contains an unexpected review authority".to_owned())
    }
}
