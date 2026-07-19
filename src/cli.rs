use clap::Parser;

mod arguments;
mod commands;
mod handoff_arguments;
mod handoff_commands;
mod input;
mod inventory_arguments;
mod inventory_commands;
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
use commands::{discover_current, inject_current_directory_failure};
#[cfg(test)]
use output_arguments::*;
#[cfg(test)]
use render::*;
#[cfg(test)]
use value_arguments::*;

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
