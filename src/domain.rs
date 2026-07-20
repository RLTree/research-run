use serde::Serialize;
#[cfg(test)]
use std::cell::Cell;

use crate::Result;

mod experiments;
mod inventory;
mod knowledge;
mod migration;
mod records;
mod validation;

pub use experiments::{
    ArtifactLocatorType, ArtifactPointer, ExperimentReceipt, REVIEW_SIGNATURE_NAMESPACE,
    ReviewAuthority, ReviewAuthorization, ReviewDecision, ReviewRequest,
};
pub use inventory::{
    InventoryPlan, InventorySnapshot, MaterialClass, MaterialEntry, ReconciliationChange,
    ReconciliationKind,
};
pub use knowledge::{
    EntityKind, EntityRef, KnowledgeKind, KnowledgeRecord, KnowledgeState, RelationshipKind,
    RelationshipRecord,
};
pub use migration::{MigrationPlan, MigrationRecord};
pub use records::{
    Assessment, Authorship, ClaimRecord, EvidenceLink, Outcome, ProjectManifest, SourceProvenance,
    SourceRecord, Stance,
};
pub(crate) use validation::validate_timestamp;
pub use validation::{required_text, validate_id, validate_workspace_locator};

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_TEXT_BYTES: usize = 65_536;
pub const MAX_LIST_ITEMS: usize = 256;
pub const MAX_ARTIFACT_POINTERS: usize = 128;

#[cfg(test)]
thread_local! {
    static RANDOM_FAILURE: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn inject_random_failure() {
    RANDOM_FAILURE.set(true);
}

#[cfg(test)]
fn take_random_failure() -> bool {
    RANDOM_FAILURE.replace(false)
}

#[cfg(not(test))]
const fn take_random_failure() -> bool {
    false
}
pub const MAX_INVENTORY_ENTRIES: usize = 2_048;

pub trait CanonicalRecord: Serialize + Sized {
    const KIND: &'static str;

    fn id(&self) -> &str;
    fn validate(&self) -> Result<()>;
}

#[cfg(test)]
#[path = "expanded_domain_tests.rs"]
mod expanded_tests;
#[cfg(test)]
#[path = "domain_inventory_tests.rs"]
mod inventory_tests;
#[cfg(test)]
#[path = "domain_review_tests.rs"]
mod review_tests;
#[cfg(test)]
#[path = "domain_tests.rs"]
mod tests;
