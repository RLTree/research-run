use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{MaterialEntry, ReconciliationChange, ReconciliationKind};

pub(super) fn reconcile(
    previous: &[MaterialEntry],
    current: &[MaterialEntry],
) -> Vec<ReconciliationChange> {
    let old = previous
        .iter()
        .map(|entry| (&entry.path, entry))
        .collect::<BTreeMap<_, _>>();
    let new = current
        .iter()
        .map(|entry| (&entry.path, entry))
        .collect::<BTreeMap<_, _>>();
    let mut changes = Vec::new();
    let mut missing = Vec::new();
    let mut added = Vec::new();
    for (path, before) in &old {
        match new.get(path) {
            Some(after) if before.sha256 != after.sha256 || before.bytes != after.bytes => {
                changes.push(change(
                    ReconciliationKind::Changed,
                    Some(path),
                    Some(path),
                    "content fingerprint changed",
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
            Some(&entry.path),
            None,
            "previous material is absent; no deletion inferred",
        ));
    }
    for entry in added {
        changes.push(change(
            ReconciliationKind::Added,
            None,
            Some(&entry.path),
            "new material path observed",
        ));
    }
    duplicate_changes(&mut changes, current);
    changes.sort();
    changes
}

fn pair_moves(
    changes: &mut Vec<ReconciliationChange>,
    missing: &mut Vec<&MaterialEntry>,
    added: &mut Vec<&MaterialEntry>,
) {
    let mut consumed_old = BTreeSet::new();
    let mut consumed_new = BTreeSet::new();
    for before in missing.iter().copied() {
        let candidates = added
            .iter()
            .copied()
            .filter(|after| after.sha256 == before.sha256 && after.bytes == before.bytes)
            .collect::<Vec<_>>();
        if candidates.len() == 1 {
            let after = candidates[0];
            let reverse = missing
                .iter()
                .filter(|other| other.sha256 == after.sha256 && other.bytes == after.bytes)
                .count();
            if reverse == 1 {
                changes.push(change(
                    ReconciliationKind::Moved,
                    Some(&before.path),
                    Some(&after.path),
                    "exact size and SHA-256 identity",
                ));
                consumed_old.insert(before.path.as_str());
                consumed_new.insert(after.path.as_str());
            } else {
                changes.push(change(
                    ReconciliationKind::Conflict,
                    Some(&before.path),
                    Some(&after.path),
                    "move identity is ambiguous",
                ));
            }
        }
    }
    missing.retain(|entry| !consumed_old.contains(entry.path.as_str()));
    added.retain(|entry| !consumed_new.contains(entry.path.as_str()));
}

fn duplicate_changes(changes: &mut Vec<ReconciliationChange>, current: &[MaterialEntry]) {
    let mut by_digest = BTreeMap::<(&str, u64), Vec<&str>>::new();
    for entry in current {
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
