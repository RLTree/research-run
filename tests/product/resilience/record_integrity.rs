use super::*;
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

#[test]
fn mutations_reject_damaged_authority_and_unbound_reviews() {
    reject_mutation_over_damaged_references();
    reject_unbound_review_authority();
}

fn reject_mutation_over_damaged_references() {
    let root = initialize("damaged-mutation-authority");
    fs::write(
        root.0.join(".research-run/evidence/damaged.json"),
        serde_json::to_vec_pretty(&evidence("damaged")).expect("evidence JSON"),
    )
    .expect("write damaged reference");
    let workspace = Workspace::discover(&root.0).expect("discover workspace");
    assert!(workspace.add_source(&source("source-two")).is_err());
    assert!(workspace.add_claim(&claim("claim-two")).is_err());
    assert!(
        workspace
            .add_experiment(&experiment("experiment-two"))
            .is_err()
    );
    assert!(workspace.add_evidence(&evidence("evidence-two")).is_err());
    assert!(workspace.add_review(&review("review-two")).is_err());
}

fn reject_unbound_review_authority() {
    let root = initialize("unbound-review-authority");
    let workspace = Workspace::discover(&root.0).expect("discover workspace");
    assert!(workspace.add_claim(&claim("claim-one")).is_ok());

    let mut supported_without_evidence = review("supported-without-evidence");
    supported_without_evidence.decision = Assessment::Supported;
    assert!(workspace.add_review(&supported_without_evidence).is_err());

    let mut mismatched = review("mismatched-review");
    mismatched.decision = Assessment::Limited;
    mismatched.evidence_ids = vec!["evidence-missing".to_owned()];
    assert!(workspace.add_review(&mismatched).is_err());

    let mut missing_claim = review("missing-claim-review");
    missing_claim.claim_id = "claim-missing".to_owned();
    missing_claim.decision = Assessment::Limited;
    assert!(workspace.add_review(&missing_claim).is_err());
    assert!(workspace.claim_evidence_ids("claim-missing").is_err());

    fs::write(root.0.join(".research-run/manifest.json"), b"not-json")
        .expect("corrupt manifest after discovery");
    assert!(workspace.claim_evidence_ids("claim-one").is_err());
}

#[test]
fn maximum_record_budget_remains_idempotent() {
    let root = initialize("maximum-record-budget");
    let workspace = Workspace::discover(&root.0).expect("discover workspace");
    let record = maximum_record_sized_experiment();
    assert_eq!(
        serde_json::to_vec_pretty(&record)
            .expect("experiment JSON")
            .len()
            + 1,
        1_048_576
    );
    assert_eq!(workspace.add_experiment(&record).expect("first add"), true);
    assert_eq!(workspace.add_experiment(&record).expect("retry"), false);
}

#[test]
fn manifest_and_record_directory_errors_propagate_at_their_owner() {
    let invalid_discovery = initialize("manifest-validation-discovery");
    fs::write(
        invalid_discovery.0.join(".research-run/manifest.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "kind": "project-manifest",
            "project_id": "bad id",
            "name": "Name",
            "declared_roots": ["."]
        }))
        .expect("manifest JSON"),
    )
    .expect("write invalid manifest");
    assert!(Workspace::discover(&invalid_discovery.0).is_err());
    let direct = Workspace::for_recovery(&invalid_discovery.0).expect("recovery workspace");
    assert!(direct.status().is_err());

    let invalid_record = initialize("record-validation-load");
    let mut stored = source("source-one");
    stored.citation.clear();
    fs::write(
        invalid_record
            .0
            .join(".research-run/sources/source-one.json"),
        serde_json::to_vec_pretty(&stored).expect("source JSON"),
    )
    .expect("write invalid source");
    let workspace = Workspace::for_recovery(&invalid_record.0).expect("recovery workspace");
    assert!(workspace.status().is_err());

    let directory = initialize("record-directory-faults");
    assert!(
        add_source(&directory.0, "source-one", None)
            .status
            .success()
    );
    for fault in ["read record directory", "read record entry"] {
        assert!(
            !run(&directory.0, &["status", "--json"], Some(fault))
                .status
                .success()
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let symlinked = initialize("record-directory-symlink");
        let outside = TempDir::new("record-directory-symlink-outside");
        fs::remove_dir(symlinked.0.join(".research-run/sources")).expect("remove sources");
        symlink(&outside.0, symlinked.0.join(".research-run/sources")).expect("symlink sources");
        let workspace = Workspace::for_recovery(&symlinked.0).expect("recovery workspace");
        assert!(workspace.status().is_err());
    }
}
