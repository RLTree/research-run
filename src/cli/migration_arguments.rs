use std::path::PathBuf;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(super) enum MigrationCommand {
    /// Emit a read-only lossless migration plan bound to canonical v0.1 bytes.
    Plan {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        migrated_at: String,
    },
    /// Apply an unchanged migration plan and retain every v0.1 record byte.
    Apply {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        json: bool,
    },
}
