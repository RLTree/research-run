use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

use serde::de::DeserializeOwned;

use crate::domain::{CanonicalRecord, ProjectManifest};
use crate::{Error, Result};

use super::recovery_commit::commit_recovery;
use super::recovery_plan::PendingRecord;
use super::storage::{ReadBudget, parse_json, read_bounded, read_json_with_budget};
use super::{RecoveryResult, Snapshot, Workspace, injected_storage_failure};

pub(super) struct RecoveryBatch {
    manifest: Vec<PendingRecord>,
    sources: Vec<PendingRecord>,
    claims: Vec<PendingRecord>,
    experiments: Vec<PendingRecord>,
    evidence: Vec<PendingRecord>,
    reviews: Vec<PendingRecord>,
}

impl Workspace {
    pub(super) fn preflight_recovery_batch(&self) -> Result<RecoveryBatch> {
        let manifest_pending = self.collect_recovery_pending(&self.state)?;
        let source_pending = self.collect_recovery_pending(&self.state.join("sources"))?;
        let claim_pending = self.collect_recovery_pending(&self.state.join("claims"))?;
        let experiment_pending = self.collect_recovery_pending(&self.state.join("experiments"))?;
        let evidence_pending = self.collect_recovery_pending(&self.state.join("evidence"))?;
        let review_pending = self.collect_recovery_pending(&self.state.join("reviews"))?;
        let mut budget = ReadBudget::default();
        let snapshot = Snapshot {
            manifest: prospective_manifest(self, &manifest_pending, &mut budget)?,
            sources: prospective_records(self, "sources", &source_pending, &mut budget)?,
            claims: prospective_records(self, "claims", &claim_pending, &mut budget)?,
            experiments: prospective_records(
                self,
                "experiments",
                &experiment_pending,
                &mut budget,
            )?,
            evidence: prospective_records(self, "evidence", &evidence_pending, &mut budget)?,
            reviews: prospective_records(self, "reviews", &review_pending, &mut budget)?,
        };
        if injected_storage_failure("final snapshot load") {
            return Err(Error::invalid(
                "recovery plan",
                "injected final snapshot failure",
            ));
        }
        let errors = self.reference_errors(&snapshot);
        validate_pending_review_graphs(&snapshot, &review_pending)?;
        if errors.is_empty() && !injected_storage_failure("recovered references") {
            Ok(RecoveryBatch {
                manifest: manifest_pending,
                sources: source_pending,
                claims: claim_pending,
                experiments: experiment_pending,
                evidence: evidence_pending,
                reviews: review_pending,
            })
        } else {
            Err(Error::invalid(
                "recovery plan",
                format!(
                    "prospective reference validation failed: {}",
                    errors.join("; ")
                ),
            ))
        }
    }
}

impl RecoveryBatch {
    pub(super) fn publish(self, workspace: &Workspace, result: &mut RecoveryResult) -> Result<()> {
        commit_recovery(&workspace.state, self.manifest, result)?;
        for (directory, pending) in [
            ("sources", self.sources),
            ("claims", self.claims),
            ("experiments", self.experiments),
            ("evidence", self.evidence),
            ("reviews", self.reviews),
        ] {
            commit_recovery(&workspace.state.join(directory), pending, result)?;
        }
        Ok(())
    }
}

fn validate_pending_review_graphs(snapshot: &Snapshot, pending: &[PendingRecord]) -> Result<()> {
    let mut current = BTreeMap::<&str, Vec<&str>>::new();
    for evidence in &snapshot.evidence {
        current
            .entry(evidence.claim_id.as_str())
            .or_default()
            .push(evidence.id.as_str());
    }
    for ids in current.values_mut() {
        ids.sort_unstable();
    }
    for review in &snapshot.reviews {
        let mut pending_review = false;
        for record in pending {
            if record.target.file_stem().and_then(OsStr::to_str) == Some(review.id.as_str()) {
                pending_review = true;
                break;
            }
        }
        if !pending_review {
            continue;
        }
        let evidence = current
            .get(review.claim_id.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();
        if !review
            .evidence_ids
            .iter()
            .map(String::as_str)
            .eq(evidence.iter().copied())
        {
            return Err(Error::invalid(
                "review evidence_ids",
                "must exactly match the current claim evidence graph",
            ));
        }
    }
    Ok(())
}

fn prospective_manifest(
    workspace: &Workspace,
    pending: &[PendingRecord],
    budget: &mut ReadBudget,
) -> Result<ProjectManifest> {
    let target = workspace.state.join("manifest.json");
    let unique = unique_pending(pending)?;
    if unique.iter().any(|record| record.target != target) {
        return Err(Error::invalid(
            "recovery target",
            "project manifest must publish only as manifest.json",
        ));
    }
    if target.exists() {
        for record in unique {
            ensure_identical_target(record)?;
        }
        return read_json_with_budget::<ProjectManifest>(&target, budget);
    }
    let record = unique
        .into_iter()
        .next()
        .ok_or_else(|| Error::NotFound("recovery requires a project manifest".to_owned()))?;
    budget.consume(&record.path, record.bytes.len() as u64)?;
    let candidate: ProjectManifest = parse_json(&record.bytes, &record.target)?;
    candidate.validate()?;
    Ok(candidate)
}

fn prospective_records<T>(
    workspace: &Workspace,
    directory: &str,
    pending: &[PendingRecord],
    budget: &mut ReadBudget,
) -> Result<Vec<T>>
where
    T: CanonicalRecord + DeserializeOwned,
{
    let mut records = workspace.load_records(directory, true, budget)?;
    for record in unique_pending(pending)? {
        if record.target.exists() {
            ensure_identical_target(record)?;
            continue;
        }
        budget.consume(&record.path, record.bytes.len() as u64)?;
        let candidate: T = parse_json(&record.bytes, &record.target)?;
        candidate.validate()?;
        if record.target.file_stem().and_then(OsStr::to_str) != Some(candidate.id()) {
            return Err(Error::invalid(
                "recovery target",
                "filename does not match record id",
            ));
        }
        records.push(candidate);
    }
    Ok(records)
}

fn unique_pending(pending: &[PendingRecord]) -> Result<Vec<&PendingRecord>> {
    let mut by_target = BTreeMap::<&Path, &PendingRecord>::new();
    for record in pending {
        if let Some(existing) = by_target.get(record.target.as_path())
            && existing.bytes != record.bytes
        {
            return Err(Error::AmbiguousEffect(format!(
                "conflicting pending publications target {}; inspect them before recovery",
                record.target.display()
            )));
        }
        by_target.insert(&record.target, record);
    }
    Ok(by_target.into_values().collect())
}

fn ensure_identical_target(record: &PendingRecord) -> Result<()> {
    if read_bounded(&record.target)? == record.bytes {
        Ok(())
    } else {
        Err(Error::AmbiguousEffect(format!(
            "recovery conflict for {}; inspect both files",
            record.target.display()
        )))
    }
}
