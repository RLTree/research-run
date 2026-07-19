use std::path::Path;

use crate::domain::{
    FORMAT_VERSION, InventoryPlan, InventorySnapshot, ProjectManifest, ReconciliationKind,
};
use crate::{Error, Result};

use super::inventory_reconcile::reconcile;
use super::inventory_scan::scan_materials;
use super::path_safety::{create_directory_chain, reject_symlink_chain};
use super::storage::{ReadBudget, read_json_with_budget};
use super::write_lock::WorkspaceWriteLock;
use super::{STATE_DIRECTORY, Workspace};

impl Workspace {
    pub fn plan_retrofit(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
    ) -> Result<InventoryPlan> {
        Self::plan_inventory(root, name, id, observed_at, false)
    }

    pub fn plan_reconciliation(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
    ) -> Result<InventoryPlan> {
        Self::plan_inventory(root, name, id, observed_at, true)
    }

    fn plan_inventory(
        root: &Path,
        name: &str,
        id: &str,
        observed_at: &str,
        require_previous: bool,
    ) -> Result<InventoryPlan> {
        let (root, entries) = scan_materials(root)?;
        let requested = ProjectManifest::new(name)?;
        let existing = Self::at_exact_root(&root)?;
        let manifest = match &existing {
            Some(workspace) => workspace.read_manifest()?,
            None => requested,
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
            entries,
            changes,
        };
        plan.validate()?;
        Ok(plan)
    }

    pub fn apply_inventory_plan(root: &Path, plan: InventoryPlan) -> Result<bool> {
        plan.validate()?;
        if plan
            .changes
            .iter()
            .any(|change| change.kind == ReconciliationKind::Conflict)
        {
            return Err(Error::Conflict(
                "reconciliation contains ambiguous identity conflicts".to_owned(),
            ));
        }
        let (root, current) = scan_materials(root)?;
        if current != plan.entries {
            return Err(Error::Conflict(
                "project materials changed after planning; create a fresh plan".to_owned(),
            ));
        }
        let workspace = match Self::at_exact_root(&root)? {
            Some(workspace) => workspace,
            None => Self::initialize(&root, &plan.project_name)?,
        };
        let manifest = workspace.read_manifest()?;
        if manifest.project_id != plan.project_id || manifest.name != plan.project_name {
            return Err(Error::Conflict(
                "inventory plan project identity does not match target workspace".to_owned(),
            ));
        }
        let directory = workspace.state.join("inventories");
        create_directory_chain(&directory)?;
        let _write_lock = WorkspaceWriteLock::acquire(&workspace.state)?;
        let snapshot = InventorySnapshot::from(plan.clone());
        if workspace.record_is_identical("inventories", &snapshot)? {
            return Ok(false);
        }
        let latest = workspace.latest_inventory()?;
        if latest.as_ref().map(|value| value.id.as_str()) != plan.previous_snapshot_id.as_deref() {
            return Err(Error::Conflict(
                "inventory authority changed after planning; create a fresh plan".to_owned(),
            ));
        }
        workspace.publish_record("inventories", &snapshot)
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
