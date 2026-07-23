use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::domain::{FORMAT_VERSION, MigrationPlan, MigrationRecord};
use crate::{Error, Result};

use super::path_safety::{absolute_path, create_directory_chain, reject_symlink_chain};
use super::storage::{ReadBudget, map_io, read_bounded, read_json_with_budget};
use super::write_lock::WorkspaceWriteLock;
use super::{Workspace, injected_storage_failure};

impl Workspace {
    pub fn plan_migration(root: &Path, id: &str, migrated_at: &str) -> Result<MigrationPlan> {
        let root = absolute_path(root)?;
        let workspace = Self::at_exact_root(&root)?.ok_or_else(|| {
            Error::NotFound("migration requires an existing v0.1 workspace".to_owned())
        })?;
        let _write_lock = WorkspaceWriteLock::acquire(&workspace.state)?;
        if injected_storage_failure("migration fingerprint under lock") {
            return Err(Error::AmbiguousEffect(
                "migration authority recheck interrupted after lock acquisition".to_owned(),
            ));
        }
        let manifest = workspace.read_manifest()?;
        let (authority_files, authority_bytes, authority_sha256) =
            workspace.authority_fingerprint()?;
        let plan = MigrationPlan {
            schema_version: FORMAT_VERSION,
            kind: "migration-plan".to_owned(),
            id: id.to_owned(),
            project_id: manifest.project_id,
            from_format: "research-run-v0.1".to_owned(),
            to_format: "research-run-v0.1-extended".to_owned(),
            migrated_at: migrated_at.to_owned(),
            authority_files,
            authority_bytes,
            authority_sha256,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn apply_migration(root: &Path, plan: MigrationPlan) -> Result<bool> {
        plan.validate()?;
        let root = absolute_path(root)?;
        let workspace = Self::at_exact_root(&root)?.ok_or_else(|| {
            Error::NotFound("migration requires an existing v0.1 workspace".to_owned())
        })?;
        let _write_lock = WorkspaceWriteLock::acquire(&workspace.state)?;
        if injected_storage_failure("migration fingerprint under lock") {
            return Err(Error::AmbiguousEffect(
                "migration authority recheck interrupted after lock acquisition".to_owned(),
            ));
        }
        let manifest = workspace.read_manifest()?;
        if manifest.project_id != plan.project_id {
            return Err(Error::Conflict(
                "migration project identity does not match target workspace".to_owned(),
            ));
        }
        let current = workspace.authority_fingerprint()?;
        let expected = (
            plan.authority_files,
            plan.authority_bytes,
            plan.authority_sha256.clone(),
        );
        if current != expected {
            return Err(Error::Conflict(
                "canonical v0.1 authority changed after migration planning".to_owned(),
            ));
        }
        let record = MigrationRecord::from(plan);
        for directory in [
            "inventories",
            "knowledge",
            "relationships",
            "review-authorities",
            "migrations",
            super::CONTRIBUTION_PROTOCOL_DIRECTORY,
        ] {
            create_directory_chain(&workspace.state.join(directory))?;
        }
        let snapshot = workspace.load_snapshot_for_activation()?;
        if workspace.record_is_identical("migrations", &record)? {
            workspace.install_contribution_protocol()?;
            return Ok(false);
        }
        if !snapshot.migrations.is_empty() {
            return Err(Error::Conflict(
                "workspace already contains a different v0.1 migration record".to_owned(),
            ));
        }
        workspace.install_contribution_protocol()?;
        workspace.publish_record("migrations", &record)
    }

    pub fn read_migration_plan(path: &Path) -> Result<MigrationPlan> {
        reject_symlink_chain(path)?;
        let mut budget = ReadBudget::default();
        let plan = read_json_with_budget(path, &mut budget)?;
        MigrationPlan::validate(&plan)?;
        Ok(plan)
    }

    pub(super) fn authority_fingerprint(&self) -> Result<(usize, u64, String)> {
        let mut paths = Vec::new();
        collect_authority(&self.state, &self.state, &mut paths)?;
        paths.sort();
        let mut hasher = Sha256::new();
        let mut total = 0_u64;
        for path in &paths {
            let relative = path.strip_prefix(&self.state).expect("authority is rooted");
            let relative = (!super::injected_storage_failure("migration path UTF-8"))
                .then(|| relative.to_str())
                .flatten()
                .ok_or_else(|| Error::invalid("migration authority", "path must be UTF-8"))?;
            let bytes = read_bounded(path)?;
            // Canonical record and directory-count budgets keep this below u64::MAX.
            total += bytes.len() as u64;
            hasher.update((relative.len() as u64).to_be_bytes());
            hasher.update(relative.as_bytes());
            hasher.update((bytes.len() as u64).to_be_bytes());
            hasher.update(bytes);
        }
        Ok((paths.len(), total, format!("{:x}", hasher.finalize())))
    }
}

pub(super) fn collect_authority(
    root: &Path,
    directory: &Path,
    paths: &mut Vec<PathBuf>,
) -> Result<()> {
    reject_symlink_chain(directory)?;
    let mut entries = map_io(
        fs::read_dir(directory),
        "read migration authority",
        directory,
    )?
    .map(|entry| map_io(entry, "read migration authority entry", directory))
    .collect::<Result<Vec<_>>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let relative = path.strip_prefix(root).expect("authority entry is rooted");
        let metadata = map_io(
            fs::symlink_metadata(&path),
            "inspect migration authority",
            &path,
        )?;
        if metadata.file_type().is_symlink() {
            return Err(Error::invalid(
                "migration authority",
                format!("symlink is forbidden: {}", path.display()),
            ));
        }
        if relative.components().count() == 1
            && matches!(
                entry.file_name().to_str(),
                Some("migrations") | Some(super::CONTRIBUTION_PROTOCOL_DIRECTORY)
            )
        {
            continue;
        }
        if metadata.is_dir() {
            collect_authority(root, &path, paths)?;
        } else if metadata.is_file() && path.extension() == Some(OsStr::new("json")) {
            paths.push(path);
        } else if relative != Path::new("write.lock") {
            return Err(Error::invalid(
                "migration authority",
                format!("unexpected file {}", path.display()),
            ));
        }
    }
    Ok(())
}
