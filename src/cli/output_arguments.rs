use std::path::PathBuf;

use clap::Args;

#[derive(Debug, Args)]
pub(super) struct OutputArgs {
    #[arg(long)]
    pub(super) json: bool,
}

#[derive(Debug, Args)]
pub(super) struct RecoveryArgs {
    #[arg(default_value = ".")]
    pub(super) path: PathBuf,
    #[arg(long)]
    pub(super) json: bool,
}
