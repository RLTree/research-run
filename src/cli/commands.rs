use std::path::Path;

use crate::domain::{
    ClaimRecord, EvidenceLink, ExperimentReceipt, FORMAT_VERSION, ReviewDecision, SourceRecord,
};
use crate::workspace::Workspace;
use crate::{Error, Result};

use super::arguments::{
    ClaimCommand, Cli, Command, EvidenceCommand, ExperimentCommand, RecoveryArgs, ReviewCommand,
    SourceCommand,
};
use super::render::{
    parse_artifact, print_effect, print_human_status, print_json, print_text, terminal_text,
};

pub(super) fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init { path, name } => initialize(&path, &name),
        Command::Recover(output) => recover(output),
        Command::Source { command } => add_source(command),
        Command::Claim { command } => add_claim(command),
        Command::Evidence { command } => add_evidence(command),
        Command::Experiment { command } => add_experiment(command),
        Command::Review { command } => add_review(command),
        Command::Validate(output) => validate(output.json),
        Command::Status(output) => status(output.json),
    }
}

fn initialize(path: &Path, name: &str) -> Result<()> {
    let workspace = Workspace::initialize(path, name)?;
    print_text(&format!(
        "Initialized Research Run workspace at {}",
        terminal_text(&workspace.root().display().to_string())
    ))
}

fn recover(output: RecoveryArgs) -> Result<()> {
    let workspace = Workspace::for_recovery(&output.path)?;
    let recovery = workspace.recover()?;
    if output.json {
        print_json(&recovery)
    } else {
        print_text(&format!(
            "Recovered publications: {}\nDiscarded identical pending files: {}",
            recovery.recovered.len(),
            recovery.discarded_identical.len()
        ))
    }
}

fn add_source(command: SourceCommand) -> Result<()> {
    let SourceCommand::Add {
        id,
        citation,
        locator,
        provenance,
        notes,
    } = command;
    let record = SourceRecord {
        schema_version: FORMAT_VERSION,
        kind: "source".to_owned(),
        id,
        citation,
        locator,
        provenance: provenance.into(),
        notes,
    };
    let workspace = discover_current()?;
    print_effect("source", &record.id, workspace.add_source(&record)?)
}

fn add_claim(command: ClaimCommand) -> Result<()> {
    let ClaimCommand::Add {
        id,
        text,
        scope,
        owner,
        authorship,
    } = command;
    let record = ClaimRecord {
        schema_version: FORMAT_VERSION,
        kind: "claim".to_owned(),
        id,
        text,
        scope,
        owner,
        authorship: authorship.into(),
    };
    let workspace = discover_current()?;
    print_effect("claim", &record.id, workspace.add_claim(&record)?)
}

fn add_evidence(command: EvidenceCommand) -> Result<()> {
    let EvidenceCommand::Add(fields) = command;
    let record = EvidenceLink {
        schema_version: FORMAT_VERSION,
        kind: "evidence".to_owned(),
        id: fields.id,
        claim_id: fields.claim,
        source_id: fields.source,
        experiment_id: fields.experiment,
        artifact: fields.artifact,
        stance: fields.stance.into(),
        specific_evidence: fields.specific_evidence,
        authorship: fields.authorship.into(),
    };
    let workspace = discover_current()?;
    print_effect("evidence", &record.id, workspace.add_evidence(&record)?)
}

fn add_experiment(command: ExperimentCommand) -> Result<()> {
    let ExperimentCommand::Add {
        id,
        question,
        method_ref,
        observation,
        interpretation,
        limitation,
        outcome,
        next_move,
        artifact,
    } = command;
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
    let workspace = discover_current()?;
    print_effect("experiment", &record.id, workspace.add_experiment(&record)?)
}

fn add_review(command: ReviewCommand) -> Result<()> {
    let ReviewCommand::Add {
        id,
        claim,
        decision,
        rationale,
        reviewer,
    } = command;
    let workspace = discover_current()?;
    let evidence_ids = workspace.claim_evidence_ids(&claim)?;
    let record = ReviewDecision {
        schema_version: FORMAT_VERSION,
        kind: "review".to_owned(),
        id,
        claim_id: claim,
        evidence_ids,
        decision: decision.into(),
        rationale,
        reviewer,
    };
    print_effect("review", &record.id, workspace.add_review(&record)?)
}

fn validate(json: bool) -> Result<()> {
    let validation = discover_current()?.validate();
    if json {
        print_json(&validation)?;
    } else if validation.valid {
        print_text(&format!(
            "Workspace is valid.\nCounts: {}",
            validation
                .counts
                .iter()
                .map(|(kind, count)| format!("{kind}={count}"))
                .collect::<Vec<_>>()
                .join(", ")
        ))?;
    } else {
        let errors = validation
            .errors
            .iter()
            .map(|error| format!("- {}", terminal_text(error)))
            .collect::<Vec<_>>()
            .join("\n");
        print_text(&format!("Workspace is invalid:\n{errors}"))?;
    }
    if validation.valid {
        Ok(())
    } else {
        Err(Error::invalid("workspace", "validation failed"))
    }
}

fn status(json: bool) -> Result<()> {
    let status = discover_current()?.status()?;
    if json {
        print_json(&status)
    } else {
        print_human_status(&status)
    }
}

pub(super) fn discover_current() -> Result<Workspace> {
    #[cfg(any(test, coverage))]
    let current = if take_current_directory_failure() {
        Err(std::io::Error::other("injected current directory failure"))
    } else {
        std::env::current_dir()
    };
    #[cfg(not(any(test, coverage)))]
    let current = std::env::current_dir();
    match current {
        Ok(current) => Workspace::discover(&current),
        Err(error) => Err(Error::io("read current directory", ".", error)),
    }
}

#[cfg(test)]
thread_local! {
    static CURRENT_DIRECTORY_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(super) fn inject_current_directory_failure() {
    CURRENT_DIRECTORY_FAILURE.set(true);
}

#[cfg(test)]
fn take_current_directory_failure() -> bool {
    CURRENT_DIRECTORY_FAILURE.replace(false)
}

#[cfg(all(coverage, not(test)))]
fn take_current_directory_failure() -> bool {
    std::env::var("RESEARCH_RUN_COVERAGE_FAULT").as_deref() == Ok("current directory")
}
