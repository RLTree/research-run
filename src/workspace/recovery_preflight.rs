use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::domain::{
    CanonicalRecord, ClaimRecord, EvidenceLink, ExperimentReceipt, ProjectManifest, ReviewDecision,
    SourceRecord,
};
use crate::{Error, Result};

use super::publication::canonical_json_bytes;
use super::recovery_commit::commit_recovery;
use super::recovery_optional::{OptionalRecovery, preflight_optional};
use super::recovery_plan::PendingRecord;
use super::recovery_review::validate_pending_review_graphs;
use super::storage::{ReadBudget, parse_json, read_bounded, read_json_with_budget};
use super::{RecoveryResult, Snapshot, Workspace, injected_storage_failure};

pub(super) struct RecoveryBatch {
    manifest: Vec<PendingRecord>,
    sources: Vec<PendingRecord>,
    claims: Vec<PendingRecord>,
    experiments: Vec<PendingRecord>,
    evidence: Vec<PendingRecord>,
    reviews: Vec<PendingRecord>,
    inventories: Vec<PendingRecord>,
    knowledge: Vec<PendingRecord>,
    relationships: Vec<PendingRecord>,
    migrations: Vec<PendingRecord>,
}

impl Workspace {
    pub(super) fn preflight_recovery_batch(&self) -> Result<RecoveryBatch> {
        let mut manifest_pending = self.collect_recovery_pending(&self.state)?;
        let mut source_pending = collect_named(self, "sources")?;
        let mut claim_pending = collect_named(self, "claims")?;
        let mut experiment_pending = collect_named(self, "experiments")?;
        let mut evidence_pending = collect_named(self, "evidence")?;
        let mut review_pending = collect_named(self, "reviews")?;
        let mut budget = ReadBudget::default();
        let manifest = canonical_manifest(self, &mut manifest_pending, &mut budget)?;
        let sources =
            canonical_records::<SourceRecord>(self, "sources", &mut source_pending, &mut budget)?;
        let claims =
            canonical_records::<ClaimRecord>(self, "claims", &mut claim_pending, &mut budget)?;
        let experiments = canonical_records::<ExperimentReceipt>(
            self,
            "experiments",
            &mut experiment_pending,
            &mut budget,
        )?;
        let evidence = canonical_records::<EvidenceLink>(
            self,
            "evidence",
            &mut evidence_pending,
            &mut budget,
        )?;
        let reviews =
            canonical_records::<ReviewDecision>(self, "reviews", &mut review_pending, &mut budget)?;
        let OptionalRecovery {
            inventories,
            knowledge,
            relationships,
            migrations,
            inventory_pending,
            knowledge_pending,
            relationship_pending,
            migration_pending,
        } = preflight_optional(self, &mut budget)?;
        let snapshot = Snapshot {
            manifest,
            sources,
            claims,
            experiments,
            evidence,
            reviews,
            inventories,
            knowledge,
            relationships,
            migrations,
        };
        if injected_storage_failure("final snapshot load") {
            return Err(Error::invalid(
                "recovery plan",
                "injected final snapshot failure",
            ));
        }
        let errors = self.reference_errors(&snapshot);
        validate_pending_review_graphs(self, &snapshot, &review_pending)?;
        if errors.is_empty() && !injected_storage_failure("recovered references") {
            Ok(RecoveryBatch {
                manifest: manifest_pending,
                sources: source_pending,
                claims: claim_pending,
                experiments: experiment_pending,
                evidence: evidence_pending,
                reviews: review_pending,
                inventories: inventory_pending,
                knowledge: knowledge_pending,
                relationships: relationship_pending,
                migrations: migration_pending,
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

fn collect_named(workspace: &Workspace, name: &str) -> Result<Vec<PendingRecord>> {
    workspace.collect_recovery_pending(&workspace.state.join(name))
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
            ("inventories", self.inventories),
            ("knowledge", self.knowledge),
            ("relationships", self.relationships),
            ("migrations", self.migrations),
        ] {
            commit_recovery(&workspace.state.join(directory), pending, result)?;
        }
        Ok(())
    }
}

fn canonical_manifest(
    workspace: &Workspace,
    pending: &mut [PendingRecord],
    budget: &mut ReadBudget,
) -> Result<ProjectManifest> {
    let target = workspace.state.join("manifest.json");
    let mut candidate = None;
    for record in pending.iter_mut() {
        budget.consume(&record.path, record.bytes.len() as u64)?;
        let parsed: ProjectManifest = parse_json(&record.bytes, &record.target)?;
        parsed.validate()?;
        record.bytes = canonical_json_bytes(&parsed);
        candidate.get_or_insert(parsed);
    }
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
    candidate.ok_or_else(|| Error::NotFound("recovery requires a project manifest".to_owned()))
}

pub(super) fn canonical_records<T>(
    workspace: &Workspace,
    directory: &str,
    pending: &mut [PendingRecord],
    budget: &mut ReadBudget,
) -> Result<Vec<T>>
where
    T: CanonicalRecord + DeserializeOwned + Serialize,
{
    let mut candidates = BTreeMap::<std::path::PathBuf, T>::new();
    for record in pending.iter_mut() {
        budget.consume(&record.path, record.bytes.len() as u64)?;
        let candidate: T = parse_json(&record.bytes, &record.target)?;
        candidate.validate()?;
        if record.target.file_stem().and_then(OsStr::to_str) != Some(candidate.id()) {
            return Err(Error::invalid(
                "recovery target",
                "filename does not match record id",
            ));
        }
        record.bytes = canonical_json_bytes(&candidate);
        candidates.entry(record.target.clone()).or_insert(candidate);
    }
    let mut records = workspace.load_records(directory, true, budget)?;
    for record in unique_pending(pending)? {
        if record.target.exists() {
            ensure_identical_target(record)?;
            continue;
        }
        records.push(
            candidates
                .remove(&record.target)
                .expect("every canonical pending target retains its typed record"),
        );
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
