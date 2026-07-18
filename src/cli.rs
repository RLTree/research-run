use clap::Parser;

mod arguments;
mod commands;
mod render;

use arguments::Cli;
use commands::execute;

pub use render::terminal_text;

use crate::Result;

pub fn run() -> Result<()> {
    execute(Cli::parse())
}

#[cfg(test)]
use arguments::*;
#[cfg(test)]
use commands::{discover_current, inject_current_directory_failure};
#[cfg(test)]
use render::*;

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
