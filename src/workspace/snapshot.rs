use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use serde::de::DeserializeOwned;

use crate::domain::{CanonicalRecord, ProjectManifest};
use crate::{Error, Result};

use super::path_safety::{interrupted_target_name, reject_symlink_chain};
use super::storage::{ReadBudget, map_io, read_json_with_budget};
use super::{MAX_RECORDS_PER_KIND, Snapshot, Workspace, injected_storage_failure};

impl Workspace {
    pub(super) fn read_manifest(&self) -> Result<ProjectManifest> {
        let mut budget = ReadBudget::default();
        let manifest: ProjectManifest =
            read_json_with_budget(&self.state.join("manifest.json"), &mut budget)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub(super) fn load_snapshot(&self) -> Result<Snapshot> {
        self.load_snapshot_allow_pending(false)
    }

    pub(super) fn load_mutable_snapshot(&self) -> Result<Snapshot> {
        let snapshot = self.load_snapshot()?;
        if !self.reference_errors(&snapshot).is_empty() {
            return Err(Error::invalid(
                "existing workspace",
                "reference validation failed before mutation",
            ));
        }
        Ok(snapshot)
    }

    pub(super) fn load_snapshot_allow_pending(&self, allow_pending: bool) -> Result<Snapshot> {
        let mut budget = ReadBudget::default();
        let manifest: ProjectManifest =
            read_json_with_budget(&self.state.join("manifest.json"), &mut budget)?;
        manifest.validate()?;
        let relationships =
            self.load_optional_records("relationships", allow_pending, &mut budget)?;
        let knowledge = self.load_optional_records("knowledge", allow_pending, &mut budget)?;
        let migrations = self.load_optional_records("migrations", allow_pending, &mut budget)?;
        Ok(Snapshot {
            manifest,
            sources: self.load_records("sources", allow_pending, &mut budget)?,
            claims: self.load_records("claims", allow_pending, &mut budget)?,
            evidence: self.load_records("evidence", allow_pending, &mut budget)?,
            experiments: self.load_records("experiments", allow_pending, &mut budget)?,
            reviews: self.load_records("reviews", allow_pending, &mut budget)?,
            inventories: self.load_optional_records("inventories", allow_pending, &mut budget)?,
            knowledge,
            relationships,
            migrations,
        })
    }

    pub(super) fn load_optional_records<T>(
        &self,
        directory: &str,
        allow_pending: bool,
        budget: &mut ReadBudget,
    ) -> Result<Vec<T>>
    where
        T: DeserializeOwned + CanonicalRecord,
    {
        let path = self.state.join(directory);
        reject_symlink_chain(&path)?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        self.load_records(directory, allow_pending, budget)
    }

    pub(super) fn load_records<T>(
        &self,
        directory: &str,
        allow_pending: bool,
        budget: &mut ReadBudget,
    ) -> Result<Vec<T>>
    where
        T: DeserializeOwned + CanonicalRecord,
    {
        let entries = self.record_paths(directory, allow_pending)?;
        let mut records = Vec::with_capacity(entries.len());
        for record_path in entries {
            let record: T = read_json_with_budget(&record_path, budget)?;
            record.validate()?;
            if record_path.file_stem().and_then(OsStr::to_str) != Some(record.id()) {
                return Err(Error::invalid(
                    "record filename",
                    format!("must match id in {}", record_path.display()),
                ));
            }
            records.push(record);
        }
        Ok(records)
    }

    fn record_paths(&self, directory: &str, allow_pending: bool) -> Result<Vec<PathBuf>> {
        let path = self.state.join(directory);
        reject_symlink_chain(&path)?;
        let mut entries = Vec::new();
        for entry in map_io(fs::read_dir(&path), "read record directory", &path)? {
            let entry = map_io(entry, "read record entry", &path)?;
            let record_path = entry.path();
            reject_symlink_chain(&record_path)?;
            if interrupted_target_name(&entry.file_name()).is_some() {
                if allow_pending {
                    continue;
                }
                return Err(Error::AmbiguousEffect(format!(
                    "interrupted publication found at {}; run 'research-run recover'",
                    record_path.display()
                )));
            }
            if record_path.extension() != Some(OsStr::new("json")) {
                return Err(Error::invalid(
                    "record directory",
                    format!("unexpected file {}", record_path.display()),
                ));
            }
            entries.push(record_path);
            if entries.len() > MAX_RECORDS_PER_KIND || injected_storage_failure("record count") {
                return Err(Error::Budget(format!(
                    "{directory} exceeds the {MAX_RECORDS_PER_KIND} record budget"
                )));
            }
        }
        entries.sort();
        Ok(entries)
    }
}
