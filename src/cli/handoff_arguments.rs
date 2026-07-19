use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(super) enum HandoffCommand {
    /// Generate a bounded deterministic handoff from the current workspace.
    Create {
        #[arg(long)]
        id: String,
        #[arg(long)]
        generated_at: String,
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
        #[arg(long)]
        human: bool,
    },
    /// Validate and inspect a handoff without requiring its source workspace.
    Inspect {
        /// JSON file path, or '-' for standard input.
        #[arg(long)]
        input: String,
        #[arg(long)]
        human: bool,
    },
}
