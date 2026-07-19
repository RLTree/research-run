use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
    CanonicalRecord, EntityKind, EntityRef, KnowledgeRecord, RelationshipKind, RelationshipRecord,
};
use crate::{Error, Result};

use super::write_lock::WorkspaceWriteLock;
use super::{MAX_RECORDS_PER_KIND, Snapshot, Workspace, injected_storage_failure};

impl Workspace {
    pub fn add_knowledge(&self, record: &KnowledgeRecord) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        let snapshot = self.load_mutable_snapshot()?;
        if self.record_is_identical("knowledge", record)? {
            return Ok(false);
        }
        ensure_capacity(snapshot.knowledge.len(), "knowledge")?;
        self.publish_record("knowledge", record)
    }

    pub fn add_relationship(&self, record: &RelationshipRecord) -> Result<bool> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        record.validate()?;
        let snapshot = self.load_mutable_snapshot()?;
        if self.record_is_identical("relationships", record)? {
            return Ok(false);
        }
        ensure_capacity(snapshot.relationships.len(), "relationships")?;
        if !entity_exists(&snapshot, &record.from) {
            return Err(unknown_reference("from", &record.from));
        }
        if !entity_exists(&snapshot, &record.to) {
            return Err(unknown_reference("to", &record.to));
        }
        if is_history(record.relationship) & creates_history_cycle(&snapshot, record) {
            return Err(Error::invalid(
                "relationship history",
                "supersession, revision, and invalidation must remain acyclic",
            ));
        }
        self.publish_record("relationships", record)
    }
}

pub(super) fn relationship_reference_errors(snapshot: &Snapshot) -> Vec<String> {
    let mut errors = Vec::new();
    for record in &snapshot.relationships {
        if !entity_exists(snapshot, &record.from) {
            errors.push(format!(
                "relationships/{}: unknown from reference",
                record.id
            ));
        }
        if !entity_exists(snapshot, &record.to) {
            errors.push(format!("relationships/{}: unknown to reference", record.id));
        }
    }
    errors
}

fn entity_exists(snapshot: &Snapshot, reference: &EntityRef) -> bool {
    match reference.kind {
        EntityKind::Source => snapshot
            .sources
            .iter()
            .any(|record| record.id == reference.id),
        EntityKind::Claim => snapshot
            .claims
            .iter()
            .any(|record| record.id == reference.id),
        EntityKind::Evidence => snapshot
            .evidence
            .iter()
            .any(|record| record.id == reference.id),
        EntityKind::Experiment => snapshot
            .experiments
            .iter()
            .any(|record| record.id == reference.id),
        EntityKind::Review => snapshot
            .reviews
            .iter()
            .any(|record| record.id == reference.id),
        EntityKind::Knowledge => snapshot
            .knowledge
            .iter()
            .any(|record| record.id == reference.id),
        EntityKind::Inventory => snapshot
            .inventories
            .iter()
            .any(|record| record.id == reference.id),
    }
}

fn creates_history_cycle(snapshot: &Snapshot, candidate: &RelationshipRecord) -> bool {
    let mut edges = BTreeMap::<&str, Vec<&str>>::new();
    for record in snapshot
        .relationships
        .iter()
        .filter(|record| is_history(record.relationship))
    {
        edges
            .entry(&record.from.id)
            .or_default()
            .push(&record.to.id);
    }
    edges
        .entry(&candidate.from.id)
        .or_default()
        .push(&candidate.to.id);
    reaches(
        &edges,
        &candidate.to.id,
        &candidate.from.id,
        &mut BTreeSet::new(),
    )
}

fn reaches<'a>(
    edges: &BTreeMap<&'a str, Vec<&'a str>>,
    current: &'a str,
    target: &str,
    visited: &mut BTreeSet<&'a str>,
) -> bool {
    if current == target {
        return true;
    }
    visited.insert(current)
        && edges
            .get(current)
            .is_some_and(|next| next.iter().any(|id| reaches(edges, id, target, visited)))
}

fn is_history(kind: RelationshipKind) -> bool {
    matches!(
        kind,
        RelationshipKind::Supersedes | RelationshipKind::Revises | RelationshipKind::Invalidates
    )
}

pub(super) fn ensure_capacity(count: usize, directory: &str) -> Result<()> {
    if count >= MAX_RECORDS_PER_KIND
        || injected_storage_failure(if directory == "knowledge" {
            "knowledge capacity"
        } else {
            "relationship capacity"
        })
    {
        return Err(Error::Budget(format!(
            "{directory} already reached the {MAX_RECORDS_PER_KIND} record budget"
        )));
    }
    Ok(())
}

fn unknown_reference(endpoint: &str, reference: &EntityRef) -> Error {
    Error::invalid(
        format!("relationship {endpoint}"),
        format!("unknown {:?} record {}", reference.kind, reference.id),
    )
}
