use std::collections::BTreeMap;

use crate::domain::{
    CanonicalRecord, ClaimRecord, EvidenceLink, ExperimentReceipt, ReviewDecision, SourceRecord,
};
use crate::{Error, Result};

use super::publication::canonical_json_bytes;
use super::references::claim_evidence_ids;
use super::validation_types::bound_validation_errors;
use super::write_lock::WorkspaceWriteLock;
use super::{AgentIntegrationStatus, MAX_RECORDS_PER_KIND, ValidationResult, Workspace};

impl Workspace {
    pub fn add_source(&self, record: &SourceRecord) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        let snapshot = self.load_mutable_snapshot()?;
        if self.record_is_identical("sources", record)? {
            return Ok(false);
        }
        ensure_record_capacity(snapshot.sources.len(), "sources")?;
        self.publish_record("sources", record)
    }

    pub fn add_claim(&self, record: &ClaimRecord) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        let snapshot = self.load_mutable_snapshot()?;
        if self.record_is_identical("claims", record)? {
            return Ok(false);
        }
        ensure_record_capacity(snapshot.claims.len(), "claims")?;
        self.publish_record("claims", record)
    }

    pub fn add_experiment(&self, record: &ExperimentReceipt) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        self.validate_artifact_paths(&record.artifacts)?;
        let snapshot = self.load_mutable_snapshot()?;
        if self.record_is_identical("experiments", record)? {
            return Ok(false);
        }
        ensure_record_capacity(snapshot.experiments.len(), "experiments")?;
        self.publish_record("experiments", record)
    }

    pub fn add_evidence(&self, record: &EvidenceLink) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        let snapshot = self.load_mutable_snapshot()?;
        if self.record_is_identical("evidence", record)? {
            return Ok(false);
        }
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
        if self.record_is_identical("reviews", record)? {
            return Ok(false);
        }
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
        let expected_digest = self.review_subject_sha256(&snapshot, &record.claim_id)?;
        if record.subject_sha256.as_deref() != Some(expected_digest.as_str()) {
            return Err(Error::invalid(
                "review subject_sha256",
                "must exactly bind the current claim, evidence, and referenced authority",
            ));
        }
        self.verify_review_authorization(&snapshot, record)?;
        if snapshot.reviews.iter().any(|review| {
            review.claim_id == record.claim_id
                && review.evidence_ids == record.evidence_ids
                && review.subject_sha256 == record.subject_sha256
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
            Ok(snapshot) => self.validation_from_snapshot(&snapshot),
            Err(error) => {
                let diagnostic = error.to_string();
                ValidationResult {
                    schema_version: 2,
                    authority: None,
                    valid: false,
                    errors: vec![diagnostic.clone()],
                    counts: BTreeMap::new(),
                    agent_integration: AgentIntegrationStatus::inspection_unavailable(false),
                }
            }
        }
    }

    pub(super) fn validation_from_snapshot(&self, snapshot: &super::Snapshot) -> ValidationResult {
        let counts = snapshot.counts();
        let mut errors = self.reference_errors(snapshot);
        errors.extend(self.review_binding_errors(snapshot));
        errors.extend(self.review_authorization_errors(snapshot));
        let errors = bound_validation_errors(errors);
        let agent_integration = self
            .agent_integration_status_from_snapshot(snapshot)
            .unwrap_or_else(|_| AgentIntegrationStatus::inspection_unavailable(true));
        ValidationResult {
            schema_version: 2,
            authority: self.validation_authority(snapshot).ok(),
            valid: errors.is_empty(),
            errors,
            counts,
            agent_integration,
        }
    }

    pub(super) fn record_is_identical<T: CanonicalRecord + serde::Serialize>(
        &self,
        directory: &str,
        record: &T,
    ) -> Result<bool> {
        let target = self
            .state
            .join(directory)
            .join(format!("{}.json", record.id()));
        if !target.exists() {
            return Ok(false);
        }
        Ok(
            super::storage::read_bounded_with_limit(&target, super::record_byte_limit(directory))?
                == canonical_json_bytes(record),
        )
    }
}

fn ensure_record_capacity(count: usize, directory: &str) -> Result<()> {
    if count >= MAX_RECORDS_PER_KIND {
        return Err(Error::Budget(format!(
            "{directory} already reached the {MAX_RECORDS_PER_KIND} record budget"
        )));
    }
    Ok(())
}
