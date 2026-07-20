use crate::domain::{
    ClaimRecord, EvidenceLink, ExperimentReceipt, FORMAT_VERSION, ReviewRequest, SourceRecord,
};
use crate::{Error, Result};

use super::arguments::{
    ClaimCommand, Cli, Command, EvidenceCommand, ExperimentCommand, ReviewCommand, SourceCommand,
};
use super::current_directory::discover_current;
use super::handoff_commands;
use super::input::{read_json_input, read_text_input};
use super::inventory_commands;
use super::lifecycle_commands;
use super::migration_commands;
use super::render::{
    parse_artifact, print_effect, print_human_status, print_json, print_text, terminal_text,
};
use super::retrieval_commands;
use super::structured_commands;

pub(super) fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init {
            path,
            name,
            review_authority_id,
            review_authority_public_key,
            without_review_authority,
        } => lifecycle_commands::initialize(
            &path,
            &name,
            review_authority_id,
            review_authority_public_key,
            without_review_authority,
        ),
        Command::Retrofit { command } => inventory_commands::execute(command, false),
        Command::Reconcile { command } => inventory_commands::execute(command, true),
        Command::Migrate { command } => migration_commands::execute(command),
        Command::Recover(output) => lifecycle_commands::recover(output),
        Command::Source { command } => add_source(command),
        Command::Claim { command } => add_claim(command),
        Command::Evidence { command } => add_evidence(command),
        Command::Experiment { command } => add_experiment(command),
        Command::Review { command } => add_review(command),
        Command::Knowledge { command } => structured_commands::add_knowledge(command),
        Command::Relationship { command } => structured_commands::add_relationship(command),
        Command::List(args) => retrieval_commands::list(args),
        Command::Show(args) => retrieval_commands::show(args),
        Command::Search(args) => retrieval_commands::search(args),
        Command::Recent(args) => retrieval_commands::recent(args),
        Command::Timeline(args) => retrieval_commands::timeline(args),
        Command::Related(args) => retrieval_commands::related(args),
        Command::Unresolved(args) => retrieval_commands::unresolved(args),
        Command::Blockers(args) => retrieval_commands::blockers(args),
        Command::Next(args) => retrieval_commands::next(args),
        Command::Context(args) => retrieval_commands::context(args),
        Command::Handoff { command } => handoff_commands::execute(command),
        Command::Validate(output) => validate(output.json),
        Command::Status(output) => status(output.json),
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
    match command {
        ReviewCommand::Prepare {
            id,
            claim,
            decision,
            rationale,
            reviewer,
        } => {
            let request = discover_current()?.prepare_review_request(
                id,
                claim,
                decision.into(),
                rationale,
                reviewer,
            )?;
            print_json(&request)
        }
        ReviewCommand::Add { request, signature } => {
            let request: ReviewRequest = read_json_input(&request)?;
            let signature = read_text_input(&signature)?;
            let workspace = discover_current()?;
            let record = workspace.authorize_review_request(request, signature)?;
            print_effect("review", &record.id, workspace.add_review(&record)?)
        }
    }
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
