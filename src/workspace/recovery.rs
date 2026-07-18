use std::ffi::OsStr;
use std::path::Path;

use crate::domain::{
    CanonicalRecord, ClaimRecord, EvidenceLink, ExperimentReceipt, ProjectManifest, ReviewDecision,
    SourceRecord,
};
use crate::{Error, Result};

use super::publication::WorkspaceWriteLock;
use super::storage::{ReadBudget, parse_json};
use super::{RecoveryResult, Workspace, injected_storage_failure};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RecoveryKind {
    Manifest,
    Source,
    Claim,
    Experiment,
    Evidence,
    Review,
}

impl Workspace {
    pub fn recover(&self) -> Result<RecoveryResult> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        let mut result = RecoveryResult {
            recovered: Vec::new(),
            discarded_identical: Vec::new(),
        };
        self.recover_directory(&self.state, RecoveryKind::Manifest, &mut result)?;
        let snapshot = self.load_snapshot_allow_pending(true)?;
        if !self.reference_errors(&snapshot).is_empty() {
            return Err(Error::invalid(
                "existing workspace",
                "reference validation failed before recovery",
            ));
        }
        for (directory, kind) in [
            ("sources", RecoveryKind::Source),
            ("claims", RecoveryKind::Claim),
            ("experiments", RecoveryKind::Experiment),
            ("evidence", RecoveryKind::Evidence),
            ("reviews", RecoveryKind::Review),
        ] {
            self.recover_directory(&self.state.join(directory), kind, &mut result)?;
        }
        let final_snapshot = self.load_snapshot()?;
        if !self.reference_errors(&final_snapshot).is_empty()
            || injected_storage_failure("recovered references")
        {
            return Err(Error::invalid(
                "recovered workspace",
                "reference validation failed after recovery",
            ));
        }
        Ok(result)
    }

    pub(super) fn validate_pending_record(
        &self,
        kind: RecoveryKind,
        target: &Path,
        bytes: &[u8],
    ) -> Result<Option<String>> {
        macro_rules! parse_record {
            ($record_type:ty) => {{
                let record: $record_type = parse_json(bytes, target)?;
                record.validate()?;
                if target.file_stem().and_then(OsStr::to_str) != Some(record.id()) {
                    return Err(Error::invalid(
                        "recovery target",
                        "filename does not match record id",
                    ));
                }
                record
            }};
        }
        let review_claim = match kind {
            RecoveryKind::Manifest => {
                let manifest: ProjectManifest = parse_json(bytes, target)?;
                manifest.validate()?;
                None
            }
            RecoveryKind::Source => {
                let _: SourceRecord = parse_record!(SourceRecord);
                None
            }
            RecoveryKind::Claim => {
                let _: ClaimRecord = parse_record!(ClaimRecord);
                None
            }
            RecoveryKind::Experiment => {
                let record: ExperimentReceipt = parse_record!(ExperimentReceipt);
                self.validate_artifact_paths(&record.artifacts)?;
                None
            }
            RecoveryKind::Evidence => {
                let record: EvidenceLink = parse_record!(EvidenceLink);
                self.require_existing_record("claims", &record.claim_id)?;
                if let Some(source_id) = &record.source_id {
                    self.require_existing_record("sources", source_id)?;
                }
                if let Some(experiment_id) = &record.experiment_id {
                    self.require_existing_record("experiments", experiment_id)?;
                }
                if let Some(locator) = &record.artifact {
                    self.validate_workspace_path(locator)?;
                }
                None
            }
            RecoveryKind::Review => {
                let record: ReviewDecision = parse_record!(ReviewDecision);
                self.require_existing_record("claims", &record.claim_id)?;
                let mut budget = ReadBudget::default();
                let reviews: Vec<ReviewDecision> =
                    self.load_records("reviews", true, &mut budget)?;
                if reviews
                    .iter()
                    .any(|existing| existing.claim_id == record.claim_id)
                {
                    return Err(Error::Conflict(format!(
                        "claim {} already has a v0.1 review decision",
                        record.claim_id
                    )));
                }
                Some(record.claim_id)
            }
        };
        Ok(review_claim)
    }
}
