use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;

use serde::Serialize;

use crate::domain::{
    Assessment, Authorship, ClaimRecord, ContributionProtocol, EvidenceLink, ExperimentReceipt,
    InventorySnapshot, KnowledgeRecord, MigrationRecord, ProjectManifest, RelationshipRecord,
    ReviewAuthority, ReviewDecision, SourceRecord,
};

mod agent_integration;
mod agent_integration_content;
mod agent_integration_publication;
mod agent_integration_types;
mod inventory;
mod inventory_authority;
mod inventory_bootstrap;
mod inventory_reconcile;
mod inventory_scan;
mod knowledge;
mod lifecycle;
mod migration;
mod path_safety;
mod pending_cleanup;
mod publication;
mod records;
mod recovery;
mod recovery_canonical;
mod recovery_commit;
mod recovery_optional;
mod recovery_plan;
mod recovery_preflight;
mod recovery_review;
mod recovery_semantics;
mod references;
mod retrieval;
mod retrieval_canonical;
mod retrieval_context;
mod retrieval_items;
mod retrieval_types;
mod review_authority;
mod review_binding;
mod snapshot;
mod sshsig;
mod status;
mod status_authority;
mod storage;
mod write_lock;

pub use inventory::{InventoryApplyResult, ReviewBootstrap};
pub use recovery::RecoveryResult;

pub(super) const STATE_DIRECTORY: &str = ".research-run";
pub(super) const CONTRIBUTION_PROTOCOL_DIRECTORY: &str = "contribution-protocols";
pub(super) const MAX_RECORD_BYTES: u64 = 1_048_576;
pub(super) const MAX_INVENTORY_RECORD_BYTES: u64 = 32 * 1_048_576;
pub(super) const MAX_RECORDS_PER_KIND: usize = 10_000;
pub(super) const MAX_SNAPSHOT_BYTES: u64 = 64 * 1_048_576;
pub const CLAIM_CEILING: &str = "Assessments describe reviewed support within this workspace; they do not establish scientific truth or real-world validity.";
pub(super) static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) fn record_byte_limit(directory: &str) -> u64 {
    if directory == "inventories" {
        MAX_INVENTORY_RECORD_BYTES
    } else {
        MAX_RECORD_BYTES
    }
}

pub(super) fn record_byte_limit_for_path(path: &Path) -> u64 {
    path.parent()
        .and_then(Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .map(record_byte_limit)
        .unwrap_or(MAX_RECORD_BYTES)
}
#[cfg(all(coverage, not(test)))]
static COVERAGE_FAULT_OCCURRENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
thread_local! {
    static STORAGE_FAILURE: std::cell::RefCell<Option<(&'static str, u64)>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn inject_storage_failure(point: &'static str) {
    let (point, occurrence) = point
        .rsplit_once('#')
        .and_then(|(point, occurrence)| occurrence.parse().ok().map(|count| (point, count)))
        .unwrap_or((point, 1));
    STORAGE_FAILURE.with_borrow_mut(|failure| *failure = Some((point, occurrence)));
}

#[cfg(test)]
pub(crate) fn take_storage_failure(point: &str) -> bool {
    STORAGE_FAILURE.with_borrow_mut(|failure| match failure.as_mut() {
        Some((target, remaining)) if *target == point && *remaining == 1 => {
            failure.take();
            true
        }
        Some((target, remaining)) if *target == point => {
            *remaining -= 1;
            false
        }
        _ => false,
    })
}

#[cfg(all(coverage, not(test)))]
pub(super) fn take_storage_failure(point: &str) -> bool {
    let Ok(configured) = std::env::var("RESEARCH_RUN_COVERAGE_FAULT") else {
        return false;
    };
    let (target, occurrence) = configured
        .rsplit_once('#')
        .and_then(|(target, occurrence)| occurrence.parse::<u64>().ok().map(|n| (target, n)))
        .unwrap_or((configured.as_str(), 1));
    if target != point {
        return false;
    }
    COVERAGE_FAULT_OCCURRENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1 == occurrence
}

#[cfg(test)]
pub(super) fn injected_storage_failure(point: &str) -> bool {
    take_storage_failure(point)
}

#[cfg(not(test))]
pub(super) fn injected_storage_failure(point: &str) -> bool {
    #[cfg(coverage)]
    {
        return take_storage_failure(point);
    }
    #[cfg(not(coverage))]
    {
        let _ = point;
        false
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    pub(super) root: PathBuf,
    pub(super) state: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub counts: BTreeMap<&'static str, usize>,
    pub agent_integration: AgentIntegrationStatus,
}

pub use agent_integration_types::{AgentIntegrationApplyResult, AgentIntegrationStatus};

#[derive(Debug, Serialize)]
pub struct Status {
    pub format_version: u32,
    pub project: ProjectStatus,
    pub review_authority: ReviewAuthorityStatus,
    pub claim_ceiling: &'static str,
    pub counts: BTreeMap<&'static str, usize>,
    pub claims: Vec<ClaimStatus>,
    pub experiments: Vec<ExperimentStatus>,
    pub unreviewed_ai_drafts: Vec<AiDraftStatus>,
    pub blockers: Vec<String>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectStatus {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ReviewAuthorityStatus {
    pub mode: &'static str,
    pub id: Option<String>,
    pub fingerprint: Option<String>,
    pub promotion_capable: bool,
    pub repairable: bool,
    pub blocker: Option<&'static str>,
    pub next_action: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ClaimStatus {
    pub id: String,
    pub text: String,
    pub scope: String,
    pub assessment: Assessment,
    pub evidence: Vec<EvidenceStatus>,
    pub blockers: Vec<String>,
    pub next_action: String,
}

#[derive(Debug, Serialize)]
pub struct EvidenceStatus {
    pub id: String,
    pub stance: crate::domain::Stance,
    pub authorship: Authorship,
}

#[derive(Debug, Serialize)]
pub struct ExperimentStatus {
    pub id: String,
    pub outcome: crate::domain::Outcome,
    pub observations: Vec<String>,
    pub interpretation: String,
    pub limitations: Vec<String>,
    pub next_move: String,
}

#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct AiDraftStatus {
    pub kind: &'static str,
    pub id: String,
    pub reason: &'static str,
}

pub use retrieval_types::{
    ContextBundle, HandoffBundle, ProjectionItem, ProjectionResult, RelationshipProjection,
};

pub(super) struct Snapshot {
    pub(super) manifest: ProjectManifest,
    pub(super) contribution_protocol: ContributionProtocol,
    pub(super) sources: Vec<SourceRecord>,
    pub(super) claims: Vec<ClaimRecord>,
    pub(super) evidence: Vec<EvidenceLink>,
    pub(super) experiments: Vec<ExperimentReceipt>,
    pub(super) reviews: Vec<ReviewDecision>,
    pub(super) review_authorities: Vec<ReviewAuthority>,
    pub(super) inventories: Vec<InventorySnapshot>,
    pub(super) knowledge: Vec<KnowledgeRecord>,
    pub(super) relationships: Vec<RelationshipRecord>,
    pub(super) migrations: Vec<MigrationRecord>,
}

impl Snapshot {
    pub(super) fn counts(&self) -> BTreeMap<&'static str, usize> {
        BTreeMap::from([
            ("contribution-protocol", 1),
            ("source", self.sources.len()),
            ("claim", self.claims.len()),
            ("evidence", self.evidence.len()),
            ("experiment", self.experiments.len()),
            ("review", self.reviews.len()),
            ("review-authority", self.review_authorities.len()),
            ("inventory", self.inventories.len()),
            ("knowledge", self.knowledge.len()),
            ("relationship", self.relationships.len()),
            ("migration", self.migrations.len()),
        ])
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
use path_safety::{
    create_directory_chain, ensure_no_pending_effect, interrupted_target_name, reject_symlink_chain,
};
#[cfg(test)]
use pending_cleanup::PendingCleanup;
#[cfg(test)]
use storage::{ReadBudget, read_bounded, read_json_with_budget};
#[cfg(test)]
use write_lock::WorkspaceWriteLock;
