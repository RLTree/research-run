use clap::Args;

#[derive(Debug, Args)]
pub(super) struct LimitArgs {
    #[arg(long, default_value_t = 50)]
    pub(super) limit: usize,
    #[arg(long)]
    pub(super) human: bool,
}

#[derive(Debug, Args)]
pub(super) struct ListArgs {
    #[arg(long)]
    pub(super) kind: Option<String>,
    #[command(flatten)]
    pub(super) output: LimitArgs,
}

#[derive(Debug, Args)]
pub(super) struct ShowArgs {
    #[arg(long)]
    pub(super) kind: String,
    #[arg(long)]
    pub(super) id: String,
    #[arg(long)]
    pub(super) human: bool,
}

#[derive(Debug, Args)]
pub(super) struct SearchArgs {
    pub(super) query: String,
    #[command(flatten)]
    pub(super) output: LimitArgs,
}

#[derive(Debug, Args)]
pub(super) struct RelatedArgs {
    #[arg(long)]
    pub(super) kind: String,
    #[arg(long)]
    pub(super) id: String,
    #[command(flatten)]
    pub(super) output: LimitArgs,
}

#[derive(Debug, Args)]
pub(super) struct ContextArgs {
    #[arg(long)]
    pub(super) query: Option<String>,
    #[command(flatten)]
    pub(super) output: LimitArgs,
}
