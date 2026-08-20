use std::fmt::Write;

use crate::Result;
use crate::workspace::HandoffBundle;

use super::current_directory::discover_current;
use super::handoff_arguments::HandoffCommand;
use super::input::read_json_input;
use super::render::{print_json, print_text, terminal_text};
use super::retrieval_commands::render_context;

pub(super) fn execute(command: HandoffCommand) -> Result<()> {
    match command {
        HandoffCommand::Create {
            id,
            generated_at,
            query,
            limit,
            human,
        } => {
            let workspace = discover_current()?;
            let bundle = workspace.handoff(&id, &generated_at, query.as_deref(), limit)?;
            print_bundle(&bundle, human)
        }
        HandoffCommand::Inspect { input, human } => {
            let bundle: HandoffBundle = read_json_input(&input)?;
            bundle.validate()?;
            print_bundle(&bundle, human)
        }
    }
}

fn print_bundle(bundle: &HandoffBundle, human: bool) -> Result<()> {
    if human {
        print_text(&render_human_bundle(bundle))
    } else {
        print_json(bundle)
    }
}

fn render_human_bundle(bundle: &HandoffBundle) -> String {
    let mut output = render_validation(bundle);
    output.push_str(&render_context(&bundle.context));
    output
}

fn render_validation(bundle: &HandoffBundle) -> String {
    let mut output = String::from("Validation receipt\n");
    let Some(receipt) = &bundle.validation else {
        output.push_str(
            "- Not embedded in this legacy v1 handoff; workspace validity and agent integration are unverified.\n",
        );
        return output;
    };
    let result = &receipt.result;
    writeln!(output, "- Workspace valid: {}", result.valid)
        .expect("String rendering is infallible");
    if result.errors.is_empty() {
        output.push_str("- Validation errors: None.\n");
    } else {
        output.push_str("- Validation errors:\n");
        for error in &result.errors {
            writeln!(output, "  - {}", terminal_text(error))
                .expect("String rendering is infallible");
        }
    }
    let integration = &result.agent_integration;
    writeln!(
        output,
        "- Agent integration: {} (scope: {})",
        if integration.agent_integration_ready {
            "ready"
        } else {
            "NOT READY"
        },
        terminal_text(&integration.ready_scope),
    )
    .expect("String rendering is infallible");
    writeln!(
        output,
        "  Protocol installed: {}; instruction contract installed: {}",
        integration.protocol_installed, integration.instruction_contract_installed,
    )
    .expect("String rendering is infallible");
    writeln!(
        output,
        "  Current session load: {}; fresh session required: {}",
        terminal_text(&integration.current_session_loaded),
        integration.fresh_session_required,
    )
    .expect("String rendering is infallible");
    writeln!(
        output,
        "  Diagnostic: {}",
        terminal_text(&integration.diagnostic),
    )
    .expect("String rendering is infallible");
    output
}
