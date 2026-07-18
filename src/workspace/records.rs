use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use crate::domain::{
    CanonicalRecord, ClaimRecord, EvidenceLink, ExperimentReceipt, ProjectManifest, ReviewDecision,
    SourceRecord,
};
use crate::{Error, Result};

use super::path_safety::{interrupted_target_name, reject_symlink_chain};
use super::publication::WorkspaceWriteLock;
use super::storage::{ReadBudget, map_io, read_bounded, read_json_with_budget};
use super::{
    MAX_RECORDS_PER_KIND, Snapshot, ValidationResult, Workspace, injected_storage_failure,
};

impl Workspace {
    pub fn add_source(&self, record: &SourceRecord) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        let snapshot = self.load_mutable_snapshot()?;
        ensure_record_capacity(snapshot.sources.len(), "sources")?;
        self.publish_record("sources", record)
    }

    pub fn add_claim(&self, record: &ClaimRecord) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        let snapshot = self.load_mutable_snapshot()?;
        ensure_record_capacity(snapshot.claims.len(), "claims")?;
        self.publish_record("claims", record)
    }

    pub fn add_experiment(&self, record: &ExperimentReceipt) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        self.validate_artifact_paths(&record.artifacts)?;
        let snapshot = self.load_mutable_snapshot()?;
        ensure_record_capacity(snapshot.experiments.len(), "experiments")?;
        self.publish_record("experiments", record)
    }

    pub fn add_evidence(&self, record: &EvidenceLink) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        let snapshot = self.load_mutable_snapshot()?;
        ensure_record_capacity(snapshot.evidence.len(), "evidence")?;
        if !snapshot
            .claims
            .iter()
            .any(|claim| claim.id == record.claim_id)
        {
            return Err(Error::invalid("claim reference", "claim does not exist"));
        }
        if let Some(source_id) = &record.source_id
            && !snapshot
                .sources
                .iter()
                .any(|source| &source.id == source_id)
        {
            return Err(Error::invalid("source reference", "source does not exist"));
        }
        if let Some(experiment_id) = &record.experiment_id
            && !snapshot
                .experiments
                .iter()
                .any(|experiment| &experiment.id == experiment_id)
        {
            return Err(Error::invalid(
                "experiment reference",
                "experiment does not exist",
            ));
        }
        if let Some(locator) = &record.artifact {
            self.validate_workspace_path(locator)?;
        }
        self.publish_record("evidence", record)
    }

    pub fn add_review(&self, record: &ReviewDecision) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        let snapshot = self.load_mutable_snapshot()?;
        ensure_record_capacity(snapshot.reviews.len(), "reviews")?;
        if !snapshot
            .claims
            .iter()
            .any(|claim| claim.id == record.claim_id)
        {
            return Err(Error::invalid("claim reference", "claim does not exist"));
        }
        let evidence_ids = claim_evidence_ids(&snapshot, &record.claim_id);
        if record.evidence_ids != evidence_ids {
            return Err(Error::invalid(
                "review evidence_ids",
                "must exactly match the current claim evidence graph",
            ));
        }
        if snapshot.reviews.iter().any(|review| {
            review.claim_id == record.claim_id && review.evidence_ids == record.evidence_ids
        }) {
            return Err(Error::Conflict(format!(
                "claim {} already has a v0.1 review decision for this evidence graph",
                record.claim_id
            )));
        }
        self.publish_record("reviews", record)
    }

    pub fn claim_evidence_ids(&self, claim_id: &str) -> Result<Vec<String>> {
        let snapshot = self.load_snapshot()?;
        if !snapshot.claims.iter().any(|claim| claim.id == claim_id) {
            return Err(Error::invalid("claim reference", "claim does not exist"));
        }
        Ok(claim_evidence_ids(&snapshot, claim_id))
    }

    pub fn validate(&self) -> ValidationResult {
        match self.load_snapshot() {
            Ok(snapshot) => {
                let counts = snapshot.counts();
                let errors = self.reference_errors(&snapshot);
                ValidationResult {
                    valid: errors.is_empty(),
                    errors,
                    counts,
                }
            }
            Err(error) => ValidationResult {
                valid: false,
                errors: vec![error.to_string()],
                counts: BTreeMap::new(),
            },
        }
    }

    pub(super) fn require_existing_record(&self, directory: &str, id: &str) -> Result<()> {
        let path = self.state.join(directory).join(format!("{id}.json"));
        match read_bounded(&path) {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::invalid(
                "recovered record reference",
                format!("{directory}/{id} is missing or invalid"),
            )),
        }
    }

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

    fn load_mutable_snapshot(&self) -> Result<Snapshot> {
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
        let snapshot = Snapshot {
            manifest,
            sources: self.load_records("sources", allow_pending, &mut budget)?,
            claims: self.load_records("claims", allow_pending, &mut budget)?,
            evidence: self.load_records("evidence", allow_pending, &mut budget)?,
            experiments: self.load_records("experiments", allow_pending, &mut budget)?,
            reviews: self.load_records("reviews", allow_pending, &mut budget)?,
        };
        Ok(snapshot)
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

fn claim_evidence_ids(snapshot: &Snapshot, claim_id: &str) -> Vec<String> {
    let mut ids = snapshot
        .evidence
        .iter()
        .filter(|evidence| evidence.claim_id == claim_id)
        .map(|evidence| evidence.id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

fn ensure_record_capacity(count: usize, directory: &str) -> Result<()> {
    if count >= MAX_RECORDS_PER_KIND {
        return Err(Error::Budget(format!(
            "{directory} already reached the {MAX_RECORDS_PER_KIND} record budget"
        )));
    }
    Ok(())
}
