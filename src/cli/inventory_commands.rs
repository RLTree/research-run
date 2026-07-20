use crate::workspace::Workspace;
use crate::{Error, Result};

use super::input::read_text_input;
use super::inventory_arguments::InventoryCommand;
use super::render::{print_json, print_text, terminal_text};

pub(super) fn execute(command: InventoryCommand, reconcile: bool) -> Result<()> {
    match command {
        InventoryCommand::Plan {
            path,
            name,
            id,
            observed_at,
            review_authority_id,
            review_authority_public_key,
            without_review_authority,
        } => {
            let plan = if reconcile {
                if review_authority_id.is_some()
                    || review_authority_public_key.is_some()
                    || without_review_authority
                {
                    return Err(Error::invalid(
                        "review authority",
                        "review authority bootstrap applies only to retrofit",
                    ));
                }
                Workspace::plan_reconciliation(&path, &name, &id, &observed_at)
            } else {
                let bootstrap = match (review_authority_id, review_authority_public_key) {
                    (Some(id), Some(public_key)) => {
                        Some(crate::workspace::ReviewBootstrap::Authority(
                            crate::domain::ReviewAuthority::from_openssh(
                                id,
                                &read_text_input(&public_key)?,
                            )?,
                        ))
                    }
                    (None, None) if without_review_authority => {
                        Some(crate::workspace::ReviewBootstrap::WithoutReviewAuthority)
                    }
                    (None, None) => None,
                    _ => {
                        return Err(Error::invalid(
                            "review authority",
                            "id and public key must be supplied together",
                        ));
                    }
                };
                Workspace::plan_retrofit(&path, &name, &id, &observed_at, bootstrap)
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
            let outcome = Workspace::apply_inventory_plan_with_status(&path, plan)?;
            let created = outcome.created;
            let authority = outcome.review_authority;
            if json {
                print_json(&serde_json::json!({
                    "kind": "inventory-apply",
                    "id": id,
                    "created": created,
                    "review_authority": authority
                }))
            } else {
                print_text(&format!(
                    "{} inventory {}\nReview authority: {}; promotion capable: {}",
                    if created { "Added" } else { "Already present" },
                    terminal_text(&id),
                    terminal_text(authority.mode),
                    authority.promotion_capable
                ))
            }
        }
    }
}
