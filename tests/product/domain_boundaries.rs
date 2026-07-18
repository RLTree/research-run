use research_run::domain::{
    ArtifactLocatorType, ArtifactPointer, Assessment, Authorship, CanonicalRecord, ClaimRecord,
    EvidenceLink, ExperimentReceipt, Outcome, ProjectManifest, ReviewDecision, SourceProvenance,
    SourceRecord, Stance, required_text,
};

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

fn valid_source() -> SourceRecord {
    SourceRecord {
        schema_version: 1,
        kind: "source".to_owned(),
        id: "source-one".to_owned(),
        citation: "Citation".to_owned(),
        locator: "local:source".to_owned(),
        provenance: SourceProvenance::Human,
        notes: "Notes".to_owned(),
    }
}

fn valid_claim() -> ClaimRecord {
    ClaimRecord {
        schema_version: 1,
        kind: "claim".to_owned(),
        id: "claim-one".to_owned(),
        text: "Claim".to_owned(),
        scope: "Scope".to_owned(),
        owner: "Owner".to_owned(),
        authorship: Authorship::Human,
    }
}

#[test]
fn each_record_field_fails_independently() {
    assert!(ProjectManifest::new(" ").is_err());
    let mut manifest = ProjectManifest::new("Valid").expect("manifest");
    manifest.project_id = "Invalid".to_owned();
    assert!(manifest.validate().is_err());
    manifest.project_id = "valid".to_owned();
    manifest.name.clear();
    assert!(manifest.validate().is_err());

    let baseline = valid_source();
    for source in [
        SourceRecord {
            schema_version: 2,
            ..baseline.clone()
        },
        SourceRecord {
            kind: "wrong".to_owned(),
            ..baseline.clone()
        },
        SourceRecord {
            id: "Invalid".to_owned(),
            ..baseline.clone()
        },
        SourceRecord {
            locator: String::new(),
            ..baseline.clone()
        },
        SourceRecord {
            notes: "x".repeat(65_537),
            ..baseline.clone()
        },
    ] {
        assert!(source.validate().is_err());
    }
    let claim = valid_claim();
    for candidate in [
        ClaimRecord {
            schema_version: 2,
            ..claim.clone()
        },
        ClaimRecord {
            kind: "wrong".to_owned(),
            ..claim.clone()
        },
        ClaimRecord {
            id: "Invalid".to_owned(),
            ..claim.clone()
        },
        ClaimRecord {
            text: String::new(),
            ..claim.clone()
        },
        ClaimRecord {
            owner: String::new(),
            ..claim.clone()
        },
    ] {
        assert!(candidate.validate().is_err());
    }
}

fn valid_evidence() -> EvidenceLink {
    EvidenceLink {
        schema_version: 1,
        kind: "evidence".to_owned(),
        id: "evidence-one".to_owned(),
        claim_id: "claim-one".to_owned(),
        source_id: Some("source-one".to_owned()),
        experiment_id: None,
        artifact: None,
        stance: Stance::Supports,
        specific_evidence: "Specific".to_owned(),
        authorship: Authorship::Human,
    }
}

#[test]
fn each_evidence_reference_fails_independently() {
    let baseline = valid_evidence();
    for candidate in [
        EvidenceLink {
            schema_version: 2,
            ..baseline.clone()
        },
        EvidenceLink {
            kind: "wrong".to_owned(),
            ..baseline.clone()
        },
        EvidenceLink {
            id: "Invalid".to_owned(),
            ..baseline.clone()
        },
        EvidenceLink {
            claim_id: "Invalid".to_owned(),
            ..baseline.clone()
        },
        EvidenceLink {
            source_id: Some("Invalid".to_owned()),
            ..baseline.clone()
        },
        EvidenceLink {
            source_id: None,
            experiment_id: Some("Invalid".to_owned()),
            ..baseline.clone()
        },
        EvidenceLink {
            source_id: None,
            artifact: Some("../escape".to_owned()),
            ..baseline.clone()
        },
        EvidenceLink {
            specific_evidence: String::new(),
            ..baseline.clone()
        },
    ] {
        assert!(candidate.validate().is_err());
    }
}

fn valid_experiment() -> ExperimentReceipt {
    ExperimentReceipt {
        schema_version: 1,
        kind: "experiment".to_owned(),
        id: "experiment-one".to_owned(),
        question: "Question?".to_owned(),
        method_ref: "protocol.md".to_owned(),
        observations: vec!["Observation".to_owned()],
        interpretation: "Interpretation".to_owned(),
        limitations: vec!["Limitation".to_owned()],
        outcome: Outcome::Positive,
        next_move: "Repeat".to_owned(),
        artifacts: Vec::new(),
    }
}

#[test]
fn each_experiment_and_review_field_fails_independently() {
    each_experiment_field_fails_independently();
    each_review_field_fails_independently();
}

fn each_experiment_field_fails_independently() {
    let baseline = valid_experiment();
    for candidate in [
        ExperimentReceipt {
            schema_version: 2,
            ..baseline.clone()
        },
        ExperimentReceipt {
            kind: "wrong".to_owned(),
            ..baseline.clone()
        },
        ExperimentReceipt {
            id: "Invalid".to_owned(),
            ..baseline.clone()
        },
        ExperimentReceipt {
            question: String::new(),
            ..baseline.clone()
        },
        ExperimentReceipt {
            method_ref: String::new(),
            ..baseline.clone()
        },
        ExperimentReceipt {
            observations: vec![String::new()],
            ..baseline.clone()
        },
        ExperimentReceipt {
            interpretation: String::new(),
            ..baseline.clone()
        },
        ExperimentReceipt {
            limitations: vec![String::new()],
            ..baseline.clone()
        },
        ExperimentReceipt {
            next_move: String::new(),
            ..baseline.clone()
        },
    ] {
        assert!(candidate.validate().is_err());
    }
    let pointer = ArtifactPointer {
        locator_type: ArtifactLocatorType::External,
        locator: "external:item".to_owned(),
        description: String::new(),
        digest: None,
    };
    assert!(pointer.validate().is_err());
}

fn each_review_field_fails_independently() {
    let review = ReviewDecision {
        schema_version: 1,
        kind: "review".to_owned(),
        id: "review-one".to_owned(),
        claim_id: "claim-one".to_owned(),
        decision: Assessment::Supported,
        rationale: "Rationale".to_owned(),
        reviewer: "Reviewer".to_owned(),
    };
    for candidate in [
        ReviewDecision {
            schema_version: 2,
            ..review.clone()
        },
        ReviewDecision {
            kind: "wrong".to_owned(),
            ..review.clone()
        },
        ReviewDecision {
            id: "Invalid".to_owned(),
            ..review.clone()
        },
        ReviewDecision {
            claim_id: "Invalid".to_owned(),
            ..review.clone()
        },
        ReviewDecision {
            rationale: String::new(),
            ..review.clone()
        },
        ReviewDecision {
            reviewer: String::new(),
            ..review.clone()
        },
    ] {
        assert!(candidate.validate().is_err());
    }
}
