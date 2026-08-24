use crate::Result;
use crate::workspace::Workspace;

use super::agent_integration_arguments::AgentIntegrationCommand;
use super::render::{print_json, print_text, terminal_text};

pub(super) fn execute(command: AgentIntegrationCommand) -> Result<()> {
    match command {
        AgentIntegrationCommand::Plan { path } => {
            print_json(&Workspace::plan_agent_integration(&path)?)
        }
        AgentIntegrationCommand::Apply { path, input, json } => {
            let plan = Workspace::read_agent_integration_plan(&input)?;
            let result = Workspace::apply_agent_integration(&path, plan)?;
            if json {
                print_json(&result)
            } else {
                print_text(&format!(
                    "Project instruction contract {} at {}.\nCurrent session load: {}. Start a fresh agent run/session before claiming readiness.",
                    if result.changed {
                        "installed"
                    } else {
                        "already installed"
                    },
                    terminal_text(&result.instruction_path),
                    terminal_text(result.current_session_loaded),
                ))
            }
        }
        AgentIntegrationCommand::Status { path, json } => {
            let status = Workspace::for_recovery(&path)?.agent_integration_status()?;
            if json {
                print_json(&status)
            } else {
                print_text(&format!(
                    "Canonical protocol installed: {}\nProject instruction contract installed: {}\nReady scope: {}\nCurrent session load: {}\n{}",
                    status.protocol_installed,
                    status.instruction_contract_installed,
                    status.ready_scope,
                    status.current_session_loaded,
                    terminal_text(&status.diagnostic),
                ))
            }
        }
    }
}
