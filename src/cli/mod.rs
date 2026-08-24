use clap::Parser;

mod agent_integration_arguments;
mod agent_integration_commands;
mod arguments;
mod commands;
mod current_directory;
mod handoff_arguments;
mod handoff_commands;
mod input;
mod inventory_arguments;
mod inventory_commands;
mod lifecycle_commands;
mod migration_arguments;
mod migration_commands;
mod output_arguments;
mod render;
mod retrieval_arguments;
mod retrieval_commands;
mod structured_arguments;
mod structured_commands;
mod value_arguments;

use arguments::Cli;
use commands::execute;

pub use render::{print_error, terminal_text};

use crate::Result;

pub fn run() -> Result<()> {
    execute(Cli::parse())
}

#[cfg(test)]
use arguments::*;
#[cfg(test)]
use current_directory::{
    discover_current, inject_current_directory, inject_current_directory_failure,
};
#[cfg(test)]
use handoff_arguments::HandoffCommand;
#[cfg(test)]
use output_arguments::*;
#[cfg(test)]
use render::*;
#[cfg(test)]
use retrieval_arguments::{ContextArgs, LimitArgs, ListArgs, RelatedArgs, SearchArgs, ShowArgs};
#[cfg(test)]
use value_arguments::*;

#[cfg(test)]
#[path = "tests/bootstrap.rs"]
mod bootstrap_tests;
#[cfg(test)]
#[path = "tests/lifecycle.rs"]
mod expanded_lifecycle_tests;
#[cfg(test)]
#[path = "tests/expanded.rs"]
mod expanded_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
