use crate::workspace::Workspace;
use crate::{Error, Result};

use super::input::read_review_authority_pair;
use super::inventory_arguments::InventoryCommand;
use super::render::{print_json, print_text, terminal_text};

pub(super) fn execute(command: InventoryCommand, reconcile: bool) -> Result<()> {
    match command {
        command @ InventoryCommand::Plan { .. } => execute_plan(command, reconcile),
        InventoryCommand::Apply { path, input, json } => {
            execute_apply(&path, &input, json, reconcile)
        }
    }
}

fn execute_plan(command: InventoryCommand, reconcile: bool) -> Result<()> {
    match command {
        InventoryCommand::Plan {
            path,
            name,
            id,
            observed_at,
            policy,
            review_authority_id,
            review_authority_public_key,
            without_review_authority,
        } => {
            let policy = policy
                .as_deref()
                .map(Workspace::read_inventory_policy)
                .transpose()?;
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
                Workspace::plan_reconciliation_with_policy(&path, &name, &id, &observed_at, policy)
            } else {
                let bootstrap = match read_review_authority_pair(
                    review_authority_id,
                    review_authority_public_key,
                )? {
                    Some(authority) => {
                        Some(crate::workspace::ReviewBootstrap::Authority(authority))
                    }
                    None if without_review_authority => {
                        Some(crate::workspace::ReviewBootstrap::WithoutReviewAuthority)
                    }
                    None => None,
                };
                Workspace::plan_retrofit_with_policy(
                    &path,
                    &name,
                    &id,
                    &observed_at,
                    bootstrap,
                    policy,
                )
            }?;
            print_json(&plan)
        }
        InventoryCommand::Apply { .. } => unreachable!("plan dispatch selected apply command"),
    }
}

fn execute_apply(
    path: &std::path::Path,
    input: &std::path::Path,
    json: bool,
    reconcile: bool,
) -> Result<()> {
    let plan = Workspace::read_inventory_plan(input)?;
    if reconcile != plan.previous_snapshot_id.is_some() {
        return Err(Error::invalid(
            "inventory plan",
            "plan does not match the selected retrofit or reconcile command",
        ));
    }
    let id = plan.id.clone();
    let outcome = Workspace::apply_inventory_plan_with_status(path, plan)?;
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
