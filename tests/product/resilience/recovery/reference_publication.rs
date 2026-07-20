use super::*;

#[test]
fn installed_process_recovers_each_typed_reference_and_rejects_existing_review() {
    let root = initialize("typed-recovery-references");
    add_claim(&root.0);
    add_recovery_experiment(&root.0);
    recover_experiment_and_artifact_evidence(&root.0);
    reject_second_review_during_recovery(&root.0);
}

fn add_recovery_experiment(root: &Path) {
    assert!(
        run(
            root,
            &[
                "experiment",
                "add",
                "--id",
                "experiment-one",
                "--question",
                "Question?",
                "--method-ref",
                "protocol.md",
                "--observation",
                "Observed",
                "--interpretation",
                "Interpretation",
                "--limitation",
                "Limited",
                "--outcome",
                "positive",
                "--next-move",
                "Repeat",
            ],
            None,
        )
        .status
        .success()
    );
}

fn recover_experiment_and_artifact_evidence(root: &Path) {
    pending_evidence(root, "evidence-experiment", Some("experiment-one"), None);
    assert!(run(root, &["recover", "--json"], None).status.success());
    pending_evidence(
        root,
        "evidence-artifact-recovered",
        None,
        Some("artifacts/result.txt"),
    );
    assert!(run(root, &["recover", "--json"], None).status.success());
    fs::create_dir(root.join("artifacts")).expect("artifact directory");
    fs::write(root.join("artifacts/result.txt"), b"observation").expect("artifact");
}

fn reject_second_review_during_recovery(root: &Path) {
    crate::review_test_signing::add_signed_review(
        root,
        "review-one",
        "claim-one",
        "supported",
        "Rationale",
        "Reviewer",
    );
    let reviews = root.join(".research-run/reviews");
    let pending = reviews.join(".review-two.json.9.1.tmp");
    let second = crate::review_test_signing::signed_review_record(
        root,
        "review-two",
        "claim-one",
        "limited",
        "Second",
        "Reviewer",
    );
    fs::write(
        &pending,
        serde_json::to_vec_pretty(&second).expect("review JSON"),
    )
    .expect("pending review");
    let output = run(root, &["recover", "--json"], None);
    assert!(!output.status.success());
    assert!(pending.exists());
}

fn pending_json(root: &Path, directory: &str, id: &str, value: serde_json::Value) -> PathBuf {
    let path = root
        .join(".research-run")
        .join(directory)
        .join(format!(".{id}.json.9.7.tmp"));
    fs::write(
        &path,
        serde_json::to_vec_pretty(&value).expect("pending JSON"),
    )
    .expect("write pending JSON");
    path
}

#[test]
fn installed_recovery_covers_each_typed_record_boundary() {
    recover_pending_claim();
    recover_pending_experiment();
    recover_source_evidence();
    reject_missing_evidence_references();
    reject_invalid_pending_records();
}

fn recover_pending_claim() {
    let claim_root = initialize("pending-claim");
    pending_json(
        &claim_root.0,
        "claims",
        "claim-one",
        json!({
            "schema_version": 1, "kind": "claim", "id": "claim-one",
            "text": "Claim", "scope": "Scope", "owner": "Owner", "authorship": "human"
        }),
    );
    assert!(
        run(&claim_root.0, &["recover", "--json"], None)
            .status
            .success()
    );
}

fn recover_pending_experiment() {
    let experiment_root = initialize("pending-experiment");
    pending_json(
        &experiment_root.0,
        "experiments",
        "experiment-one",
        json!({
            "schema_version": 1, "kind": "experiment", "id": "experiment-one",
            "question": "Question?", "method_ref": "method.md", "observations": ["Observed"],
            "interpretation": "Interpretation", "limitations": ["Limited"],
            "outcome": "inconclusive", "next_move": "Repeat",
            "artifacts": [{"locator_type":"external", "locator":"local:result", "description":"Result", "digest":null}]
        }),
    );
    assert!(
        run(&experiment_root.0, &["recover", "--json"], None)
            .status
            .success()
    );
}

fn recover_source_evidence() {
    let source_evidence = initialize("pending-source-evidence");
    add_claim(&source_evidence.0);
    assert!(
        add_source(&source_evidence.0, "source-one", None)
            .status
            .success()
    );
    pending_json(
        &source_evidence.0,
        "evidence",
        "evidence-one",
        json!({
            "schema_version": 1, "kind": "evidence", "id": "evidence-one",
            "claim_id": "claim-one", "source_id": "source-one", "experiment_id": null,
            "artifact": null, "stance": "supports", "specific_evidence": "Specific",
            "authorship": "human"
        }),
    );
    assert!(
        run(&source_evidence.0, &["recover", "--json"], None)
            .status
            .success()
    );
}

fn reject_missing_evidence_references() {
    for (label, source, experiment) in [
        ("missing-source", Some("source-missing"), None),
        ("missing-experiment", None, Some("experiment-missing")),
    ] {
        let root = initialize(label);
        add_claim(&root.0);
        let pending = pending_json(
            &root.0,
            "evidence",
            "evidence-one",
            json!({
                "schema_version": 1, "kind": "evidence", "id": "evidence-one",
                "claim_id": "claim-one", "source_id": source, "experiment_id": experiment,
                "artifact": null, "stance": "context", "specific_evidence": "Specific",
                "authorship": "human"
            }),
        );
        assert!(!run(&root.0, &["recover", "--json"], None).status.success());
        assert!(pending.exists());
    }
}

fn reject_invalid_pending_records() {
    for directory in ["sources", "claims", "experiments", "evidence", "reviews"] {
        let root = initialize(&format!("invalid-pending-{directory}"));
        let pending = pending_json(
            &root.0,
            directory,
            "bad-id",
            json!({"schema_version": 1, "kind": directory.trim_end_matches('s'), "id": "bad id"}),
        );
        assert!(!run(&root.0, &["recover", "--json"], None).status.success());
        assert!(pending.exists());
    }
}

#[test]
fn recovery_rejects_preexisting_reference_damage_before_publication() {
    let root = initialize("preexisting-reference-damage");
    fs::write(
        root.0.join(".research-run/evidence/evidence-one.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1, "kind": "evidence", "id": "evidence-one",
            "claim_id": "claim-missing", "source_id": "source-missing", "experiment_id": null,
            "artifact": null, "stance": "context", "specific_evidence": "Specific",
            "authorship": "human"
        }))
        .expect("evidence JSON"),
    )
    .expect("write damaged evidence");
    assert!(!run(&root.0, &["recover", "--json"], None).status.success());
}
