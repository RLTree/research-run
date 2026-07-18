use super::*;

#[test]
fn installed_process_exercises_recovery_preflight_faults() {
    for fault in [
        "pending record count",
        "recovery record count",
        "publish recovered record",
        "recovered references",
    ] {
        let root = initialize(fault);
        let pending = pending_source(&root.0, "source-one", 1);
        let output = run(&root.0, &["recover", "--json"], Some(fault));
        assert!(!output.status.success(), "fault {fault} was ignored");
        if fault == "recovered references" {
            assert!(
                root.0
                    .join(".research-run/sources/source-one.json")
                    .exists()
            );
        } else {
            assert!(pending.exists(), "fault {fault} discarded pending evidence");
        }
    }
}

#[test]
fn installed_process_exercises_recovery_commit_faults() {
    let conflict = initialize("recovery-conflict");
    let pending = pending_source(&conflict.0, "source-one", 1);
    let canonical = conflict.0.join(".research-run/sources/source-one.json");
    fs::copy(&pending, &canonical).expect("copy canonical source");
    let output = run(
        &conflict.0,
        &["recover", "--json"],
        Some("recovery target conflict"),
    );
    assert!(!output.status.success());
    assert!(pending.exists());

    let cleanup = initialize("recovery-discard-cleanup");
    let pending = pending_source(&cleanup.0, "source-one", 1);
    let canonical = cleanup.0.join(".research-run/sources/source-one.json");
    fs::copy(&pending, &canonical).expect("copy canonical source");
    let output = run(
        &cleanup.0,
        &["recover", "--json"],
        Some("remove identical pending file"),
    );
    assert!(!output.status.success());

    let discard = initialize("recovery-discard-fault");
    let pending = pending_source(&discard.0, "source-one", 1);
    let canonical = discard.0.join(".research-run/sources/source-one.json");
    fs::copy(&pending, &canonical).expect("copy canonical source");
    let output = run(
        &discard.0,
        &["recover", "--json"],
        Some("publish recovered record"),
    );
    assert!(!output.status.success());
    assert!(pending.exists());

    for fault in [
        "read recovery directory",
        "read recovery entry",
        "count recovery directory",
        "count recovery entry",
        "remove recovered pending file",
    ] {
        let root = initialize(fault);
        let pending = pending_source(&root.0, "source-one", 1);
        let output = run(&root.0, &["recover", "--json"], Some(fault));
        assert!(!output.status.success(), "fault {fault} was ignored");
        if fault != "remove recovered pending file" {
            assert!(pending.exists());
        }
    }
}

pub(super) fn add_claim(root: &Path) {
    assert!(
        run(
            root,
            &[
                "claim",
                "add",
                "--id",
                "claim-one",
                "--text",
                "Claim",
                "--scope",
                "Scope",
                "--owner",
                "Owner",
                "--authorship",
                "human",
            ],
            None,
        )
        .status
        .success()
    );
}

#[test]
fn installed_process_exercises_artifact_and_recovery_reference_variants() {
    let root = initialize("reference-variants");
    add_claim(&root.0);
    let output = run(
        &root.0,
        &[
            "evidence",
            "add",
            "--id",
            "evidence-artifact",
            "--claim",
            "claim-one",
            "--artifact",
            "artifacts/result.txt",
            "--stance",
            "context",
            "--specific-evidence",
            "Artifact observation",
            "--authorship",
            "human",
        ],
        None,
    );
    assert!(output.status.success());

    let review = |id: &str| {
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "kind": "review",
            "id": id,
            "claim_id": "claim-one",
            "decision": "supported",
            "rationale": "Rationale",
            "reviewer": "Reviewer"
        }))
        .expect("review JSON")
    };
    let reviews = root.0.join(".research-run/reviews");
    fs::write(
        reviews.join(".review-one.json.9.1.tmp"),
        review("review-one"),
    )
    .expect("pending review one");
    fs::write(
        reviews.join(".review-two.json.9.2.tmp"),
        review("review-two"),
    )
    .expect("pending review two");
    let output = run(&root.0, &["recover", "--json"], None);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("multiple pending review"));
}

fn pending_evidence(root: &Path, id: &str, experiment: Option<&str>, artifact: Option<&str>) {
    let path = root
        .join(".research-run/evidence")
        .join(format!(".{id}.json.9.1.tmp"));
    fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "kind": "evidence",
            "id": id,
            "claim_id": "claim-one",
            "source_id": null,
            "experiment_id": experiment,
            "artifact": artifact,
            "stance": "context",
            "specific_evidence": "Recovered evidence",
            "authorship": "human"
        }))
        .expect("evidence JSON"),
    )
    .expect("pending evidence");
}

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
}

fn reject_second_review_during_recovery(root: &Path) {
    assert!(
        run(
            root,
            &[
                "review",
                "add",
                "--id",
                "review-one",
                "--claim",
                "claim-one",
                "--decision",
                "supported",
                "--rationale",
                "Rationale",
                "--reviewer",
                "Reviewer",
            ],
            None,
        )
        .status
        .success()
    );
    let pending = root.join(".research-run/reviews/.review-two.json.9.1.tmp");
    fs::write(
        &pending,
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "kind": "review",
            "id": "review-two",
            "claim_id": "claim-one",
            "decision": "limited",
            "rationale": "Second",
            "reviewer": "Reviewer"
        }))
        .expect("review JSON"),
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
