use std::fs::{self, File};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::domain::{
    ArtifactLocatorType, ArtifactPointer, Assessment, Authorship, CanonicalRecord, ClaimRecord,
    EntityKind, EntityRef, EvidenceLink, ExperimentReceipt, KnowledgeKind, KnowledgeRecord,
    KnowledgeState, Outcome, ProjectManifest, RelationshipKind, RelationshipRecord,
    ReviewAuthority, ReviewDecision, SourceProvenance, SourceRecord, Stance,
};

use super::{
    MAX_RECORD_BYTES, MAX_SNAPSHOT_BYTES, PendingCleanup, ReadBudget, ReviewBootstrap, Snapshot,
    Workspace, WorkspaceWriteLock, create_directory_chain, ensure_no_pending_effect,
    inject_storage_failure, interrupted_target_name, read_bounded, read_json_with_budget,
    reject_symlink_chain,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[path = "tests/agent_integration.rs"]
mod agent_integration;
#[path = "tests/agent_integration_recovery.rs"]
mod agent_integration_recovery;
#[path = "tests/review_binding_policy.rs"]
mod review_binding_policy;
#[path = "tests/review_signing.rs"]
mod review_signing;
use review_signing::*;
#[path = "tests/coverage_edges.rs"]
mod coverage_edges;
#[path = "tests/inventory_authority_coverage.rs"]
mod inventory_authority_coverage;
#[path = "tests/inventory_bootstrap_coverage.rs"]
mod inventory_bootstrap_coverage;
#[path = "tests/inventory_bootstrap_faults.rs"]
mod inventory_bootstrap_faults;
#[path = "tests/inventory_bootstrap_pending_faults.rs"]
mod inventory_bootstrap_pending_faults;
#[path = "tests/inventory_budgets.rs"]
mod inventory_budgets;
#[path = "tests/inventory_policy_child_identity.rs"]
mod inventory_policy_child_identity;
#[path = "tests/inventory_policy_limits.rs"]
mod inventory_policy_limits;
#[path = "tests/knowledge_inventory.rs"]
mod knowledge_inventory;
#[path = "tests/lifecycle_publication.rs"]
mod lifecycle_publication;
#[path = "tests/recovery_coverage_edges.rs"]
mod recovery_coverage_edges;
#[path = "tests/recovery_semantic_coverage.rs"]
mod recovery_semantic_coverage;
#[path = "tests/retrieval_inventory.rs"]
mod retrieval_inventory;
#[path = "tests/security_findings.rs"]
mod security_findings;

fn temporary() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "research-run-workspace-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).expect("temporary directory");
    path
}

fn explicit_unanchored_retrofit() -> Option<ReviewBootstrap> {
    Some(ReviewBootstrap::WithoutReviewAuthority)
}

#[test]
fn initialization_rejects_conflicting_existing_identity() {
    let root = temporary();
    Workspace::initialize(&root, "Original").expect("initialize");
    Workspace::initialize(&root, "Original").expect("idempotent retry");
    assert!(Workspace::initialize(&root, "Different").is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

fn source(id: &str) -> SourceRecord {
    SourceRecord {
        schema_version: 1,
        kind: "source".to_owned(),
        id: id.to_owned(),
        citation: "Citation".to_owned(),
        locator: "local:source".to_owned(),
        provenance: SourceProvenance::Human,
        notes: String::new(),
    }
}

fn claim(id: &str) -> ClaimRecord {
    ClaimRecord {
        schema_version: 1,
        kind: "claim".to_owned(),
        id: id.to_owned(),
        text: "Claim".to_owned(),
        scope: "Scope".to_owned(),
        owner: "Researcher".to_owned(),
        authorship: Authorship::Human,
    }
}

fn evidence(id: &str, claim_id: &str, source_id: Option<&str>) -> EvidenceLink {
    EvidenceLink {
        schema_version: 1,
        kind: "evidence".to_owned(),
        id: id.to_owned(),
        claim_id: claim_id.to_owned(),
        source_id: source_id.map(str::to_owned),
        experiment_id: None,
        artifact: source_id.is_none().then(|| "artifact.txt".to_owned()),
        stance: Stance::Limits,
        specific_evidence: "Specific evidence".to_owned(),
        authorship: Authorship::Human,
    }
}

fn review(id: &str, claim_id: &str) -> ReviewDecision {
    ReviewDecision {
        schema_version: 1,
        kind: "review".to_owned(),
        id: id.to_owned(),
        claim_id: claim_id.to_owned(),
        evidence_ids: Vec::new(),
        subject_sha256: None,
        decision: Assessment::Limited,
        rationale: "Bounded rationale".to_owned(),
        reviewer: "Researcher".to_owned(),
        authorization: None,
    }
}

fn experiment(id: &str) -> ExperimentReceipt {
    ExperimentReceipt {
        schema_version: 1,
        kind: "experiment".to_owned(),
        id: id.to_owned(),
        question: "Question?".to_owned(),
        method_ref: "protocol.md".to_owned(),
        observations: vec!["Observation".to_owned()],
        interpretation: "Interpretation".to_owned(),
        limitations: vec!["Limitation".to_owned()],
        outcome: Outcome::Negative,
        next_move: "Repeat".to_owned(),
        artifacts: Vec::new(),
    }
}

#[test]
fn non_history_relationship_may_close_a_history_path() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Relationship semantics").expect("initialize");
    let knowledge = |id: &str| KnowledgeRecord {
        schema_version: 1,
        kind: "knowledge".to_owned(),
        id: id.to_owned(),
        record_type: KnowledgeKind::Observation,
        title: id.to_owned(),
        body: "Retained history".to_owned(),
        occurred_at: "2026-07-18T20:00:00Z".to_owned(),
        state: KnowledgeState::Open,
        authorship: Authorship::Human,
    };
    workspace.add_knowledge(&knowledge("old")).expect("old");
    workspace.add_knowledge(&knowledge("new")).expect("new");
    let relation =
        |id: &str, relationship: RelationshipKind, from: &str, to: &str| RelationshipRecord {
            schema_version: 1,
            kind: "relationship".to_owned(),
            id: id.to_owned(),
            relationship,
            from: EntityRef {
                kind: EntityKind::Knowledge,
                id: from.to_owned(),
            },
            to: EntityRef {
                kind: EntityKind::Knowledge,
                id: to.to_owned(),
            },
            rationale: "Typed relation".to_owned(),
            occurred_at: "2026-07-18T20:00:00Z".to_owned(),
            authorship: Authorship::Human,
        };
    workspace
        .add_relationship(&relation(
            "revision",
            RelationshipKind::Revises,
            "new",
            "old",
        ))
        .expect("history edge");
    workspace
        .add_relationship(&relation(
            "related",
            RelationshipKind::RelatedTo,
            "old",
            "new",
        ))
        .expect("non-history edge");
    fs::remove_dir_all(root).expect("remove fixture");
}

#[path = "tests/handoff_expanded.rs"]
mod handoff_expanded;
#[path = "tests/handoff_validation.rs"]
mod handoff_validation;
#[path = "tests/handoff_validation_receipt.rs"]
mod handoff_validation_receipt;
#[path = "tests/inventory_expanded.rs"]
mod inventory_expanded;
#[path = "tests/inventory_failure_expanded.rs"]
mod inventory_failure_expanded;
#[path = "tests/inventory_policy.rs"]
mod inventory_policy;
#[path = "tests/inventory_policy_failures.rs"]
mod inventory_policy_failures;
#[path = "tests/inventory_protocol_coverage.rs"]
mod inventory_protocol_coverage;
#[path = "tests/inventory_tamper.rs"]
mod inventory_tamper;
#[path = "tests/knowledge_expanded.rs"]
mod knowledge_expanded;
#[path = "tests/migration_expanded.rs"]
mod migration_expanded;
#[path = "tests/migration_failure_expanded.rs"]
mod migration_failure_expanded;
#[path = "tests/migration_protocol_retry.rs"]
mod migration_protocol_retry;
#[path = "tests/records.rs"]
mod records;
#[path = "tests/recovery.rs"]
mod recovery;
#[path = "tests/retrieval_boundaries.rs"]
mod retrieval_boundaries;
#[path = "tests/retrieval_expanded.rs"]
mod retrieval_expanded;
#[path = "tests/retrieval_failure_expanded.rs"]
mod retrieval_failure_expanded;
#[path = "tests/review_authority_coverage.rs"]
mod review_authority_coverage;
#[path = "tests/review_binding_expanded.rs"]
mod review_binding_expanded;
#[path = "tests/snapshot_expanded.rs"]
mod snapshot_expanded;
#[path = "tests/sshsig.rs"]
mod sshsig;
#[path = "tests/status_authority.rs"]
mod status_authority;
#[path = "tests/storage.rs"]
mod storage;
#[path = "tests/validation_limits.rs"]
mod validation_limits;
