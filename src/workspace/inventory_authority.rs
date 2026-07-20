use std::path::Path;

use crate::domain::{InventoryPlan, InventorySnapshot, ReviewAuthority};
use crate::{Error, Result};

use super::inventory::ReviewBootstrap;
use super::status_authority::review_authority_status;
use super::{ReviewAuthorityStatus, Snapshot, Workspace};

pub(super) fn verified_inventory_plan_snapshot(
    workspace: Option<&Workspace>,
) -> Result<Option<Snapshot>> {
    let Some(workspace) = workspace else {
        return Ok(None);
    };
    let snapshot = workspace.load_snapshot()?;
    if !workspace.review_authorization_errors(&snapshot).is_empty() {
        return Err(Error::invalid(
            "inventory review authority",
            "existing workspace review authority is not valid",
        ));
    }
    Ok(Some(snapshot))
}

pub(super) fn inventory_plan_review_authority(
    snapshot: Option<&Snapshot>,
    bootstrap: Option<&ReviewBootstrap>,
) -> Result<(Option<ReviewAuthority>, bool)> {
    if super::injected_storage_failure("inventory plan authority") {
        return Err(Error::invalid(
            "inventory review authority",
            "injected plan authority failure",
        ));
    }
    match (bootstrap, snapshot) {
        (Some(ReviewBootstrap::Authority(authority)), None) => Ok((Some(authority.clone()), false)),
        (Some(ReviewBootstrap::WithoutReviewAuthority), None) => Ok((None, true)),
        (None, Some(snapshot)) => match snapshot.review_authorities.as_slice() {
            [authority] => Ok((Some(authority.clone()), false)),
            [] => Ok((None, true)),
            _ => Err(Error::invalid(
                "inventory review authority",
                "existing workspace must have zero or one valid review authority",
            )),
        },
        _ => Err(Error::invalid(
            "inventory review authority",
            "plan authority mode does not match the target workspace",
        )),
    }
}

pub(super) fn inventory_apply_workspace(root: &Path, plan: &InventoryPlan) -> Result<Workspace> {
    match Workspace::at_exact_root(root)? {
        Some(workspace) => Ok(workspace),
        None => match (&plan.review_authority, plan.without_review_authority) {
            (Some(authority), false) => Workspace::initialize_for_inventory(
                root,
                &plan.project_name,
                Some(authority),
                &plan.workspace_id,
            ),
            (None, true) => Workspace::initialize_for_inventory(
                root,
                &plan.project_name,
                None,
                &plan.workspace_id,
            ),
            _ => Err(Error::invalid(
                "inventory plan review authority",
                "new-workspace retrofit requires an authority or explicit unanchored opt-out",
            )),
        },
    }
}

pub(super) fn verify_inventory_target(
    workspace: &Workspace,
    snapshot: &Snapshot,
    plan: &InventoryPlan,
) -> Result<ReviewAuthorityStatus> {
    let manifest = &snapshot.manifest;
    if manifest.project_id != plan.project_id
        || manifest.workspace_id != plan.workspace_id
        || manifest.name != plan.project_name
    {
        return Err(Error::Conflict(
            "inventory plan project or workspace identity does not match target workspace"
                .to_owned(),
        ));
    }
    if !workspace.review_authorization_errors(snapshot).is_empty() {
        return Err(Error::invalid(
            "inventory review authority",
            "target workspace review authority is not valid",
        ));
    }
    match (
        &plan.review_authority,
        plan.without_review_authority,
        snapshot.review_authorities.as_slice(),
    ) {
        (Some(expected), false, [actual]) if actual == expected => {}
        (None, true, []) | (None, false, []) => {}
        _ => {
            return Err(Error::Conflict(
                "inventory plan review authority does not match target workspace".to_owned(),
            ));
        }
    }
    Ok(review_authority_status(snapshot))
}

pub(super) fn latest_inventory_from_snapshot(snapshot: &Snapshot) -> Option<InventorySnapshot> {
    snapshot
        .inventories
        .iter()
        .max_by(|left, right| (&left.observed_at, &left.id).cmp(&(&right.observed_at, &right.id)))
        .cloned()
}
