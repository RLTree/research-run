use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{InventoryEntry, ReconciliationChange, ReconciliationKind};

pub(super) fn reconcile(
    previous: &[InventoryEntry],
    current: &[InventoryEntry],
) -> Vec<ReconciliationChange> {
    let old = previous
        .iter()
        .map(|entry| (entry.path(), entry))
        .collect::<BTreeMap<_, _>>();
    let new = current
        .iter()
        .map(|entry| (entry.path(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut changes = Vec::new();
    let mut missing = Vec::new();
    let mut added = Vec::new();
    for (path, before) in &old {
        match new.get(path) {
            Some(after) if before != after => {
                changes.push(change(
                    ReconciliationKind::Changed,
                    Some(path),
                    Some(path),
                    reconciliation_detail(before, after),
                ));
            }
            Some(_) => {}
            None => missing.push(*before),
        }
    }
    for (path, after) in &new {
        if !old.contains_key(path) {
            added.push(*after);
        }
    }
    pair_moves(&mut changes, &mut missing, &mut added);
    for entry in missing {
        changes.push(change(
            ReconciliationKind::Missing,
            Some(entry.path()),
            None,
            "previous material is absent; no deletion inferred",
        ));
    }
    for entry in added {
        changes.push(change(
            ReconciliationKind::Added,
            None,
            Some(entry.path()),
            "new material path observed",
        ));
    }
    duplicate_changes(&mut changes, current);
    changes.sort();
    changes
}

fn pair_moves(
    changes: &mut Vec<ReconciliationChange>,
    missing: &mut Vec<&InventoryEntry>,
    added: &mut Vec<&InventoryEntry>,
) {
    let mut consumed_old = BTreeSet::new();
    let mut consumed_new = BTreeSet::new();
    let missing_by_material = material_groups(missing);
    let added_by_material = material_groups(added);
    for (identity, befores) in missing_by_material {
        let Some(afters) = added_by_material.get(&identity) else {
            continue;
        };
        if afters.len() != 1 {
            continue;
        }
        let after = afters[0];
        if befores.len() == 1 {
            let before = befores[0];
            changes.push(change(
                ReconciliationKind::Moved,
                Some(before.path()),
                Some(after.path()),
                "exact size and SHA-256 identity",
            ));
            consumed_old.insert(before.path());
            consumed_new.insert(after.path());
        } else {
            for before in befores {
                changes.push(change(
                    ReconciliationKind::Conflict,
                    Some(before.path()),
                    Some(after.path()),
                    "move identity is ambiguous",
                ));
            }
        }
    }
    missing.retain(|entry| !consumed_old.contains(entry.path()));
    added.retain(|entry| !consumed_new.contains(entry.path()));
}

fn material_groups<'a>(
    entries: &[&'a InventoryEntry],
) -> BTreeMap<(&'a str, u64), Vec<&'a InventoryEntry>> {
    let mut groups = BTreeMap::new();
    for &entry in entries {
        if let Some(material) = entry.indexed() {
            groups
                .entry((material.sha256.as_str(), material.bytes))
                .or_insert_with(Vec::new)
                .push(entry);
        }
    }
    groups
}

fn duplicate_changes(changes: &mut Vec<ReconciliationChange>, current: &[InventoryEntry]) {
    let mut by_digest = BTreeMap::<(&str, u64), Vec<&str>>::new();
    for entry in current.iter().filter_map(InventoryEntry::indexed) {
        by_digest
            .entry((&entry.sha256, entry.bytes))
            .or_default()
            .push(&entry.path);
    }
    for paths in by_digest.values_mut() {
        paths.sort_unstable();
        if paths.len() > 1 {
            changes.push(change(
                ReconciliationKind::Duplicate,
                Some(paths[0]),
                Some(paths[1]),
                "multiple current paths have exact size and SHA-256 identity",
            ));
        }
    }
}

fn reconciliation_detail(before: &InventoryEntry, after: &InventoryEntry) -> &'static str {
    match (before, after) {
        (InventoryEntry::Indexed(_), InventoryEntry::Indexed(_)) => "content fingerprint changed",
        (InventoryEntry::Boundary(_), InventoryEntry::Boundary(_)) => {
            "boundary declaration or observed identity changed"
        }
        _ => "inventory path changed epistemic classification",
    }
}

fn change(
    kind: ReconciliationKind,
    before: Option<&str>,
    after: Option<&str>,
    detail: &str,
) -> ReconciliationChange {
    ReconciliationChange {
        kind,
        before: before.map(str::to_owned),
        after: after.map(str::to_owned),
        detail: detail.to_owned(),
    }
}
