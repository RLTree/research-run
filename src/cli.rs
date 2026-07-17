use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::domain::{
    ArtifactLocatorType, ArtifactPointer, Assessment, Authorship, ClaimRecord, EvidenceLink,
    ExperimentReceipt, FORMAT_VERSION, Outcome, ReviewDecision, SourceProvenance, SourceRecord,
    Stance, required_text,
};
use crate::workspace::{Status, Workspace};
use crate::{Error, Result};

#[derive(Debug, Parser)]
#[command(
    name = "research-run",
    version,
    about = "Local-first, provenance-bound research evidence ledger"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Initialize a local research workspace.
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        name: String,
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
    Recover(OutputArgs),
}

#[derive(Debug, Subcommand)]
enum SourceCommand {
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
enum ClaimCommand {
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
enum EvidenceCommand {
    Add(EvidenceArgs),
}

#[derive(Debug, Args)]
struct EvidenceArgs {
    #[arg(long)]
    id: String,
    #[arg(long)]
    claim: String,
    #[arg(long, conflicts_with_all = ["experiment", "artifact"], required_unless_present_any = ["experiment", "artifact"])]
    source: Option<String>,
    #[arg(long, conflicts_with_all = ["source", "artifact"], required_unless_present_any = ["source", "artifact"])]
    experiment: Option<String>,
    #[arg(long, conflicts_with_all = ["source", "experiment"], required_unless_present_any = ["source", "experiment"])]
    artifact: Option<String>,
    #[arg(long, value_enum)]
    stance: StanceArg,
    #[arg(long)]
    specific_evidence: String,
    #[arg(long, value_enum)]
    authorship: AuthorshipArg,
}

#[derive(Debug, Subcommand)]
enum ExperimentCommand {
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
enum ReviewCommand {
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
struct OutputArgs {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProvenanceArg {
    Human,
    Imported,
    Ai,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AuthorshipArg {
    Human,
    Ai,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum StanceArg {
    Supports,
    Limits,
    Contradicts,
    Context,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutcomeArg {
    Positive,
    Negative,
    Ambiguous,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReviewArg {
    Unsupported,
    Limited,
    Supported,
    Contradicted,
}

pub fn run() -> Result<()> {
    execute(Cli::parse())
}

fn execute(cli: Cli) -> Result<()> {
    if let Command::Init { path, name } = &cli.command {
        let workspace = Workspace::initialize(path, name)?;
        println!(
            "Initialized Research Run workspace at {}",
            workspace.root().display()
        );
        return Ok(());
    }
    let current =
        std::env::current_dir().map_err(|error| Error::io("read current directory", ".", error))?;
    let workspace = Workspace::discover(&current)?;
    match cli.command {
        Command::Init { .. } => unreachable!("init returned above"),
        Command::Source { command } => match command {
            SourceCommand::Add {
                id,
                citation,
                locator,
                provenance,
                notes,
            } => {
                let record = SourceRecord {
                    schema_version: FORMAT_VERSION,
                    kind: "source".to_owned(),
                    id,
                    citation,
                    locator,
                    provenance: provenance.into(),
                    notes,
                };
                print_effect("source", &record.id, workspace.add_source(&record)?);
            }
        },
        Command::Claim { command } => match command {
            ClaimCommand::Add {
                id,
                text,
                scope,
                owner,
                authorship,
            } => {
                let record = ClaimRecord {
                    schema_version: FORMAT_VERSION,
                    kind: "claim".to_owned(),
                    id,
                    text,
                    scope,
                    owner,
                    authorship: authorship.into(),
                };
                print_effect("claim", &record.id, workspace.add_claim(&record)?);
            }
        },
        Command::Evidence { command } => match command {
            EvidenceCommand::Add(arguments) => {
                let record = EvidenceLink {
                    schema_version: FORMAT_VERSION,
                    kind: "evidence".to_owned(),
                    id: arguments.id,
                    claim_id: arguments.claim,
                    source_id: arguments.source,
                    experiment_id: arguments.experiment,
                    artifact: arguments.artifact,
                    stance: arguments.stance.into(),
                    specific_evidence: arguments.specific_evidence,
                    authorship: arguments.authorship.into(),
                };
                print_effect("evidence", &record.id, workspace.add_evidence(&record)?);
            }
        },
        Command::Experiment { command } => match command {
            ExperimentCommand::Add {
                id,
                question,
                method_ref,
                observation,
                interpretation,
                limitation,
                outcome,
                next_move,
                artifact,
            } => {
                let artifacts = artifact
                    .iter()
                    .map(|value| parse_artifact(value))
                    .collect::<Result<Vec<_>>>()?;
                let record = ExperimentReceipt {
                    schema_version: FORMAT_VERSION,
                    kind: "experiment".to_owned(),
                    id,
                    question,
                    method_ref,
                    observations: observation,
                    interpretation,
                    limitations: limitation,
                    outcome: outcome.into(),
                    next_move,
                    artifacts,
                };
                print_effect("experiment", &record.id, workspace.add_experiment(&record)?);
            }
        },
        Command::Review { command } => match command {
            ReviewCommand::Add {
                id,
                claim,
                decision,
                rationale,
                reviewer,
            } => {
                let record = ReviewDecision {
                    schema_version: FORMAT_VERSION,
                    kind: "review".to_owned(),
                    id,
                    claim_id: claim,
                    decision: decision.into(),
                    rationale,
                    reviewer,
                };
                print_effect("review", &record.id, workspace.add_review(&record)?);
            }
        },
        Command::Validate(output) => {
            let validation = workspace.validate();
            if output.json {
                print_json(&validation)?;
            } else if validation.valid {
                println!("Workspace is valid.");
                println!(
                    "Counts: {}",
                    validation
                        .counts
                        .iter()
                        .map(|(kind, count)| format!("{kind}={count}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            } else {
                println!("Workspace is invalid:");
                for error in &validation.errors {
                    println!("- {error}");
                }
                return Err(Error::invalid("workspace", "validation failed"));
            }
        }
        Command::Status(output) => {
            let status = workspace.status()?;
            if output.json {
                print_json(&status)?;
            } else {
                print_human_status(&status);
            }
        }
        Command::Recover(output) => {
            let recovery = workspace.recover()?;
            if output.json {
                print_json(&recovery)?;
            } else {
                println!("Recovered publications: {}", recovery.recovered.len());
                println!(
                    "Discarded identical pending files: {}",
                    recovery.discarded_identical.len()
                );
            }
        }
    }
    Ok(())
}

fn parse_artifact(value: &str) -> Result<ArtifactPointer> {
    let (locator_type, rest) = value
        .split_once(':')
        .ok_or_else(|| Error::invalid("artifact pointer", "expected TYPE:LOCATOR:DESCRIPTION"))?;
    let (locator, description) = rest
        .rsplit_once(':')
        .ok_or_else(|| Error::invalid("artifact pointer", "expected TYPE:LOCATOR:DESCRIPTION"))?;
    let locator_type = match locator_type {
        "workspace" => ArtifactLocatorType::Workspace,
        "external" => ArtifactLocatorType::External,
        _ => {
            return Err(Error::invalid(
                "artifact pointer type",
                "expected workspace or external",
            ));
        }
    };
    Ok(ArtifactPointer {
        locator_type,
        locator: required_text(locator, "artifact locator")?,
        description: required_text(description, "artifact description")?,
        digest: None,
    })
}

fn print_effect(kind: &str, id: &str, created: bool) {
    println!(
        "{} {kind} {id}",
        if created { "Added" } else { "Already present" }
    );
}

fn print_json(value: &impl serde::Serialize) -> Result<()> {
    let output = serde_json::to_string_pretty(value)
        .map_err(|_| Error::invalid("JSON projection", "serialization failed"))?;
    println!("{output}");
    Ok(())
}

fn print_human_status(status: &Status) {
    println!(
        "Research Run: {} ({})",
        status.project.name, status.project.id
    );
    println!("Claim ceiling: {}", status.claim_ceiling);
    println!("\nClaims");
    if status.claims.is_empty() {
        println!("- None. Next action: add a claim.");
    }
    for claim in &status.claims {
        println!("- {}: {:?} — {}", claim.id, claim.assessment, claim.text);
        println!("  Scope: {}", claim.scope);
        for blocker in &claim.blockers {
            println!("  Blocker: {blocker}");
        }
        println!("  Next: {}", claim.next_action);
    }
    println!("\nExperiments");
    if status.experiments.is_empty() {
        println!("- None.");
    }
    for experiment in &status.experiments {
        println!("- {}: {:?}", experiment.id, experiment.outcome);
        for observation in &experiment.observations {
            println!("  Observation: {observation}");
        }
        println!("  Interpretation: {}", experiment.interpretation);
        println!("  Next: {}", experiment.next_move);
    }
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

#[cfg(test)]
mod tests {
    use super::parse_artifact;
    use crate::domain::ArtifactLocatorType;

    #[test]
    fn external_artifact_locator_may_contain_colons() {
        let pointer = parse_artifact("external:https://example.invalid/a:Public summary")
            .expect("artifact pointer");
        assert_eq!(pointer.locator_type, ArtifactLocatorType::External);
        assert_eq!(pointer.locator, "https://example.invalid/a");
    }
}
