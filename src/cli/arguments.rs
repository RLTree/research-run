use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use super::handoff_arguments::HandoffCommand;
use super::inventory_arguments::InventoryCommand;
use super::output_arguments::{OutputArgs, RecoveryArgs};
use super::retrieval_arguments::{
    ContextArgs, LimitArgs, ListArgs, RelatedArgs, SearchArgs, ShowArgs,
};
use super::structured_arguments::StructuredCommand;
use super::value_arguments::{AuthorshipArg, OutcomeArg, ProvenanceArg, ReviewArg, StanceArg};

#[derive(Debug, Parser)]
#[command(
    name = "research-run",
    version,
    about = "Local-first, provenance-bound research evidence ledger"
)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Command,
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    /// Initialize a local research workspace.
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        name: String,
    },
    /// Inventory a populated project and initialize managed state only on apply.
    Retrofit {
        #[command(subcommand)]
        command: InventoryCommand,
    },
    /// Compare current project materials with the last accepted inventory.
    Reconcile {
        #[command(subcommand)]
        command: InventoryCommand,
    },
    /// Manage source records.
    Source {
        #[command(subcommand)]
        command: SourceCommand,
    },
    /// Manage claim records.
    Claim {
        #[command(subcommand)]
        command: ClaimCommand,
    },
    /// Connect specific evidence to one claim.
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    /// Record experiments with observations separate from interpretation.
    Experiment {
        #[command(subcommand)]
        command: ExperimentCommand,
    },
    /// Add explicit human review decisions.
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    /// Add typed goals, questions, methods, observations, analyses, and work state.
    Knowledge {
        #[command(subcommand)]
        command: StructuredCommand,
    },
    /// Connect canonical records with a typed append-only relationship.
    Relationship {
        #[command(subcommand)]
        command: StructuredCommand,
    },
    /// List bounded canonical and indexed workspace records.
    List(ListArgs),
    /// Show one canonical record projection by kind and id.
    Show(ShowArgs),
    /// Search deterministic bounded record text with match explanations.
    Search(SearchArgs),
    /// Show newest timestamped records first.
    Recent(LimitArgs),
    /// Show timestamped records in chronological order.
    Timeline(LimitArgs),
    /// Show typed relationships connected to one record.
    Related(RelatedArgs),
    /// Show open questions, risks, uncertainties, and contradictions.
    Unresolved(LimitArgs),
    /// Show open typed blockers.
    Blockers(LimitArgs),
    /// Show open typed next actions.
    Next(LimitArgs),
    /// Build a bounded fresh-agent context projection.
    Context(ContextArgs),
    /// Create or inspect a portable fresh-agent handoff bundle.
    Handoff {
        #[command(subcommand)]
        command: HandoffCommand,
    },
    /// Validate canonical records, references, budgets, and paths.
    Validate(OutputArgs),
    /// Show deterministic current state and claim ceilings.
    Status(OutputArgs),
    /// Inspect and finish interrupted atomic publications.
    Recover(RecoveryArgs),
}

#[derive(Debug, Subcommand)]
pub(super) enum SourceCommand {
    Add {
        #[arg(long)]
        id: String,
        #[arg(long)]
        citation: String,
        #[arg(long)]
        locator: String,
        #[arg(long, value_enum)]
        provenance: ProvenanceArg,
        #[arg(long, default_value = "")]
        notes: String,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum ClaimCommand {
    Add {
        #[arg(long)]
        id: String,
        #[arg(long)]
        text: String,
        #[arg(long)]
        scope: String,
        #[arg(long)]
        owner: String,
        #[arg(long, value_enum)]
        authorship: AuthorshipArg,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum EvidenceCommand {
    Add(EvidenceArgs),
}

#[derive(Debug, Args)]
pub(super) struct EvidenceArgs {
    #[arg(long)]
    pub(super) id: String,
    #[arg(long)]
    pub(super) claim: String,
    #[arg(long, conflicts_with_all = ["experiment", "artifact"], required_unless_present_any = ["experiment", "artifact"])]
    pub(super) source: Option<String>,
    #[arg(long, conflicts_with_all = ["source", "artifact"], required_unless_present_any = ["source", "artifact"])]
    pub(super) experiment: Option<String>,
    #[arg(long, conflicts_with_all = ["source", "experiment"], required_unless_present_any = ["source", "experiment"])]
    pub(super) artifact: Option<String>,
    #[arg(long, value_enum)]
    pub(super) stance: StanceArg,
    #[arg(long)]
    pub(super) specific_evidence: String,
    #[arg(long, value_enum)]
    pub(super) authorship: AuthorshipArg,
}

#[derive(Debug, Subcommand)]
pub(super) enum ExperimentCommand {
    Add {
        #[arg(long)]
        id: String,
        #[arg(long)]
        question: String,
        #[arg(long)]
        method_ref: String,
        #[arg(long, required = true)]
        observation: Vec<String>,
        #[arg(long)]
        interpretation: String,
        #[arg(long, required = true)]
        limitation: Vec<String>,
        #[arg(long, value_enum)]
        outcome: OutcomeArg,
        #[arg(long)]
        next_move: String,
        #[arg(long)]
        artifact: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum ReviewCommand {
    Add {
        #[arg(long)]
        id: String,
        #[arg(long)]
        claim: String,
        #[arg(long, value_enum)]
        decision: ReviewArg,
        #[arg(long)]
        rationale: String,
        #[arg(long)]
        reviewer: String,
    },
}
