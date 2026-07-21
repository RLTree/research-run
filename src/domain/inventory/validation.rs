use std::collections::BTreeSet;

use crate::domain::{BoundaryEntry, InventoryBoundary, InventoryEntry, InventoryPolicy};
use crate::{Error, Result};

pub(super) fn validate_entries(
    policy: Option<&InventoryPolicy>,
    entries: &[InventoryEntry],
) -> Result<()> {
    let mut paths = BTreeSet::new();
    for entry in entries {
        entry.validate()?;
        if policy.is_none() && !matches!(entry, InventoryEntry::Indexed(_)) {
            return Err(Error::invalid(
                "legacy inventory",
                "policyless records can contain only indexed material",
            ));
        }
        if !paths.insert(entry.path()) {
            return Err(Error::invalid("inventory", "contains duplicate paths"));
        }
    }
    validate_policy_boundaries(policy, entries)
}

fn validate_policy_boundaries(
    policy: Option<&InventoryPolicy>,
    entries: &[InventoryEntry],
) -> Result<()> {
    let Some(policy) = policy else { return Ok(()) };
    let observed = entries.iter().filter_map(|entry| match entry {
        InventoryEntry::Boundary(boundary) => Some(boundary),
        InventoryEntry::Indexed(_) => None,
    });
    if observed.clone().count() != policy.boundaries.len() {
        return Err(Error::invalid(
            "inventory boundaries",
            "every declaration must produce exactly one canonical boundary entry",
        ));
    }
    for (declared, observed) in policy.boundaries.iter().zip(observed) {
        if !boundary_matches(declared, observed) {
            return Err(Error::invalid(
                "inventory boundaries",
                "canonical boundary entries do not match the embedded policy",
            ));
        }
    }
    Ok(())
}

fn boundary_matches(declared: &InventoryBoundary, observed: &BoundaryEntry) -> bool {
    match (declared, observed) {
        (
            InventoryBoundary::ReferenceOnly {
                path,
                class,
                rationale,
                reference,
            },
            BoundaryEntry::ReferenceOnly {
                path: actual_path,
                class: actual_class,
                declared_rationale,
                declared_reference,
                ..
            },
        ) => {
            (path, class, rationale, reference)
                == (
                    actual_path,
                    actual_class,
                    declared_rationale,
                    declared_reference,
                )
        }
        (
            InventoryBoundary::ChildWorkspace { path, rationale },
            BoundaryEntry::ChildWorkspace {
                path: actual_path,
                declared_rationale,
                ..
            },
        ) => (path, rationale) == (actual_path, declared_rationale),
        _ => false,
    }
}
