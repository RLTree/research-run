use std::path::Path;

use crate::domain::{
    FORMAT_VERSION, InventoryPlan, InventorySnapshot, ProjectManifest, ReconciliationKind,
};
use crate::{Error, Result};

use super::super::inventory_authority::{
    inventory_apply_workspace, latest_inventory_from_snapshot, verify_inventory_target,
};
use super::super::inventory_bootstrap::{finish_inventory_bootstrap, inventory_bootstrap_error};
use super::super::inventory_reconcile::reconcile;
use super::super::inventory_scan::scan_materials_with_policy;
use super::super::path_safety::create_directory_chain;
use super::super::sshsig::parse_authority_key;
use super::super::write_lock::WorkspaceWriteLock;
use super::super::{InventoryApplyResult, Workspace};

impl Workspace {
    pub fn apply_inventory_plan(root: &Path, plan: InventoryPlan) -> Result<bool> {
        Ok(Self::apply_inventory_plan_with_status(root, plan)?.created)
    }

    pub(crate) fn apply_inventory_plan_with_status(
        root: &Path,
        plan: InventoryPlan,
    ) -> Result<InventoryApplyResult> {
        plan.validate()?;
        if let Some(authority) = &plan.review_authority {
            parse_authority_key(authority)?;
        }
        let manifest = plan_manifest(&plan);
        let (root, current) = scan_materials_with_policy(root, plan.policy.as_ref(), &manifest)?;
        if current != plan.entries {
            return Err(Error::Conflict(
                "project materials changed after planning; create a fresh plan".to_owned(),
            ));
        }
        let (workspace, bootstrap_in_progress) = inventory_apply_workspace(&root, &plan)?;
        let _write_lock = WorkspaceWriteLock::acquire(&workspace.state).map_err(|error| {
            if bootstrap_in_progress {
                inventory_bootstrap_error(&plan, error)
            } else {
                error
            }
        })?;
        let result = Self::apply_inventory_plan_locked(&root, &workspace, &plan);
        match result {
            Ok(result) => {
                if bootstrap_in_progress {
                    finish_inventory_bootstrap(&workspace)
                        .map_err(|error| inventory_bootstrap_error(&plan, error))?;
                }
                Ok(result)
            }
            Err(error) if bootstrap_in_progress => Err(inventory_bootstrap_error(&plan, error)),
            Err(error) => Err(error),
        }
    }

    fn apply_inventory_plan_locked(
        root: &Path,
        workspace: &Workspace,
        plan: &InventoryPlan,
    ) -> Result<InventoryApplyResult> {
        let authority_snapshot = workspace.load_snapshot()?;
        let (_, locked_entries) =
            scan_materials_with_policy(root, plan.policy.as_ref(), &authority_snapshot.manifest)?;
        if super::super::injected_storage_failure("materials changed under lock")
            || locked_entries != plan.entries
        {
            return Err(Error::Conflict("project materials changed while acquiring inventory authority; create a fresh plan".to_owned()));
        }
        let review_authority = verify_inventory_target(workspace, &authority_snapshot, plan)?;
        create_directory_chain(&workspace.state.join("inventories"))?;
        let snapshot = InventorySnapshot::from(plan.clone());
        if workspace.record_is_identical("inventories", &snapshot)? {
            return Ok(InventoryApplyResult {
                created: false,
                review_authority,
            });
        }
        let latest = latest_inventory_from_snapshot(&authority_snapshot);
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
        Ok(InventoryApplyResult {
            created: workspace.publish_record("inventories", &snapshot)?,
            review_authority,
        })
    }
}

fn plan_manifest(plan: &InventoryPlan) -> ProjectManifest {
    ProjectManifest {
        schema_version: FORMAT_VERSION,
        kind: "project-manifest".to_owned(),
        project_id: plan.project_id.clone(),
        workspace_id: plan.workspace_id.clone(),
        name: plan.project_name.clone(),
        declared_roots: vec![".".to_owned()],
        review_authority_id: plan.review_authority.as_ref().map(|value| value.id.clone()),
        review_authority_fingerprint: plan
            .review_authority
            .as_ref()
            .map(|value| value.fingerprint.clone()),
    }
}
