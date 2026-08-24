use std::path::Path;

mod apply;

use crate::domain::{
    FORMAT_VERSION, InventoryPlan, InventoryPolicy, InventorySnapshot, ProjectManifest,
    ReviewAuthority,
};
use crate::{Error, Result};

use super::inventory_authority::{
    inventory_plan_review_authority, latest_inventory_from_snapshot,
    verified_inventory_plan_snapshot,
};
use super::inventory_reconcile::reconcile;
use super::inventory_scan::scan_materials_with_policy;
use super::path_safety::{absolute_path, reject_symlink_chain};
use super::storage::{ReadBudget, read_json_with_budget, read_json_with_limit};
use super::{MAX_INVENTORY_RECORD_BYTES, STATE_DIRECTORY, Workspace};

pub enum ReviewBootstrap {
    Authority(ReviewAuthority),
    WithoutReviewAuthority,
}

pub struct InventoryApplyResult {
    pub created: bool,
    pub review_authority: super::ReviewAuthorityStatus,
}

impl Workspace {
    pub fn plan_retrofit(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
        bootstrap: Option<ReviewBootstrap>,
    ) -> Result<InventoryPlan> {
        Self::plan_retrofit_with_policy(root, name, id, observed_at, bootstrap, None)
    }

    pub fn plan_retrofit_with_policy(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
        bootstrap: Option<ReviewBootstrap>,
        policy: Option<InventoryPolicy>,
    ) -> Result<InventoryPlan> {
        Self::plan_inventory(root, name, id, observed_at, false, bootstrap, policy)
    }

    pub fn plan_reconciliation(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
    ) -> Result<InventoryPlan> {
        Self::plan_reconciliation_with_policy(root, name, id, observed_at, None)
    }

    pub fn plan_reconciliation_with_policy(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
        policy: Option<InventoryPolicy>,
    ) -> Result<InventoryPlan> {
        Self::plan_inventory(root, name, id, observed_at, true, None, policy)
    }

    fn plan_inventory(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
        require_previous: bool,
        bootstrap: Option<ReviewBootstrap>,
        requested_policy: Option<InventoryPolicy>,
    ) -> Result<InventoryPlan> {
        let root = absolute_path(root)?;
        reject_symlink_chain(&root)?;
        let existing = Self::at_exact_root(&root)?;
        let existing_snapshot = verified_inventory_plan_snapshot(existing.as_ref())?;
        let manifest = match (&existing_snapshot, &bootstrap) {
            (Some(snapshot), None) => snapshot.manifest.clone(),
            (Some(_), Some(_)) => {
                return Err(Error::Conflict(
                    "workspace already exists; review authority bootstrap applies only to a new retrofit"
                        .to_owned(),
                ));
            }
            (None, Some(_)) => ProjectManifest::new(name)?,
            (None, None) => {
                return Err(Error::invalid(
                    "retrofit review authority",
                    "supply a review authority or explicitly opt out before planning a new workspace",
                ));
            }
        };
        if manifest.name != name {
            return Err(Error::Conflict(format!(
                "project name does not match existing manifest: {}",
                manifest.name
            )));
        }
        let previous = existing_snapshot
            .as_ref()
            .and_then(latest_inventory_from_snapshot);
        if !require_previous && previous.is_some() {
            return Err(Error::Conflict(
                "project already has an inventory; reapply its accepted plan or use reconcile"
                    .to_owned(),
            ));
        }
        if require_previous && previous.is_none() {
            return Err(Error::NotFound(
                "no prior inventory exists; run retrofit first".to_owned(),
            ));
        }
        let policy = resolve_inventory_policy(requested_policy, previous.as_ref())?;
        let (_, entries) = scan_materials_with_policy(&root, policy.as_ref(), &manifest)?;
        let changes = previous
            .as_ref()
            .map(|snapshot| reconcile(&snapshot.entries, &entries))
            .unwrap_or_default();
        let (review_authority, without_review_authority) =
            inventory_plan_review_authority(existing_snapshot.as_ref(), bootstrap.as_ref())?;
        let plan = InventoryPlan {
            schema_version: FORMAT_VERSION,
            kind: "inventory-plan".to_owned(),
            id: id.to_owned(),
            project_name: manifest.name,
            project_id: manifest.project_id,
            workspace_id: manifest.workspace_id,
            observed_at: observed_at.to_owned(),
            previous_snapshot_id: previous.map(|snapshot| snapshot.id),
            review_authority,
            without_review_authority,
            policy,
            entries,
            changes,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn read_inventory_plan(path: &Path) -> Result<InventoryPlan> {
        reject_symlink_chain(path)?;
        let mut budget = ReadBudget::default();
        let plan = read_json_with_limit(path, &mut budget, MAX_INVENTORY_RECORD_BYTES)?;
        InventoryPlan::validate(&plan)?;
        Ok(plan)
    }

    pub fn read_inventory_policy(path: &Path) -> Result<InventoryPolicy> {
        reject_symlink_chain(path)?;
        let mut budget = ReadBudget::default();
        let policy: InventoryPolicy = read_json_with_budget(path, &mut budget)?;
        policy.canonicalized()
    }

    pub fn latest_inventory(&self) -> Result<Option<InventorySnapshot>> {
        let mut inventories = self.load_snapshot()?.inventories;
        inventories.sort_by(|left, right| {
            (&left.observed_at, &left.id).cmp(&(&right.observed_at, &right.id))
        });
        Ok(inventories.pop())
    }

    pub(super) fn at_exact_root(root: &Path) -> Result<Option<Self>> {
        let state = root.join(STATE_DIRECTORY);
        reject_symlink_chain(&state)?;
        if !state.exists() {
            return Ok(None);
        }
        if !state.is_dir() {
            return Err(Error::invalid("workspace state", "must be a directory"));
        }
        let workspace = Self {
            root: root.to_path_buf(),
            state,
        };
        workspace.read_manifest()?;
        Ok(Some(workspace))
    }
}

fn resolve_inventory_policy(
    requested: Option<InventoryPolicy>,
    previous: Option<&InventorySnapshot>,
) -> Result<Option<InventoryPolicy>> {
    match requested {
        Some(policy) => policy.canonicalized().map(Some),
        None => match previous {
            Some(previous) => Ok(previous.policy.clone()),
            None => InventoryPolicy::default().canonicalized().map(Some),
        },
    }
}
