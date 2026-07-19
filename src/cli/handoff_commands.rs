use crate::Result;
use crate::workspace::HandoffBundle;

use super::current_directory::discover_current;
use super::handoff_arguments::HandoffCommand;
use super::input::read_json_input;
use super::render::{print_json, print_text};
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
        print_text(&render_context(&bundle.context))
    } else {
        print_json(bundle)
    }
}
