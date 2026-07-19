use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(super) enum InventoryCommand {
    /// Emit a deterministic, read-only JSON plan.
    Plan {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        id: String,
        #[arg(long)]
        observed_at: String,
    },
    /// Apply an unchanged plan after revalidating every indexed project byte.
    Apply {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
}
