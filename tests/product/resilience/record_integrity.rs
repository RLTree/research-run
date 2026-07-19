use super::*;

#[path = "record_integrity/mutation_authority.rs"]
mod mutation_authority;
use research_run::domain::{
    ArtifactLocatorType, ArtifactPointer, Assessment, Authorship, ClaimRecord, EvidenceLink,
    ExperimentReceipt, FORMAT_VERSION, Outcome, ReviewDecision, SourceProvenance, SourceRecord,
    Stance,
};
use research_run::workspace::Workspace;

fn source(id: &str) -> SourceRecord {
    SourceRecord {
        schema_version: FORMAT_VERSION,
        kind: "source".to_owned(),
        id: id.to_owned(),
        citation: "Citation".to_owned(),
        locator: "local:source".to_owned(),
        provenance: SourceProvenance::Human,
        notes: String::new(),
    }
}

fn claim(id: &str) -> ClaimRecord {
    ClaimRecord {
        schema_version: FORMAT_VERSION,
        kind: "claim".to_owned(),
        id: id.to_owned(),
        text: "Claim".to_owned(),
        scope: "Scope".to_owned(),
        owner: "Owner".to_owned(),
        authorship: Authorship::Human,
    }
}

fn experiment(id: &str) -> ExperimentReceipt {
    ExperimentReceipt {
        schema_version: FORMAT_VERSION,
        kind: "experiment".to_owned(),
        id: id.to_owned(),
        question: "Question?".to_owned(),
        method_ref: "method.md".to_owned(),
        observations: vec!["Observed".to_owned()],
        interpretation: "Interpretation".to_owned(),
        limitations: vec!["Limited".to_owned()],
        outcome: Outcome::Inconclusive,
        next_move: "Repeat".to_owned(),
        artifacts: Vec::new(),
    }
}

fn maximum_record_sized_experiment() -> ExperimentReceipt {
    let mut record = experiment("experiment-record-budget");
    record.artifacts = (0..15)
        .map(|index| ArtifactPointer {
            locator_type: ArtifactLocatorType::External,
            locator: format!("external:item-{index}"),
            description: "x".repeat(research_run::domain::MAX_TEXT_BYTES),
            digest: None,
        })
        .collect();
    record.artifacts.push(ArtifactPointer {
        locator_type: ArtifactLocatorType::External,
        locator: "external:adjustable".to_owned(),
        description: "x".to_owned(),
        digest: None,
    });
    let serialized_bytes = serde_json::to_vec_pretty(&record)
        .expect("experiment JSON")
        .len()
        + 1;
    let extra = 1_048_576usize
        .checked_sub(serialized_bytes)
        .expect("fixture starts below the record budget");
    assert!(extra < research_run::domain::MAX_TEXT_BYTES);
    record
        .artifacts
        .last_mut()
        .expect("adjustable artifact")
        .description = "x".repeat(extra + 1);
    record
}

fn evidence(id: &str) -> EvidenceLink {
    EvidenceLink {
        schema_version: FORMAT_VERSION,
        kind: "evidence".to_owned(),
        id: id.to_owned(),
        claim_id: "claim-one".to_owned(),
        source_id: Some("source-one".to_owned()),
        experiment_id: None,
        artifact: None,
        stance: Stance::Supports,
        specific_evidence: "Specific".to_owned(),
        authorship: Authorship::Human,
    }
}

fn review(id: &str) -> ReviewDecision {
    ReviewDecision {
        schema_version: FORMAT_VERSION,
        kind: "review".to_owned(),
        id: id.to_owned(),
        claim_id: "claim-one".to_owned(),
        evidence_ids: Vec::new(),
        subject_sha256: None,
        decision: Assessment::Limited,
        rationale: "Rationale".to_owned(),
        reviewer: "Reviewer".to_owned(),
    }
}

#[test]
fn workspace_add_methods_propagate_each_record_validation_failure() {
    let root = initialize("direct-record-validation");
    let workspace = Workspace::discover(&root.0).expect("discover workspace");

    let mut invalid_source = source("source-one");
    invalid_source.citation.clear();
    assert!(workspace.add_source(&invalid_source).is_err());

    let mut invalid_claim = claim("claim-one");
    invalid_claim.owner.clear();
    assert!(workspace.add_claim(&invalid_claim).is_err());

    let mut invalid_experiment = experiment("experiment-one");
    invalid_experiment.observations.clear();
    assert!(workspace.add_experiment(&invalid_experiment).is_err());

    let mut invalid_evidence = evidence("evidence-one");
    invalid_evidence.source_id = None;
    assert!(workspace.add_evidence(&invalid_evidence).is_err());

    let mut invalid_review = review("review-one");
    invalid_review.decision = Assessment::Unreviewed;
    assert!(workspace.add_review(&invalid_review).is_err());
}

#[test]
fn workspace_add_methods_propagate_snapshot_and_artifact_failures() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let root = initialize("direct-experiment-artifact");
        let outside = TempDir::new("direct-experiment-artifact-outside");
        symlink(&outside.0, root.0.join("artifact-link")).expect("artifact symlink");
        let workspace = Workspace::discover(&root.0).expect("discover workspace");
        let mut record = experiment("experiment-one");
        record.artifacts.push(ArtifactPointer {
            locator_type: ArtifactLocatorType::Workspace,
            locator: "artifact-link/result.txt".to_owned(),
            description: "Result".to_owned(),
            digest: None,
        });
        assert!(workspace.add_experiment(&record).is_err());

        assert!(workspace.add_claim(&claim("claim-one")).is_ok());
        let mut link = evidence("evidence-one");
        link.source_id = None;
        link.artifact = Some("artifact-link/result.txt".to_owned());
        assert!(workspace.add_evidence(&link).is_err());
    }

    for operation in ["source", "claim", "experiment", "evidence", "review"] {
        let root = initialize(operation);
        let workspace = Workspace::discover(&root.0).expect("discover workspace");
        fs::write(root.0.join(".research-run/manifest.json"), b"not-json")
            .expect("corrupt manifest after discovery");
        let result = match operation {
            "source" => workspace.add_source(&source("source-one")),
            "claim" => workspace.add_claim(&claim("claim-one")),
            "experiment" => workspace.add_experiment(&experiment("experiment-one")),
            "evidence" => workspace.add_evidence(&evidence("evidence-one")),
            "review" => workspace.add_review(&review("review-one")),
            _ => unreachable!(),
        };
        assert!(result.is_err());
    }
}
