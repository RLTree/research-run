use crate::domain::{
    ClaimRecord, EvidenceLink, ExperimentReceipt, ReviewAuthority, ReviewDecision, SourceRecord,
};
use crate::{Error, Result};

use super::recovery_canonical::{canonical_manifest, canonical_records};
use super::recovery_commit::commit_recovery;
use super::recovery_optional::{OptionalRecovery, preflight_optional};
use super::recovery_plan::PendingRecord;
use super::recovery_review::validate_pending_review_graphs;
use super::recovery_semantics::validate_recovered_authority;
use super::storage::ReadBudget;
use super::{RecoveryResult, Snapshot, Workspace, injected_storage_failure};

pub(super) struct RecoveryBatch {
    manifest: Vec<PendingRecord>,
    sources: Vec<PendingRecord>,
    claims: Vec<PendingRecord>,
    experiments: Vec<PendingRecord>,
    evidence: Vec<PendingRecord>,
    reviews: Vec<PendingRecord>,
    review_authorities: Vec<PendingRecord>,
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
        let mut review_authority_pending = collect_named_optional(self, "review-authorities")?;
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
        let review_authorities = if self.state.join("review-authorities").exists() {
            canonical_records::<ReviewAuthority>(
                self,
                "review-authorities",
                &mut review_authority_pending,
                &mut budget,
            )?
        } else {
            Vec::new()
        };
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
            review_authorities,
            inventories,
            knowledge,
            relationships,
            migrations,
        };
        let batch = RecoveryBatch {
            manifest: manifest_pending,
            sources: source_pending,
            claims: claim_pending,
            experiments: experiment_pending,
            evidence: evidence_pending,
            reviews: review_pending,
            review_authorities: review_authority_pending,
            inventories: inventory_pending,
            knowledge: knowledge_pending,
            relationships: relationship_pending,
            migrations: migration_pending,
        };
        batch.validate(self, &snapshot)?;
        Ok(batch)
    }
}

fn collect_named(workspace: &Workspace, name: &str) -> Result<Vec<PendingRecord>> {
    workspace.collect_recovery_pending(&workspace.state.join(name))
}

fn collect_named_optional(workspace: &Workspace, name: &str) -> Result<Vec<PendingRecord>> {
    let path = workspace.state.join(name);
    if path.exists() {
        workspace.collect_recovery_pending(&path)
    } else {
        Ok(Vec::new())
    }
}

impl RecoveryBatch {
    fn validate(&self, workspace: &Workspace, snapshot: &Snapshot) -> Result<()> {
        validate_recovered_authority(
            workspace,
            snapshot,
            &self.inventories,
            &self.migrations,
            [
                &self.manifest,
                &self.sources,
                &self.claims,
                &self.experiments,
                &self.evidence,
                &self.reviews,
                &self.review_authorities,
                &self.inventories,
                &self.knowledge,
                &self.relationships,
            ],
        )?;
        if injected_storage_failure("final snapshot load") {
            return Err(Error::invalid(
                "recovery plan",
                "injected final snapshot failure",
            ));
        }
        let errors = workspace.reference_errors(snapshot);
        validate_pending_review_graphs(workspace, snapshot, &self.reviews)?;
        let authorization_errors = workspace.review_authorization_errors(snapshot);
        if errors.is_empty()
            && authorization_errors.is_empty()
            && !injected_storage_failure("recovered references")
        {
            Ok(())
        } else {
            Err(Error::invalid(
                "recovery plan",
                format!(
                    "prospective reference validation failed: {}",
                    errors
                        .into_iter()
                        .chain(authorization_errors)
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
            ))
        }
    }

    pub(super) fn publish(self, workspace: &Workspace, result: &mut RecoveryResult) -> Result<()> {
        commit_recovery(&workspace.state, self.manifest, result)?;
        for (directory, pending) in [
            ("sources", self.sources),
            ("claims", self.claims),
            ("experiments", self.experiments),
            ("evidence", self.evidence),
            ("reviews", self.reviews),
            ("review-authorities", self.review_authorities),
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
