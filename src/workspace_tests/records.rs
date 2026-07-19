use super::super::references::current_review_bindings;
use super::*;

#[test]
fn direct_workspace_journey_exercises_every_record_authority() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Direct journey").expect("initialize");
    let source = source("source-one");
    assert!(workspace.add_source(&source).expect("source"));
    assert!(!workspace.add_source(&source).expect("source retry"));
    let claim = claim("claim-one");
    assert!(workspace.add_claim(&claim).expect("claim"));
    let experiment = ExperimentReceipt {
        schema_version: 1,
        kind: "experiment".to_owned(),
        id: "experiment-one".to_owned(),
        question: "Question?".to_owned(),
        method_ref: "protocol.md".to_owned(),
        observations: vec!["Observation".to_owned()],
        interpretation: "Interpretation".to_owned(),
        limitations: vec!["Limitation".to_owned()],
        outcome: Outcome::Negative,
        next_move: "Repeat".to_owned(),
        artifacts: Vec::new(),
    };
    assert!(workspace.add_experiment(&experiment).expect("experiment"));
    let evidence = EvidenceLink {
        schema_version: 1,
        kind: "evidence".to_owned(),
        id: "evidence-one".to_owned(),
        claim_id: claim.id.clone(),
        source_id: Some(source.id.clone()),
        experiment_id: None,
        artifact: None,
        stance: Stance::Limits,
        specific_evidence: "Specific evidence".to_owned(),
        authorship: Authorship::Human,
    };
    assert!(workspace.add_evidence(&evidence).expect("evidence"));
    let review = ReviewDecision {
        schema_version: 1,
        kind: "review".to_owned(),
        id: "review-one".to_owned(),
        claim_id: claim.id.clone(),
        evidence_ids: vec![evidence.id.clone()],
        decision: Assessment::Limited,
        rationale: "Bounded rationale".to_owned(),
        reviewer: "Researcher".to_owned(),
    };
    assert!(workspace.add_review(&review).expect("review"));
    assert!(!workspace.add_review(&review).expect("review retry"));
    assert!(workspace.validate().valid);
    let status = workspace.status().expect("status");
    assert_eq!(status.claims[0].assessment, Assessment::Limited);
    assert!(
        workspace
            .recover()
            .expect("empty recovery")
            .recovered
            .is_empty()
    );
    assert_eq!(
        Workspace::discover(&root).expect("discover").root(),
        root.canonicalize().expect("canonical root")
    );
    assert!(Workspace::for_recovery(&root).is_ok());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn discovery_stops_at_the_nearest_state_boundary() {
    let root = temporary();
    Workspace::initialize(&root, "Outer workspace").expect("outer workspace");
    let nested = root.join("nested/project");
    fs::create_dir_all(nested.join(".research-run")).expect("inner state boundary");
    assert!(Workspace::discover(&nested).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn semantic_reference_failures_leave_no_effect() {
    let absent = temporary().join("absent");
    assert!(Workspace::discover(&absent).is_err());
    assert!(Workspace::for_recovery(&absent).is_err());

    let root = temporary();
    let workspace = Workspace::initialize(&root, "Reference failures").expect("initialize");
    assert!(
        workspace
            .add_evidence(&evidence("evidence-one", "missing", None))
            .is_err()
    );
    assert!(
        workspace
            .add_review(&review("review-one", "missing"))
            .is_err()
    );

    let claim = claim("claim-one");
    workspace.add_claim(&claim).expect("claim");
    assert!(
        workspace
            .add_evidence(&evidence("evidence-two", &claim.id, Some("missing")))
            .is_err()
    );

    let mut experiment_link = evidence("evidence-three", &claim.id, None);
    experiment_link.artifact = None;
    experiment_link.experiment_id = Some("missing".to_owned());
    assert!(workspace.add_evidence(&experiment_link).is_err());

    workspace
        .add_review(&review("review-one", &claim.id))
        .expect("review");
    assert!(
        workspace
            .add_review(&review("review-two", &claim.id))
            .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn corrupted_references_block_status_validation_and_recovery() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Corruption boundary").expect("initialize");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    let bad = evidence("evidence-one", "missing", None);
    let path = workspace.state.join("evidence/evidence-one.json");
    fs::write(&path, serde_json::to_vec_pretty(&bad).expect("json")).expect("write corrupt record");
    assert!(!workspace.validate().valid);
    assert!(workspace.status().is_err());
    assert!(workspace.recover().is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn review_graph_validation_uses_one_canonical_binding() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Review binding matrix").expect("initialize");
    let mut reordered = review("review-reordered", "claim-one");
    reordered.evidence_ids = vec!["evidence-two".to_owned(), "evidence-one".to_owned()];
    assert!(reordered.validate().is_err());

    let mut unknown = review("review-unknown", "claim-one");
    unknown.evidence_ids = vec!["evidence-missing".to_owned()];
    let mut cross_claim = review("review-cross", "claim-two");
    cross_claim.evidence_ids = vec!["evidence-one".to_owned()];
    let incomplete = review("review-incomplete", "claim-one");
    let snapshot = Snapshot {
        manifest: ProjectManifest::new("Review binding matrix").expect("manifest"),
        sources: Vec::new(),
        claims: vec![claim("claim-one"), claim("claim-two")],
        evidence: vec![evidence("evidence-one", "claim-one", None)],
        experiments: Vec::new(),
        reviews: vec![unknown, cross_claim, incomplete],
    };
    let errors = workspace.reference_errors(&snapshot);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("unknown evidence"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("belongs to claim"))
    );
    assert!(current_review_bindings(&snapshot).is_empty());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn reference_validation_reports_every_invalid_relationship() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Reference matrix").expect("initialize");
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&root, root.join("artifact-link")).expect("artifact symlink");
    }
    let known_claim = claim("claim-one");
    let snapshot = Snapshot {
        manifest: ProjectManifest::new("Reference matrix").expect("manifest"),
        sources: Vec::new(),
        claims: vec![known_claim.clone()],
        evidence: vec![
            EvidenceLink {
                source_id: Some("missing-source".to_owned()),
                ..evidence("evidence-source", &known_claim.id, Some("missing-source"))
            },
            EvidenceLink {
                id: "evidence-experiment".to_owned(),
                source_id: None,
                experiment_id: Some("missing-experiment".to_owned()),
                artifact: None,
                ..evidence("evidence-experiment", &known_claim.id, None)
            },
            EvidenceLink {
                id: "evidence-artifact".to_owned(),
                source_id: None,
                experiment_id: None,
                artifact: Some("artifact-link/result".to_owned()),
                ..evidence("evidence-artifact", &known_claim.id, None)
            },
        ],
        experiments: vec![ExperimentReceipt {
            artifacts: vec![ArtifactPointer {
                locator_type: ArtifactLocatorType::Workspace,
                locator: "artifact-link/result".to_owned(),
                description: "Escaping artifact".to_owned(),
                digest: None,
            }],
            ..experiment("experiment-one")
        }],
        reviews: vec![
            review("review-one", &known_claim.id),
            review("review-two", &known_claim.id),
            review("review-missing", "missing-claim"),
        ],
    };
    let errors = workspace.reference_errors(&snapshot);
    for expected in [
        "unknown source reference",
        "unknown experiment reference",
        "workspace path",
        "multiple v0.1 reviews",
        "unknown claim reference",
    ] {
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "missing {expected}: {errors:?}"
        );
    }
    fs::remove_dir_all(root).expect("remove fixture");
}
