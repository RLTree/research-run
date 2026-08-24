use crate::Result;
use crate::workspace::Workspace;

use super::migration_arguments::MigrationCommand;
use super::render::{print_effect, print_json, print_text};

pub(super) fn execute(command: MigrationCommand) -> Result<()> {
    match command {
        MigrationCommand::Plan {
            path,
            id,
            migrated_at,
        } => print_json(&Workspace::plan_migration(&path, &id, &migrated_at)?),
        MigrationCommand::Apply { path, input, json } => {
            let plan = Workspace::read_migration_plan(&input)?;
            let id = plan.id.clone();
            let created = Workspace::apply_migration(&path, plan)?;
            let agent_integration =
                Workspace::for_recovery(&path)?.agent_integration_onboarding_status();
            if json {
                print_json(&serde_json::json!({
                    "kind": "migration-apply", "id": id, "created": created,
                    "agent_integration": agent_integration
                }))
            } else {
                print_effect("migration", &id, created)?;
                print_text(&format!(
                    "Agent integration ready: {}\nNext: plan and apply project instruction integration, then start a fresh agent run/session.",
                    agent_integration.agent_integration_ready
                ))
            }
        }
    }
}
