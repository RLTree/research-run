use serde::Serialize;

use crate::Result;

mod experiments;
mod inventory;
mod knowledge;
mod records;
mod validation;

pub use experiments::{ArtifactLocatorType, ArtifactPointer, ExperimentReceipt, ReviewDecision};
pub use inventory::{
    InventoryPlan, InventorySnapshot, MaterialClass, MaterialEntry, ReconciliationChange,
    ReconciliationKind,
};
pub use knowledge::{
    EntityKind, EntityRef, KnowledgeKind, KnowledgeRecord, KnowledgeState, RelationshipKind,
    RelationshipRecord,
};
pub use records::{
    Assessment, Authorship, ClaimRecord, EvidenceLink, Outcome, ProjectManifest, SourceProvenance,
    SourceRecord, Stance,
};
pub use validation::{required_text, validate_id, validate_workspace_locator};

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_TEXT_BYTES: usize = 65_536;
pub const MAX_LIST_ITEMS: usize = 256;
pub const MAX_ARTIFACT_POINTERS: usize = 128;
pub const MAX_INVENTORY_ENTRIES: usize = 2_048;

pub trait CanonicalRecord: Serialize + Sized {
    const KIND: &'static str;

    fn id(&self) -> &str;
    fn validate(&self) -> Result<()>;
}

#[cfg(test)]
#[path = "domain_tests.rs"]
mod tests;
