use std::collections::BTreeSet;
use std::ffi::OsStr;

use crate::domain::{CanonicalRecord, ReconciliationKind};
use crate::{Error, Result};

use super::inventory_reconcile::reconcile;
use super::inventory_scan::scan_materials_with_policy;
use super::recovery_plan::PendingRecord;
use super::{Snapshot, Workspace};

pub(super) fn validate_recovered_authority(
    workspace: &Workspace,
    snapshot: &Snapshot,
    inventory_pending: &[PendingRecord],
    migration_pending: &[PendingRecord],
    other_pending: [&[PendingRecord]; 10],
) -> Result<()> {
    validate_inventory(workspace, snapshot, inventory_pending)?;
    validate_migration(workspace, snapshot, migration_pending, other_pending)
}

fn validate_inventory(
    workspace: &Workspace,
    snapshot: &Snapshot,
    pending: &[PendingRecord],
) -> Result<()> {
    let new_ids = new_target_ids(pending)?;
    if new_ids.is_empty() {
        return Ok(());
    }
    if new_ids.len() != 1 {
        return Err(Error::invalid(
            "recovered inventory",
            "one recovery batch may publish at most one new inventory",
        ));
    }
    let candidate = record_by_id(&snapshot.inventories, &new_ids[0], "inventory")?;
    if candidate.project_id != snapshot.manifest.project_id
        || candidate.project_name != snapshot.manifest.name
    {
        return Err(Error::Conflict(
            "recovered inventory identity does not match target workspace".to_owned(),
        ));
    }
    let (_, entries) = scan_materials_with_policy(
        &workspace.root,
        candidate.policy.as_ref(),
        &snapshot.manifest,
    )?;
    let previous = snapshot
        .inventories
        .iter()
        .filter(|record| record.id != candidate.id)
        .max_by(|left, right| (&left.observed_at, &left.id).cmp(&(&right.observed_at, &right.id)));
    let previous_id = previous.map(|record| record.id.as_str());
    let expected_changes = previous
        .map(|record| reconcile(&record.entries, &entries))
        .unwrap_or_default();
    if candidate.previous_snapshot_id.as_deref() != previous_id
        || candidate.entries != entries
        || candidate.changes != expected_changes
        || expected_changes
            .iter()
            .any(|change| change.kind == ReconciliationKind::Conflict)
    {
        return Err(Error::Conflict(
            "recovered inventory does not match current material authority".to_owned(),
        ));
    }
    Ok(())
}

fn validate_migration(
    workspace: &Workspace,
    snapshot: &Snapshot,
    pending: &[PendingRecord],
    other_pending: [&[PendingRecord]; 10],
) -> Result<()> {
    let new_ids = new_target_ids(pending)?;
    if new_ids.is_empty() {
        return Ok(());
    }
    if new_ids.len() != 1
        || snapshot.migrations.len() != 1
        || other_pending
            .iter()
            .flat_map(|records| records.iter())
            .any(|record| !record.target.exists())
    {
        return Err(Error::invalid(
            "recovered migration",
            "migration recovery must be the only new canonical effect",
        ));
    }
    let candidate = record_by_id(&snapshot.migrations, &new_ids[0], "migration")?;
    if candidate.project_id != snapshot.manifest.project_id {
        return Err(Error::Conflict(
            "recovered migration identity does not match target workspace".to_owned(),
        ));
    }
    let expected = workspace.authority_fingerprint()?;
    let actual = (
        candidate.authority_files,
        candidate.authority_bytes,
        candidate.authority_sha256.clone(),
    );
    if actual != expected {
        return Err(Error::Conflict(
            "recovered migration does not match current canonical authority".to_owned(),
        ));
    }
    Ok(())
}

fn new_target_ids(pending: &[PendingRecord]) -> Result<Vec<String>> {
    let ids = pending
        .iter()
        .filter(|record| !record.target.exists())
        .map(|record| {
            record
                .target
                .file_stem()
                .and_then(OsStr::to_str)
                .map(str::to_owned)
                .ok_or_else(|| Error::invalid("recovery target", "record id must be UTF-8"))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    Ok(ids.into_iter().collect())
}

fn record_by_id<'a, T: CanonicalRecord>(records: &'a [T], id: &str, kind: &str) -> Result<&'a T> {
    records
        .iter()
        .find(|record| record.id() == id)
        .ok_or_else(|| Error::invalid("recovery plan", format!("missing typed {kind} candidate")))
}
