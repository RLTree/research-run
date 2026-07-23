use crate::domain::{
    ClaimRecord, EvidenceLink, ExperimentReceipt, ProjectManifest, ReviewAuthority, ReviewDecision,
    SourceRecord,
};
use crate::{Error, Result};

use super::recovery_canonical::{canonical_manifest, canonical_records};
use super::recovery_optional::{OptionalRecovery, preflight_optional};
use super::recovery_plan::PendingRecord;
use super::recovery_review::validate_pending_review_graphs;
use super::recovery_semantics::validate_recovered_authority;
use super::storage::ReadBudget;
use super::{Snapshot, Workspace, injected_storage_failure};

mod publish;

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
    contribution_protocols: Vec<PendingRecord>,
    requires_contribution_protocol_bootstrap: bool,
}

struct CoreRecovery {
    manifest: ProjectManifest,
    sources: Vec<SourceRecord>,
    claims: Vec<ClaimRecord>,
    experiments: Vec<ExperimentReceipt>,
    evidence: Vec<EvidenceLink>,
    reviews: Vec<ReviewDecision>,
    review_authorities: Vec<ReviewAuthority>,
    manifest_pending: Vec<PendingRecord>,
    source_pending: Vec<PendingRecord>,
    claim_pending: Vec<PendingRecord>,
    experiment_pending: Vec<PendingRecord>,
    evidence_pending: Vec<PendingRecord>,
    review_pending: Vec<PendingRecord>,
    review_authority_pending: Vec<PendingRecord>,
}

impl Workspace {
    pub(super) fn preflight_recovery_batch(&self) -> Result<RecoveryBatch> {
        let mut budget = ReadBudget::default();
        let core = preflight_core(self, &mut budget)?;
        let OptionalRecovery {
            contribution_protocols,
            inventories,
            knowledge,
            relationships,
            migrations,
            inventory_pending,
            knowledge_pending,
            relationship_pending,
            migration_pending,
            contribution_protocol_pending,
        } = preflight_optional(self, &mut budget)?;
        let (contribution_protocol, requires_contribution_protocol_bootstrap) =
            match contribution_protocols.as_slice() {
                [contribution_protocol] => (contribution_protocol.clone(), false),
                [] => (crate::domain::ContributionProtocol::agent_v1(), true),
                _ => {
                    return Err(Error::invalid(
                        "recovery activation",
                        "exactly one contribution protocol must be canonical or recoverable",
                    ));
                }
            };
        let snapshot = Snapshot {
            manifest: core.manifest,
            contribution_protocol,
            sources: core.sources,
            claims: core.claims,
            experiments: core.experiments,
            evidence: core.evidence,
            reviews: core.reviews,
            review_authorities: core.review_authorities,
            inventories,
            knowledge,
            relationships,
            migrations,
        };
        let batch = RecoveryBatch {
            manifest: core.manifest_pending,
            sources: core.source_pending,
            claims: core.claim_pending,
            experiments: core.experiment_pending,
            evidence: core.evidence_pending,
            reviews: core.review_pending,
            review_authorities: core.review_authority_pending,
            inventories: inventory_pending,
            knowledge: knowledge_pending,
            relationships: relationship_pending,
            migrations: migration_pending,
            contribution_protocols: contribution_protocol_pending,
            requires_contribution_protocol_bootstrap,
        };
        batch.validate(self, &snapshot)?;
        Ok(batch)
    }
}

fn preflight_core(workspace: &Workspace, budget: &mut ReadBudget) -> Result<CoreRecovery> {
    let mut manifest_pending = workspace.collect_recovery_pending(&workspace.state)?;
    let mut source_pending = collect_named(workspace, "sources")?;
    let mut claim_pending = collect_named(workspace, "claims")?;
    let mut experiment_pending = collect_named(workspace, "experiments")?;
    let mut evidence_pending = collect_named(workspace, "evidence")?;
    let mut review_pending = collect_named(workspace, "reviews")?;
    let mut review_authority_pending = collect_named_optional(workspace, "review-authorities")?;
    Ok(CoreRecovery {
        manifest: canonical_manifest(workspace, &mut manifest_pending, budget)?,
        sources: canonical_records(workspace, "sources", &mut source_pending, budget)?,
        claims: canonical_records(workspace, "claims", &mut claim_pending, budget)?,
        experiments: canonical_records(workspace, "experiments", &mut experiment_pending, budget)?,
        evidence: canonical_records(workspace, "evidence", &mut evidence_pending, budget)?,
        reviews: canonical_records(workspace, "reviews", &mut review_pending, budget)?,
        review_authorities: if workspace.state.join("review-authorities").exists() {
            canonical_records(
                workspace,
                "review-authorities",
                &mut review_authority_pending,
                budget,
            )?
        } else {
            Vec::new()
        },
        manifest_pending,
        source_pending,
        claim_pending,
        experiment_pending,
        evidence_pending,
        review_pending,
        review_authority_pending,
    })
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
    pub(super) fn requires_contribution_protocol_bootstrap(&self) -> bool {
        self.requires_contribution_protocol_bootstrap
    }

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
}
