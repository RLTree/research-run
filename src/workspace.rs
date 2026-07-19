use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;

use serde::Serialize;

use crate::domain::{
    Assessment, Authorship, ClaimRecord, EvidenceLink, ExperimentReceipt, InventorySnapshot,
    KnowledgeRecord, ProjectManifest, RelationshipRecord, ReviewDecision, SourceRecord,
};

mod inventory;
mod inventory_reconcile;
mod inventory_scan;
mod knowledge;
mod lifecycle;
mod path_safety;
mod pending_cleanup;
mod publication;
mod records;
mod recovery;
mod recovery_commit;
mod recovery_optional;
mod recovery_plan;
mod recovery_preflight;
mod recovery_review;
mod references;
mod retrieval;
mod retrieval_items;
mod retrieval_types;
mod snapshot;
mod status;
mod storage;
mod write_lock;

pub(super) const STATE_DIRECTORY: &str = ".research-run";
pub(super) const MAX_RECORD_BYTES: u64 = 1_048_576;
pub(super) const MAX_RECORDS_PER_KIND: usize = 10_000;
pub(super) const MAX_SNAPSHOT_BYTES: u64 = 64 * 1_048_576;
pub(super) static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[cfg(all(coverage, not(test)))]
static COVERAGE_FAULT_OCCURRENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(test)]
thread_local! {
    static STORAGE_FAILURE: std::cell::RefCell<Option<&'static str>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn inject_storage_failure(point: &'static str) {
    STORAGE_FAILURE.with_borrow_mut(|failure| *failure = Some(point));
}

#[cfg(test)]
pub(super) fn take_storage_failure(point: &str) -> bool {
    STORAGE_FAILURE.with_borrow_mut(|failure| {
        if failure.as_deref() == Some(point) {
            failure.take();
            true
        } else {
            false
        }
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
}

#[derive(Debug, Serialize)]
pub struct Status {
    pub format_version: u32,
    pub project: ProjectStatus,
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

#[derive(Debug, Serialize)]
pub struct RecoveryResult {
    pub recovered: Vec<String>,
    pub discarded_identical: Vec<String>,
}

pub use retrieval_types::{
    ContextBundle, ProjectionItem, ProjectionResult, RelationshipProjection,
};

pub(super) struct Snapshot {
    pub(super) manifest: ProjectManifest,
    pub(super) sources: Vec<SourceRecord>,
    pub(super) claims: Vec<ClaimRecord>,
    pub(super) evidence: Vec<EvidenceLink>,
    pub(super) experiments: Vec<ExperimentReceipt>,
    pub(super) reviews: Vec<ReviewDecision>,
    pub(super) inventories: Vec<InventorySnapshot>,
    pub(super) knowledge: Vec<KnowledgeRecord>,
    pub(super) relationships: Vec<RelationshipRecord>,
}

impl Snapshot {
    pub(super) fn counts(&self) -> BTreeMap<&'static str, usize> {
        BTreeMap::from([
            ("source", self.sources.len()),
            ("claim", self.claims.len()),
            ("evidence", self.evidence.len()),
            ("experiment", self.experiments.len()),
            ("review", self.reviews.len()),
            ("inventory", self.inventories.len()),
            ("knowledge", self.knowledge.len()),
            ("relationship", self.relationships.len()),
        ])
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
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
