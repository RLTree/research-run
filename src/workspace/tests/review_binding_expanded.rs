use super::*;
use crate::workspace::references::current_review_bindings;

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
        review_authorities: Vec::new(),
        inventories: Vec::new(),
        knowledge: Vec::new(),
        relationships: Vec::new(),
        migrations: Vec::new(),
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
    assert!(
        current_review_bindings(&workspace, &snapshot)
            .unwrap()
            .is_empty()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn review_writes_require_exact_current_subject_and_known_claim() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Review subject failures").expect("initialize");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    assert!(
        workspace
            .claim_evidence_ids("claim-one")
            .unwrap()
            .is_empty()
    );
    let mut wrong = review("review-wrong", "claim-one");
    wrong.subject_sha256 = Some("a".repeat(64));
    assert!(workspace.add_review(&wrong).is_err());
    assert!(workspace.claim_evidence_ids("missing-claim").is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn review_binding_covers_experiment_artifact_and_stale_authority() {
    let root = temporary();
    let workspace = review_workspace(&root, "Review subject graph");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    workspace
        .add_experiment(&experiment("experiment-one"))
        .expect("experiment");
    fs::write(root.join("artifact.txt"), b"observed bytes").expect("artifact");
    let mut linked = evidence("evidence-experiment", "claim-one", None);
    linked.artifact = None;
    linked.experiment_id = Some("experiment-one".to_owned());
    workspace
        .add_evidence(&linked)
        .expect("experiment evidence");
    let artifact = evidence("evidence-artifact", "claim-one", None);
    workspace
        .add_evidence(&artifact)
        .expect("artifact evidence");
    let mut decision = review("review-one", "claim-one");
    decision.evidence_ids = vec![
        "evidence-artifact".to_owned(),
        "evidence-experiment".to_owned(),
    ];
    decision.subject_sha256 = Some(workspace.review_subject_binding("claim-one").unwrap().1);
    authorize_review(&workspace, &mut decision);
    workspace.add_review(&decision).expect("review");
    fs::remove_file(root.join("artifact.txt")).expect("remove artifact");
    let errors = workspace.review_binding_errors(&workspace.load_snapshot().expect("snapshot"));
    assert!(errors.iter().any(|error| error.contains("artifact.txt")));
    let mut replacement = review("review-two", "claim-one");
    replacement.evidence_ids = decision.evidence_ids.clone();
    replacement.subject_sha256 = decision.subject_sha256.clone();
    authorize_review(&workspace, &mut replacement);
    assert!(workspace.add_review(&replacement).is_err());
    assert!(workspace.list(None, 10).is_err());
    assert!(workspace.show("claim", "claim-one").is_err());
    assert!(workspace.search("claim", 10).is_err());
    assert!(workspace.recent(10).is_err());
    assert!(workspace.timeline(10).is_err());
    assert!(workspace.unresolved(10).is_err());
    assert!(workspace.blocker_items(10).is_err());
    assert!(workspace.next_action_items(10).is_err());
    assert!(workspace.context(None, 10).is_err());
    assert!(workspace.status().is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = temporary();
        fs::write(outside.join("target.txt"), b"outside").expect("outside artifact");
        symlink(outside.join("target.txt"), root.join("artifact.txt")).expect("artifact symlink");
        assert!(workspace.review_subject_binding("claim-one").is_err());
        fs::remove_dir_all(outside).expect("remove outside");
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn legacy_and_missing_experiment_bindings_are_stale() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Legacy review binding").expect("initialize");
    let mut legacy = review("review-legacy", "claim-one");
    let mut missing = review("review-missing", "claim-two");
    missing.evidence_ids = vec!["evidence-missing".to_owned()];
    missing.subject_sha256 = Some("a".repeat(64));
    let mut missing_evidence = evidence("evidence-missing", "claim-two", None);
    missing_evidence.artifact = None;
    missing_evidence.experiment_id = Some("experiment-missing".to_owned());
    let mut missing_source_review = review("review-source", "claim-three");
    missing_source_review.evidence_ids = vec!["evidence-source".to_owned()];
    missing_source_review.subject_sha256 = Some("b".repeat(64));
    for review in [&mut legacy, &mut missing, &mut missing_source_review] {
        review.authorization = Some(crate::domain::ReviewAuthorization {
            authority_id: "test-human".to_owned(),
            signer_fingerprint: "SHA256:test".to_owned(),
            signature: "test-signature".to_owned(),
        });
    }
    let snapshot = Snapshot {
        manifest: workspace.read_manifest().expect("manifest"),
        sources: Vec::new(),
        claims: vec![claim("claim-one"), claim("claim-two"), claim("claim-three")],
        evidence: vec![
            missing_evidence,
            evidence("evidence-source", "claim-three", Some("source-missing")),
        ],
        experiments: Vec::new(),
        reviews: vec![legacy.clone(), missing, missing_source_review],
        review_authorities: Vec::new(),
        inventories: Vec::new(),
        knowledge: Vec::new(),
        relationships: Vec::new(),
        migrations: Vec::new(),
    };
    legacy.evidence_ids.clear();
    let errors = workspace.review_binding_errors(&snapshot);
    assert!(errors.iter().any(|error| error.contains("legacy review")));
    assert!(
        errors
            .iter()
            .any(|error| error.contains("experiment reference"))
    );
    assert!(
        errors
            .iter()
            .any(|error| error.contains("source reference"))
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn review_binding_propagates_snapshot_load_failure() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Binding load failure").expect("initialize");
    fs::remove_file(workspace.state.join("manifest.json")).expect("remove manifest");
    assert!(workspace.review_subject_binding("claim-one").is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}
