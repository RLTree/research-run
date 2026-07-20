use std::fs;
use std::io::{self, ErrorKind};
use std::path::Path;

use crate::domain::{InventoryPlan, InventorySnapshot, ReviewAuthority};
use crate::{Error, Result};

use super::STATE_DIRECTORY;
use super::inventory::ReviewBootstrap;
use super::inventory_bootstrap::{
    INVENTORY_BOOTSTRAP_MARKER, ensure_inventory_bootstrap_marker, initialize_inventory_bootstrap,
    inventory_bootstrap_error, retryable_empty_bootstrap_state,
};
use super::path_safety::reject_symlink_chain;
use super::status_authority::review_authority_status;
use super::write_lock::WorkspaceWriteLock;
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

pub(super) fn inventory_apply_workspace(
    root: &Path,
    plan: &InventoryPlan,
) -> Result<(Workspace, bool)> {
    let state = root.join(STATE_DIRECTORY);
    reject_symlink_chain(&state)?;
    let creation = if super::injected_storage_failure("create inventory bootstrap state") {
        Err(io::Error::other("injected storage failure"))
    } else {
        fs::create_dir(&state)
    };
    let created_state = match creation {
        Ok(()) => true,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => false,
        Err(error) => return Err(Error::io("create inventory bootstrap state", &state, error)),
    };
    let workspace = Workspace {
        root: root.to_path_buf(),
        state,
    };
    let marker_path = workspace.state.join(INVENTORY_BOOTSTRAP_MARKER);
    let marker_exists = marker_path.exists();
    let retryable_empty_state =
        !created_state && !marker_exists && retryable_empty_bootstrap_state(&workspace)?;
    if !created_state && !marker_exists && !retryable_empty_state {
        let existing = if super::injected_storage_failure(
            "inventory workspace disappeared after inspection",
        ) {
            None
        } else {
            Workspace::at_exact_root(root)?
        };
        return existing
            .map(|workspace| (workspace, false))
            .ok_or_else(|| Error::NotFound("workspace manifest is missing".to_owned()));
    }
    let _write_lock = WorkspaceWriteLock::acquire(&workspace.state)?;
    if retryable_empty_state {
        let unchanged = retryable_empty_bootstrap_state(&workspace).unwrap_or(false);
        if super::injected_storage_failure("inventory bootstrap state changed under lock")
            || !unchanged
        {
            return Err(Error::AmbiguousEffect(
                "inventory bootstrap state changed while acquiring recovery authority".to_owned(),
            ));
        }
    }
    ensure_inventory_bootstrap_marker(&workspace, plan)?;
    drop(_write_lock);
    initialize_inventory_bootstrap(root, plan)
        .map(|workspace| (workspace, true))
        .map_err(|error| inventory_bootstrap_error(plan, error))
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
