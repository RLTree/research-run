use super::{
    AuthorshipArg, ClaimCommand, Cli, Command, EvidenceArgs, EvidenceCommand, ExperimentCommand,
    OutcomeArg, OutputArgs, ProvenanceArg, ReviewArg, ReviewCommand, SourceCommand, StanceArg,
    discover_current, execute, inject_current_directory_failure, parse_artifact,
    print_human_status, terminal_text,
};
use crate::domain::{
    ArtifactLocatorType, Assessment, Authorship, Outcome, SourceProvenance, Stance,
};
use crate::workspace::{ProjectStatus, Status};

#[test]
fn external_artifact_locator_may_contain_colons() {
    let pointer = parse_artifact("external:https://example.invalid/a:Public summary")
        .expect("artifact pointer");
    assert_eq!(pointer.locator_type, ArtifactLocatorType::External);
    assert_eq!(pointer.locator, "https://example.invalid/a");
}

fn assert_discovery_failure(command: Command) {
    inject_current_directory_failure();
    assert!(execute(Cli { command }).is_err());
}

#[test]
fn every_current_workspace_command_propagates_discovery_failure() {
    assert_discovery_failure(Command::Source {
        command: SourceCommand::Add {
            id: "source-one".to_owned(),
            citation: "Citation".to_owned(),
            locator: "local:source".to_owned(),
            provenance: ProvenanceArg::Human,
            notes: String::new(),
        },
    });
    assert_discovery_failure(Command::Claim {
        command: ClaimCommand::Add {
            id: "claim-one".to_owned(),
            text: "Claim".to_owned(),
            scope: "Scope".to_owned(),
            owner: "Owner".to_owned(),
            authorship: AuthorshipArg::Human,
        },
    });
    assert_discovery_failure(Command::Evidence {
        command: EvidenceCommand::Add(EvidenceArgs {
            id: "evidence-one".to_owned(),
            claim: "claim-one".to_owned(),
            source: Some("source-one".to_owned()),
            experiment: None,
            artifact: None,
            stance: StanceArg::Supports,
            specific_evidence: "Specific".to_owned(),
            authorship: AuthorshipArg::Human,
        }),
    });
    assert_discovery_failure(Command::Experiment {
        command: ExperimentCommand::Add {
            id: "experiment-one".to_owned(),
            question: "Question?".to_owned(),
            method_ref: "method".to_owned(),
            observation: vec!["Observed".to_owned()],
            interpretation: "Interpretation".to_owned(),
            limitation: vec!["Limited".to_owned()],
            outcome: OutcomeArg::Positive,
            next_move: "Repeat".to_owned(),
            artifact: Vec::new(),
        },
    });
    assert_discovery_failure(Command::Review {
        command: ReviewCommand::Add {
            id: "review-one".to_owned(),
            claim: "claim-one".to_owned(),
            decision: ReviewArg::Supported,
            rationale: "Rationale".to_owned(),
            reviewer: "Reviewer".to_owned(),
        },
    });
    assert_discovery_failure(Command::Validate(OutputArgs { json: true }));
    assert_discovery_failure(Command::Status(OutputArgs { json: true }));
}

#[test]
fn artifact_parser_and_value_arguments_cover_every_semantic_variant() {
    let workspace =
        parse_artifact("workspace:artifacts/result.txt:Result").expect("workspace pointer");
    assert_eq!(workspace.locator_type, ArtifactLocatorType::Workspace);
    for invalid in [
        "missing-separators",
        "workspace:missing-description",
        "unknown:value:description",
        "workspace::description",
        "workspace:path:",
    ] {
        assert!(parse_artifact(invalid).is_err(), "accepted {invalid}");
    }

    assert_eq!(
        SourceProvenance::from(ProvenanceArg::Human),
        SourceProvenance::Human
    );
    assert_eq!(
        SourceProvenance::from(ProvenanceArg::Imported),
        SourceProvenance::Imported
    );
    assert_eq!(
        SourceProvenance::from(ProvenanceArg::Ai),
        SourceProvenance::Ai
    );
    assert_eq!(Authorship::from(AuthorshipArg::Human), Authorship::Human);
    assert_eq!(Authorship::from(AuthorshipArg::Ai), Authorship::Ai);
    for (argument, expected) in [
        (StanceArg::Supports, Stance::Supports),
        (StanceArg::Limits, Stance::Limits),
        (StanceArg::Contradicts, Stance::Contradicts),
        (StanceArg::Context, Stance::Context),
    ] {
        assert_eq!(Stance::from(argument), expected);
    }
    for (argument, expected) in [
        (OutcomeArg::Positive, Outcome::Positive),
        (OutcomeArg::Negative, Outcome::Negative),
        (OutcomeArg::Ambiguous, Outcome::Ambiguous),
        (OutcomeArg::Inconclusive, Outcome::Inconclusive),
    ] {
        assert_eq!(Outcome::from(argument), expected);
    }
    for (argument, expected) in [
        (ReviewArg::Unsupported, Assessment::Unsupported),
        (ReviewArg::Limited, Assessment::Limited),
        (ReviewArg::Supported, Assessment::Supported),
        (ReviewArg::Contradicted, Assessment::Contradicted),
    ] {
        assert_eq!(Assessment::from(argument), expected);
    }
    assert_eq!(terminal_text("line\n\u{1b}"), "line\\n\\u{1b}");
}

#[test]
fn empty_human_projection_and_current_directory_failure_are_explicit() {
    print_human_status(&Status {
        format_version: 1,
        project: ProjectStatus {
            id: "empty-project".to_owned(),
            name: "Empty project".to_owned(),
        },
        claim_ceiling: "Test ceiling",
        counts: std::collections::BTreeMap::new(),
        claims: Vec::new(),
        experiments: Vec::new(),
        unreviewed_ai_drafts: Vec::new(),
        blockers: Vec::new(),
        next_actions: Vec::new(),
    });
    inject_current_directory_failure();
    assert!(discover_current().is_err());
}
