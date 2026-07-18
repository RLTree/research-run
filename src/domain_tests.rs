use super::{
    ArtifactLocatorType, ArtifactPointer, Assessment, Authorship, CanonicalRecord, ClaimRecord,
    EvidenceLink, ExperimentReceipt, Outcome, ProjectManifest, ReviewDecision, SourceProvenance,
    SourceRecord, Stance, required_text, validate_id, validate_workspace_locator,
};

#[test]
fn identifiers_and_paths_fail_closed() {
    assert!(validate_id("claim-one", "id").is_ok());
    assert!(validate_id(&format!("a{}", "b".repeat(63)), "id").is_ok());
    assert!(validate_id("", "id").is_err());
    assert!(validate_id(&"a".repeat(65), "id").is_err());
    assert!(validate_id("1claim", "id").is_err());
    assert!(validate_id("claim_underscore", "id").is_err());
    assert!(validate_id("../claim", "id").is_err());
    assert!(validate_workspace_locator("").is_err());
    assert!(validate_workspace_locator("artifacts/summary.txt").is_ok());
    assert!(validate_workspace_locator("../secret").is_err());
    assert!(validate_workspace_locator("/tmp/secret").is_err());
}

#[test]
fn manifest_identity_is_deterministic() {
    let manifest = ProjectManifest::new("Synthetic Assay").expect("manifest");
    assert_eq!(manifest.project_id, "synthetic-assay");
}

#[test]
fn domain_validators_reject_each_illegal_state_class() {
    manifest_validation_rejects_invalid_shapes();
    source_and_claim_validation_reject_missing_text();
    evidence_and_artifact_validation_reject_invalid_references();
    experiment_and_review_validation_reject_invalid_states();
}

fn manifest_validation_rejects_invalid_shapes() {
    assert!(ProjectManifest::new("!!!").is_ok());
    assert!(
        ProjectManifest::new("123 project")
            .expect("numeric name")
            .project_id
            .starts_with("project-")
    );
    assert!(required_text("   ", "text").is_err());
    assert!(required_text(&"x".repeat(65_537), "text").is_err());

    let mut manifest = ProjectManifest::new("Valid").expect("manifest");
    manifest.schema_version = 2;
    assert!(manifest.validate().is_err());
    manifest.schema_version = 1;
    manifest.kind = "wrong".to_owned();
    assert!(manifest.validate().is_err());
    manifest.kind = "project-manifest".to_owned();
    manifest.declared_roots = vec!["other".to_owned()];
    assert!(manifest.validate().is_err());

    let long_name = format!("a{} z", "b".repeat(80));
    let truncated = ProjectManifest::new(&long_name).expect("long manifest");
    assert!(truncated.project_id.len() <= 64);
    assert!(!truncated.project_id.ends_with('-'));
}

fn source_and_claim_validation_reject_missing_text() {
    let source = SourceRecord {
        schema_version: 1,
        kind: "source".to_owned(),
        id: "source-one".to_owned(),
        citation: String::new(),
        locator: "local:x".to_owned(),
        provenance: SourceProvenance::Human,
        notes: String::new(),
    };
    assert!(source.validate().is_err());
    let claim = ClaimRecord {
        schema_version: 1,
        kind: "claim".to_owned(),
        id: "claim-one".to_owned(),
        text: "Claim".to_owned(),
        scope: String::new(),
        owner: "Owner".to_owned(),
        authorship: Authorship::Human,
    };
    assert!(claim.validate().is_err());
}

fn evidence_and_artifact_validation_reject_invalid_references() {
    let mut evidence = EvidenceLink {
        schema_version: 1,
        kind: "evidence".to_owned(),
        id: "evidence-one".to_owned(),
        claim_id: "claim-one".to_owned(),
        source_id: None,
        experiment_id: None,
        artifact: None,
        stance: Stance::Context,
        specific_evidence: "Specific".to_owned(),
        authorship: Authorship::Human,
    };
    assert!(evidence.validate().is_err());
    evidence.source_id = Some("source-one".to_owned());
    evidence.experiment_id = Some("experiment-one".to_owned());
    assert!(evidence.validate().is_err());
    evidence.experiment_id = None;
    evidence.specific_evidence.clear();
    assert!(evidence.validate().is_err());
    evidence.specific_evidence = "Specific".to_owned();
    evidence.source_id = None;
    evidence.artifact = Some("../escape".to_owned());
    assert!(evidence.validate().is_err());

    let workspace_pointer = ArtifactPointer {
        locator_type: ArtifactLocatorType::Workspace,
        locator: "../escape".to_owned(),
        description: "Description".to_owned(),
        digest: None,
    };
    assert!(workspace_pointer.validate().is_err());
    let empty_pointer = ArtifactPointer {
        locator_type: ArtifactLocatorType::External,
        locator: String::new(),
        description: "Description".to_owned(),
        digest: None,
    };
    assert!(empty_pointer.validate().is_err());
    let external_pointer = ArtifactPointer {
        locator_type: ArtifactLocatorType::External,
        locator: "https://example.invalid".to_owned(),
        description: "Description".to_owned(),
        digest: Some(String::new()),
    };
    assert!(external_pointer.validate().is_err());
}

fn experiment_and_review_validation_reject_invalid_states() {
    let mut experiment = ExperimentReceipt {
        schema_version: 1,
        kind: "experiment".to_owned(),
        id: "experiment-one".to_owned(),
        question: "Question?".to_owned(),
        method_ref: "protocol.md".to_owned(),
        observations: Vec::new(),
        interpretation: "Interpretation".to_owned(),
        limitations: vec!["Limit".to_owned()],
        outcome: Outcome::Inconclusive,
        next_move: "Repeat".to_owned(),
        artifacts: Vec::new(),
    };
    assert!(experiment.validate().is_err());
    experiment.observations = vec!["Observation".to_owned()];
    experiment.limitations = vec!["Limit".to_owned(); 257];
    assert!(experiment.validate().is_err());
    experiment.limitations = vec!["Limit".to_owned()];
    experiment.artifacts = (0..129)
        .map(|_| ArtifactPointer {
            locator_type: ArtifactLocatorType::External,
            locator: "https://example.invalid".to_owned(),
            description: "Description".to_owned(),
            digest: None,
        })
        .collect();
    assert!(experiment.validate().is_err());

    let review = ReviewDecision {
        schema_version: 1,
        kind: "review".to_owned(),
        id: "review-one".to_owned(),
        claim_id: "claim-one".to_owned(),
        decision: Assessment::Unreviewed,
        rationale: "Rationale".to_owned(),
        reviewer: "Researcher".to_owned(),
    };
    assert!(review.validate().is_err());
}
