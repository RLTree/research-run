use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(super) enum StructuredCommand {
    Add {
        /// JSON file path, or '-' for standard input.
        #[arg(long)]
        input: String,
    },
}
