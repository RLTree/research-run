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
        /// Stable identifier to anchor if retrofit creates the workspace.
        #[arg(long, requires = "review_authority_public_key")]
        review_authority_id: Option<String>,
        /// OpenSSH Ed25519 public key file, or - for standard input.
        #[arg(long, requires = "review_authority_id")]
        review_authority_public_key: Option<String>,
        /// Irreversibly create a non-promoting workspace.
        #[arg(
            long,
            conflicts_with_all = ["review_authority_id", "review_authority_public_key"]
        )]
        without_review_authority: bool,
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
