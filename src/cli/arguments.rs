use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use super::inventory_arguments::InventoryCommand;
use crate::domain::{Assessment, Authorship, Outcome, SourceProvenance, Stance};

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

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum ProvenanceArg {
    Human,
    Imported,
    Ai,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum AuthorshipArg {
    Human,
    Ai,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum StanceArg {
    Supports,
    Limits,
    Contradicts,
    Context,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum OutcomeArg {
    Positive,
    Negative,
    Ambiguous,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(super) enum ReviewArg {
    Unsupported,
    Limited,
    Supported,
    Contradicted,
}

impl From<ProvenanceArg> for SourceProvenance {
    fn from(value: ProvenanceArg) -> Self {
        match value {
            ProvenanceArg::Human => Self::Human,
            ProvenanceArg::Imported => Self::Imported,
            ProvenanceArg::Ai => Self::Ai,
        }
    }
}

impl From<AuthorshipArg> for Authorship {
    fn from(value: AuthorshipArg) -> Self {
        match value {
            AuthorshipArg::Human => Self::Human,
            AuthorshipArg::Ai => Self::Ai,
        }
    }
}

impl From<StanceArg> for Stance {
    fn from(value: StanceArg) -> Self {
        match value {
            StanceArg::Supports => Self::Supports,
            StanceArg::Limits => Self::Limits,
            StanceArg::Contradicts => Self::Contradicts,
            StanceArg::Context => Self::Context,
        }
    }
}

impl From<OutcomeArg> for Outcome {
    fn from(value: OutcomeArg) -> Self {
        match value {
            OutcomeArg::Positive => Self::Positive,
            OutcomeArg::Negative => Self::Negative,
            OutcomeArg::Ambiguous => Self::Ambiguous,
            OutcomeArg::Inconclusive => Self::Inconclusive,
        }
    }
}

impl From<ReviewArg> for Assessment {
    fn from(value: ReviewArg) -> Self {
        match value {
            ReviewArg::Unsupported => Self::Unsupported,
            ReviewArg::Limited => Self::Limited,
            ReviewArg::Supported => Self::Supported,
            ReviewArg::Contradicted => Self::Contradicted,
        }
    }
}
