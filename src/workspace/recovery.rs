use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::path::Path;

use crate::domain::{
    CanonicalRecord, ClaimRecord, EvidenceLink, ExperimentReceipt, ProjectManifest, ReviewDecision,
    SourceRecord,
};
use crate::{Error, Result};

use super::publication::WorkspaceWriteLock;
use super::storage::parse_json;
use super::{RecoveryResult, Snapshot, Workspace, injected_storage_failure};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RecoveryKind {
    Manifest,
    Source,
    Claim,
    Experiment,
    Evidence,
    Review,
}

#[derive(Clone, Copy)]
pub(super) enum RecordRecoveryKind {
    Manifest,
    Source,
    Claim,
    Experiment,
    Evidence,
}

pub(super) struct ReviewRecoveryAuthority {
    claims: BTreeSet<String>,
    evidence_by_claim: BTreeMap<String, Vec<String>>,
    reviewed_graphs: BTreeSet<(String, Vec<String>)>,
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
        let final_snapshot = if injected_storage_failure("final snapshot load") {
            Err(Error::invalid(
                "recovered workspace",
                "injected final snapshot failure",
            ))
        } else {
            self.load_snapshot()
        };
        #[allow(clippy::question_mark)]
        let final_snapshot = match final_snapshot {
            Ok(snapshot) => snapshot,
            Err(error) => return Err(error),
        };
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
        kind: RecordRecoveryKind,
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
            RecordRecoveryKind::Manifest => {
                let manifest: ProjectManifest = parse_json(bytes, target)?;
                manifest.validate()?;
                if target.file_name() != Some(OsStr::new("manifest.json")) {
                    return Err(Error::invalid(
                        "recovery target",
                        "project manifest must publish only as manifest.json",
                    ));
                }
                None
            }
            RecordRecoveryKind::Source => {
                let _: SourceRecord = parse_record!(SourceRecord);
                None
            }
            RecordRecoveryKind::Claim => {
                let _: ClaimRecord = parse_record!(ClaimRecord);
                None
            }
            RecordRecoveryKind::Experiment => {
                let record: ExperimentReceipt = parse_record!(ExperimentReceipt);
                self.validate_artifact_paths(&record.artifacts)?;
                None
            }
            RecordRecoveryKind::Evidence => {
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
        };
        Ok(review_claim)
    }

    pub(super) fn validate_pending_review(
        &self,
        target: &Path,
        bytes: &[u8],
        authority: &ReviewRecoveryAuthority,
    ) -> Result<Option<String>> {
        validate_pending_review(target, bytes, authority)
    }

    pub(super) fn review_recovery_authority(&self) -> Result<ReviewRecoveryAuthority> {
        Ok(ReviewRecoveryAuthority::from_snapshot(
            self.load_snapshot_allow_pending(true)?,
        ))
    }
}

impl ReviewRecoveryAuthority {
    fn from_snapshot(snapshot: Snapshot) -> Self {
        let claims = snapshot.claims.into_iter().map(|claim| claim.id).collect();
        let mut evidence_by_claim = BTreeMap::<String, Vec<String>>::new();
        for evidence in snapshot.evidence {
            evidence_by_claim
                .entry(evidence.claim_id)
                .or_default()
                .push(evidence.id);
        }
        for ids in evidence_by_claim.values_mut() {
            ids.sort();
        }
        let reviewed_graphs = snapshot
            .reviews
            .into_iter()
            .map(|review| (review.claim_id, review.evidence_ids))
            .collect();
        Self {
            claims,
            evidence_by_claim,
            reviewed_graphs,
        }
    }
}

fn validate_pending_review(
    target: &Path,
    bytes: &[u8],
    authority: &ReviewRecoveryAuthority,
) -> Result<Option<String>> {
    let record: ReviewDecision = parse_json(bytes, target)?;
    record.validate()?;
    if target.file_stem().and_then(OsStr::to_str) != Some(record.id()) {
        return Err(Error::invalid(
            "recovery target",
            "filename does not match record id",
        ));
    }
    if !authority.claims.contains(&record.claim_id) {
        return Err(Error::invalid("claim reference", "claim does not exist"));
    }
    let evidence_ids = authority
        .evidence_by_claim
        .get(&record.claim_id)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if record.evidence_ids != evidence_ids {
        return Err(Error::invalid(
            "review evidence_ids",
            "must exactly match the current claim evidence graph",
        ));
    }
    if authority
        .reviewed_graphs
        .contains(&(record.claim_id.clone(), record.evidence_ids.clone()))
    {
        return Err(Error::Conflict(format!(
            "claim {} already has a v0.1 review decision for this evidence graph",
            record.claim_id
        )));
    }
    Ok(Some(record.claim_id))
}
