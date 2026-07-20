use std::path::Path;

use crate::domain::{
    FORMAT_VERSION, InventoryPlan, InventorySnapshot, ProjectManifest, ReconciliationKind,
    ReviewAuthority,
};
use crate::{Error, Result};

use super::inventory_reconcile::reconcile;
use super::inventory_scan::scan_materials;
use super::path_safety::{create_directory_chain, reject_symlink_chain};
use super::status_authority::review_authority_status_from_manifest;
use super::storage::{ReadBudget, read_json_with_budget};
use super::write_lock::WorkspaceWriteLock;
use super::{InventoryApplyResult, STATE_DIRECTORY, Workspace};

pub enum ReviewBootstrap {
    Authority(ReviewAuthority),
    WithoutReviewAuthority,
}

impl Workspace {
    pub fn plan_retrofit(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
        bootstrap: Option<ReviewBootstrap>,
    ) -> Result<InventoryPlan> {
        Self::plan_inventory(root, name, id, observed_at, false, bootstrap)
    }

    pub fn plan_reconciliation(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
    ) -> Result<InventoryPlan> {
        Self::plan_inventory(root, name, id, observed_at, true, None)
    }

    fn plan_inventory(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
        require_previous: bool,
        bootstrap: Option<ReviewBootstrap>,
    ) -> Result<InventoryPlan> {
        let (root, entries) = scan_materials(root)?;
        let requested = ProjectManifest::new(name)?;
        let existing = Self::at_exact_root(&root)?;
        let manifest = match (&existing, &bootstrap) {
            (Some(workspace), None) => workspace.read_manifest()?,
            (Some(_), Some(_)) => {
                return Err(Error::Conflict(
                    "workspace already exists; review authority bootstrap applies only to a new retrofit"
                        .to_owned(),
                ));
            }
            (None, Some(_)) => requested,
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
        let previous = existing
            .as_ref()
            .map(Self::latest_inventory)
            .transpose()?
            .flatten();
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
        let changes = previous
            .as_ref()
            .map(|snapshot| reconcile(&snapshot.entries, &entries))
            .unwrap_or_default();
        let plan = InventoryPlan {
            schema_version: FORMAT_VERSION,
            kind: "inventory-plan".to_owned(),
            id: id.to_owned(),
            project_name: manifest.name,
            project_id: manifest.project_id,
            observed_at: observed_at.to_owned(),
            previous_snapshot_id: previous.map(|snapshot| snapshot.id),
            review_authority: match &bootstrap {
                Some(ReviewBootstrap::Authority(authority)) => Some(authority.clone()),
                _ => None,
            },
            without_review_authority: matches!(
                bootstrap,
                Some(ReviewBootstrap::WithoutReviewAuthority)
            ),
            entries,
            changes,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn apply_inventory_plan(root: &Path, plan: InventoryPlan) -> Result<bool> {
        Ok(Self::apply_inventory_plan_with_status(root, plan)?.created)
    }

    pub(crate) fn apply_inventory_plan_with_status(
        root: &Path,
        plan: InventoryPlan,
    ) -> Result<InventoryApplyResult> {
        plan.validate()?;
        let (root, current) = scan_materials(root)?;
        if current != plan.entries {
            return Err(Error::Conflict(
                "project materials changed after planning; create a fresh plan".to_owned(),
            ));
        }
        let workspace = inventory_apply_workspace(&root, &plan)?;
        let manifest = workspace.read_manifest()?;
        if manifest.project_id != plan.project_id || manifest.name != plan.project_name {
            return Err(Error::Conflict(
                "inventory plan project identity does not match target workspace".to_owned(),
            ));
        }
        let directory = workspace.state.join("inventories");
        create_directory_chain(&directory)?;
        let _write_lock = WorkspaceWriteLock::acquire(&workspace.state)?;
        let (_, locked_entries) = scan_materials(&root)?;
        if super::injected_storage_failure("materials changed under lock")
            || locked_entries != plan.entries
        {
            return Err(Error::Conflict(
                "project materials changed while acquiring inventory authority; create a fresh plan"
                    .to_owned(),
            ));
        }
        let snapshot = InventorySnapshot::from(plan.clone());
        if workspace.record_is_identical("inventories", &snapshot)? {
            return Ok(InventoryApplyResult {
                created: false,
                review_authority: review_authority_status_from_manifest(&manifest),
            });
        }
        let latest = workspace.latest_inventory()?;
        if latest.as_ref().map(|value| value.id.as_str()) != plan.previous_snapshot_id.as_deref() {
            return Err(Error::Conflict(
                "inventory authority changed after planning; create a fresh plan".to_owned(),
            ));
        }
        let expected_changes = latest
            .as_ref()
            .map(|snapshot| reconcile(&snapshot.entries, &locked_entries))
            .unwrap_or_default();
        if plan.changes != expected_changes {
            return Err(Error::Conflict(
                "inventory reconciliation changes do not match current canonical authority"
                    .to_owned(),
            ));
        }
        if expected_changes
            .iter()
            .any(|change| change.kind == ReconciliationKind::Conflict)
        {
            return Err(Error::Conflict(
                "reconciliation contains ambiguous identity conflicts".to_owned(),
            ));
        }
        let created = workspace.publish_record("inventories", &snapshot)?;
        Ok(InventoryApplyResult {
            created,
            review_authority: review_authority_status_from_manifest(&manifest),
        })
    }

    pub fn read_inventory_plan(path: &Path) -> Result<InventoryPlan> {
        reject_symlink_chain(path)?;
        let mut budget = ReadBudget::default();
        let plan = read_json_with_budget(path, &mut budget)?;
        InventoryPlan::validate(&plan)?;
        Ok(plan)
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

fn inventory_apply_workspace(root: &Path, plan: &InventoryPlan) -> Result<Workspace> {
    match Workspace::at_exact_root(root)? {
        Some(workspace) => Ok(workspace),
        None => match (&plan.review_authority, plan.without_review_authority) {
            (Some(authority), false) => {
                Workspace::initialize_with_review_authority(root, &plan.project_name, authority)
            }
            (None, true) => Workspace::initialize(root, &plan.project_name),
            _ => Err(Error::invalid(
                "inventory plan review authority",
                "new-workspace retrofit requires an authority or explicit unanchored opt-out",
            )),
        },
    }
}
