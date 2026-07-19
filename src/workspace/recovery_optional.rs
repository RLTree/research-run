use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::Result;
use crate::domain::{
    CanonicalRecord, InventorySnapshot, KnowledgeRecord, MigrationRecord, RelationshipRecord,
};

use super::Workspace;
use super::recovery_plan::PendingRecord;
use super::recovery_preflight::canonical_records;
use super::storage::ReadBudget;

pub(super) struct OptionalRecovery {
    pub(super) inventories: Vec<InventorySnapshot>,
    pub(super) knowledge: Vec<KnowledgeRecord>,
    pub(super) relationships: Vec<RelationshipRecord>,
    pub(super) migrations: Vec<MigrationRecord>,
    pub(super) inventory_pending: Vec<PendingRecord>,
    pub(super) knowledge_pending: Vec<PendingRecord>,
    pub(super) relationship_pending: Vec<PendingRecord>,
    pub(super) migration_pending: Vec<PendingRecord>,
}

pub(super) fn preflight_optional(
    workspace: &Workspace,
    budget: &mut ReadBudget,
) -> Result<OptionalRecovery> {
    let mut inventory_pending = collect_optional(workspace, "inventories")?;
    let mut knowledge_pending = collect_optional(workspace, "knowledge")?;
    let mut relationship_pending = collect_optional(workspace, "relationships")?;
    let mut migration_pending = collect_optional(workspace, "migrations")?;
    let inventories = records_optional(workspace, "inventories", &mut inventory_pending, budget)?;
    let knowledge = records_optional(workspace, "knowledge", &mut knowledge_pending, budget)?;
    let pending = &mut relationship_pending;
    let relationships = records_optional(workspace, "relationships", pending, budget)?;
    let migrations = records_optional(workspace, "migrations", &mut migration_pending, budget)?;
    Ok(OptionalRecovery {
        inventories,
        knowledge,
        relationships,
        migrations,
        inventory_pending,
        knowledge_pending,
        relationship_pending,
        migration_pending,
    })
}

fn collect_optional(workspace: &Workspace, name: &str) -> Result<Vec<PendingRecord>> {
    let directory = workspace.state.join(name);
    if directory.exists() {
        workspace.collect_recovery_pending(&directory)
    } else {
        Ok(Vec::new())
    }
}

fn records_optional<T>(
    workspace: &Workspace,
    directory: &str,
    pending: &mut [PendingRecord],
    budget: &mut ReadBudget,
) -> Result<Vec<T>>
where
    T: CanonicalRecord + DeserializeOwned + Serialize,
{
    if workspace.state.join(directory).exists() {
        canonical_records(workspace, directory, pending, budget)
    } else {
        Ok(Vec::new())
    }
}
