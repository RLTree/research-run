use crate::workspace::Workspace;
use crate::{Error, Result};

use super::inventory_arguments::InventoryCommand;
use super::render::{print_effect, print_json};

pub(super) fn execute(command: InventoryCommand, reconcile: bool) -> Result<()> {
    match command {
        InventoryCommand::Plan {
            path,
            name,
            id,
            observed_at,
        } => {
            let plan = if reconcile {
                Workspace::plan_reconciliation(&path, &name, &id, &observed_at)
            } else {
                Workspace::plan_retrofit(&path, &name, &id, &observed_at)
            }?;
            print_json(&plan)
        }
        InventoryCommand::Apply { path, input, json } => {
            let plan = Workspace::read_inventory_plan(&input)?;
            if reconcile != plan.previous_snapshot_id.is_some() {
                return Err(Error::invalid(
                    "inventory plan",
                    "plan does not match the selected retrofit or reconcile command",
                ));
            }
            let id = plan.id.clone();
            let created = Workspace::apply_inventory_plan(&path, plan)?;
            if json {
                print_json(&serde_json::json!({
                    "kind": "inventory-apply",
                    "id": id,
                    "created": created
                }))
            } else {
                print_effect("inventory", &id, created)
            }
        }
    }
}
