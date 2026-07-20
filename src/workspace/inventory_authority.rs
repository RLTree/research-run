use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::domain::{InventoryPlan, InventorySnapshot, ReviewAuthority};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::STATE_DIRECTORY;
use super::inventory::ReviewBootstrap;
use super::path_safety::reject_symlink_chain;
use super::publication::canonical_json_bytes;
use super::status_authority::review_authority_status;
use super::storage::{ReadBudget, map_io, read_json_with_budget, sync_directory};
use super::write_lock::WorkspaceWriteLock;
use super::{ReviewAuthorityStatus, Snapshot, Workspace};

const INVENTORY_BOOTSTRAP_MARKER: &str = "inventory-bootstrap.json";

#[derive(Deserialize, Serialize)]
struct InventoryBootstrapMarker {
    schema_version: u32,
    kind: String,
    plan_sha256: String,
}

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
    let created_state = match fs::create_dir(&state) {
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
    if !created_state && !marker_exists {
        return Workspace::at_exact_root(root)?
            .map(|workspace| (workspace, false))
            .ok_or_else(|| Error::NotFound("workspace manifest is missing".to_owned()));
    }
    let _write_lock = WorkspaceWriteLock::acquire(&workspace.state)?;
    ensure_inventory_bootstrap_marker(&workspace, plan, created_state)?;
    drop(_write_lock);
    initialize_inventory_bootstrap(root, plan)
        .map(|workspace| (workspace, true))
        .map_err(|error| inventory_bootstrap_error(plan, error))
}

pub(super) fn finish_inventory_bootstrap(workspace: &Workspace) -> Result<()> {
    let marker = workspace.state.join(INVENTORY_BOOTSTRAP_MARKER);
    if !marker.exists() {
        return Ok(());
    }
    if let Err(error) = map_io(
        fs::remove_file(&marker),
        "remove inventory bootstrap marker",
        &marker,
    ) {
        return Err(Error::AmbiguousEffect(format!(
            "inventory committed but bootstrap marker cleanup failed: {error}"
        )));
    }
    sync_directory(&workspace.state).map_err(|error| {
        Error::AmbiguousEffect(format!(
            "inventory committed but bootstrap marker cleanup is ambiguous: {error}"
        ))
    })
}

pub(super) fn inventory_bootstrap_error(plan: &InventoryPlan, error: Error) -> Error {
    Error::AmbiguousEffect(format!(
        "inventory bootstrap for {} may be incomplete: {error}; reapply the exact accepted plan",
        plan.id
    ))
}

fn ensure_inventory_bootstrap_marker(
    workspace: &Workspace,
    plan: &InventoryPlan,
    created_state: bool,
) -> Result<()> {
    let path = workspace.state.join(INVENTORY_BOOTSTRAP_MARKER);
    let expected = inventory_bootstrap_marker(plan);
    if path.exists() {
        let mut budget = ReadBudget::default();
        let actual: InventoryBootstrapMarker = read_json_with_budget(&path, &mut budget)?;
        if actual.schema_version != expected.schema_version
            || actual.kind != expected.kind
            || actual.plan_sha256 != expected.plan_sha256
        {
            return Err(Error::Conflict(
                "inventory bootstrap marker belongs to a different plan".to_owned(),
            ));
        }
        return Ok(());
    }
    if !created_state {
        return Err(Error::invalid(
            "inventory bootstrap",
            "partial workspace has no exact-plan recovery marker",
        ));
    }
    workspace
        .publish_value(&path, &expected)
        .map(|_| ())
        .map_err(|error| inventory_bootstrap_error(plan, error))
}

fn initialize_inventory_bootstrap(root: &Path, plan: &InventoryPlan) -> Result<Workspace> {
    match (&plan.review_authority, plan.without_review_authority) {
        (Some(authority), false) => Workspace::initialize_for_inventory(
            root,
            &plan.project_name,
            Some(authority),
            &plan.workspace_id,
        ),
        (None, true) => {
            Workspace::initialize_for_inventory(root, &plan.project_name, None, &plan.workspace_id)
        }
        _ => Err(Error::invalid(
            "inventory plan review authority",
            "new-workspace retrofit requires an authority or explicit unanchored opt-out",
        )),
    }
}

fn inventory_bootstrap_marker(plan: &InventoryPlan) -> InventoryBootstrapMarker {
    InventoryBootstrapMarker {
        schema_version: 1,
        kind: "inventory-bootstrap".to_owned(),
        plan_sha256: format!("{:x}", Sha256::digest(canonical_json_bytes(plan))),
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
