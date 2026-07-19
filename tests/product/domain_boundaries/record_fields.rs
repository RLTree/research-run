use super::*;

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
