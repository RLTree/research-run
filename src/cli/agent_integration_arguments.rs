use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(super) enum AgentIntegrationCommand {
    /// Produce a digest-bound plan without changing project instructions.
    Plan {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Apply a reviewed plan atomically without replacing existing instructions.
    Apply {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Agent integration plan JSON file.
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Distinguish protocol installation from readiness for a new agent run.
    Status {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
}
