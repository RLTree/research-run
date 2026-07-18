use super::*;

fn pending(root: &Path, directory: &str, id: &str, value: serde_json::Value) -> PathBuf {
    let path = root
        .join(".research-run")
        .join(directory)
        .join(format!(".{id}.json.11.1.tmp"));
    fs::write(
        &path,
        serde_json::to_vec_pretty(&value).expect("pending JSON"),
    )
    .expect("write pending record");
    path
}

fn pending_source_value(id: &str) -> serde_json::Value {
    json!({
        "schema_version": 1, "kind": "source", "id": id,
        "citation": "Citation", "locator": "local:source", "provenance": "human", "notes": ""
    })
}

#[test]
fn recover_propagates_lock_and_final_snapshot_failures() {
    let root = initialize("recover-stage-propagation");
    assert!(
        !run(
            &root.0,
            &["recover", "--json"],
            Some("open workspace write lock")
        )
        .status
        .success()
    );
    assert!(
        !run(&root.0, &["recover", "--json"], Some("inspect record#2"))
            .status
            .success()
    );
    assert!(
        !run(&root.0, &["recover", "--json"], Some("final snapshot load"))
            .status
            .success()
    );
    assert!(
        !run(
            &root.0,
            &["recover", "--json"],
            Some("inspect workspace path#4")
        )
        .status
        .success()
    );
}

#[test]
fn single_pending_review_recovers_without_false_duplicate() {
    let root = initialize("single-pending-review");
    super::recovery::add_claim(&root.0);
    pending(
        &root.0,
        "reviews",
        "review-one",
        json!({
            "schema_version": 1, "kind": "review", "id": "review-one",
            "claim_id": "claim-one", "evidence_ids": [], "decision": "limited",
            "rationale": "Rationale", "reviewer": "Reviewer"
        }),
    );
    assert!(run(&root.0, &["recover", "--json"], None).status.success());
}

#[test]
fn pending_manifest_experiment_evidence_and_review_fail_closed() {
    reject_invalid_pending_manifests();
    reject_symlinked_pending_artifacts();
    reject_invalid_pending_reviews();
}

fn reject_invalid_pending_manifests() {
    for (label, value) in [
        ("manifest-malformed", json!({"not": "a manifest"})),
        (
            "manifest-invalid",
            json!({
                "schema_version": 1, "kind": "project-manifest", "project_id": "bad id",
                "name": "Name", "declared_roots": ["."]
            }),
        ),
    ] {
        let root = initialize(label);
        pending(&root.0, ".", "manifest", value);
        assert!(!run(&root.0, &["recover", "--json"], None).status.success());
    }

    let noncanonical = initialize("manifest-noncanonical-target");
    pending(
        &noncanonical.0,
        ".",
        "other",
        json!({
            "schema_version": 1, "kind": "project-manifest", "project_id": "coverage",
            "name": "Coverage", "declared_roots": ["."]
        }),
    );
    assert!(
        !run(&noncanonical.0, &["recover", "--json"], None)
            .status
            .success()
    );
}

#[cfg(unix)]
fn reject_symlinked_pending_artifacts() {
    {
        use std::os::unix::fs::symlink;
        let experiment = initialize("pending-experiment-symlink");
        let outside = TempDir::new("pending-experiment-outside");
        symlink(&outside.0, experiment.0.join("artifact-link")).expect("artifact symlink");
        pending(
            &experiment.0,
            "experiments",
            "experiment-one",
            json!({
                "schema_version": 1, "kind": "experiment", "id": "experiment-one",
                "question": "Question?", "method_ref": "method.md", "observations": ["Observed"],
                "interpretation": "Interpretation", "limitations": ["Limited"],
                "outcome": "inconclusive", "next_move": "Repeat",
                "artifacts": [{"locator_type":"workspace", "locator":"artifact-link/result", "description":"Result", "digest":null}]
            }),
        );
        assert!(
            !run(&experiment.0, &["recover", "--json"], None)
                .status
                .success()
        );

        let evidence = initialize("pending-evidence-symlink");
        let outside = TempDir::new("pending-evidence-outside");
        symlink(&outside.0, evidence.0.join("artifact-link")).expect("artifact symlink");
        super::recovery::add_claim(&evidence.0);
        pending(
            &evidence.0,
            "evidence",
            "evidence-one",
            json!({
                "schema_version": 1, "kind": "evidence", "id": "evidence-one",
                "claim_id": "claim-one", "source_id": null, "experiment_id": null,
                "artifact": "artifact-link/result", "stance": "context",
                "specific_evidence": "Specific", "authorship": "human"
            }),
        );
        assert!(
            !run(&evidence.0, &["recover", "--json"], None)
                .status
                .success()
        );

        let pending_link = initialize("pending-record-symlink");
        let outside_record = outside.0.join("source.json");
        fs::write(
            &outside_record,
            serde_json::to_vec(&pending_source_value("source-one")).expect("source JSON"),
        )
        .expect("outside pending record");
        symlink(
            outside_record,
            pending_link
                .0
                .join(".research-run/sources/.source-one.json.11.1.tmp"),
        )
        .expect("pending record symlink");
        assert!(
            !run(&pending_link.0, &["recover", "--json"], None)
                .status
                .success()
        );
    }
}

#[cfg(not(unix))]
fn reject_symlinked_pending_artifacts() {}

fn reject_invalid_pending_reviews() {
    let review = initialize("pending-review-missing-claim");
    pending(
        &review.0,
        "reviews",
        "review-one",
        json!({
            "schema_version": 1, "kind": "review", "id": "review-one",
            "claim_id": "claim-missing", "evidence_ids": [], "decision": "limited",
            "rationale": "Rationale", "reviewer": "Reviewer"
        }),
    );
    assert!(
        !run(&review.0, &["recover", "--json"], None)
            .status
            .success()
    );

    let corrupt_reviews = initialize("pending-review-load-failure");
    super::recovery::add_claim(&corrupt_reviews.0);
    fs::write(
        corrupt_reviews.0.join(".research-run/reviews/bad.json"),
        b"not-json",
    )
    .expect("corrupt review");
    pending(
        &corrupt_reviews.0,
        "reviews",
        "review-one",
        json!({
            "schema_version": 1, "kind": "review", "id": "review-one",
            "claim_id": "claim-one", "evidence_ids": [], "decision": "limited",
            "rationale": "Rationale", "reviewer": "Reviewer"
        }),
    );
    assert!(
        !run(&corrupt_reviews.0, &["recover", "--json"], None)
            .status
            .success()
    );

    let review_read_fault = initialize("pending-review-read-fault");
    super::recovery::add_claim(&review_read_fault.0);
    pending(
        &review_read_fault.0,
        "reviews",
        "review-one",
        json!({
            "schema_version": 1, "kind": "review", "id": "review-one",
            "claim_id": "claim-one", "evidence_ids": [], "decision": "limited",
            "rationale": "Rationale", "reviewer": "Reviewer"
        }),
    );
    assert!(
        !run(
            &review_read_fault.0,
            &["recover", "--json"],
            Some("read record directory#6")
        )
        .status
        .success()
    );

    reject_mismatched_pending_review_graph();
    reject_malformed_pending_review_authority();
}

fn reject_malformed_pending_review_authority() {
    for (label, target_id, record_id, decision) in [
        (
            "pending-supported-without-evidence",
            "review-one",
            "review-one",
            "supported",
        ),
        (
            "pending-review-filename-mismatch",
            "review-one",
            "review-other",
            "limited",
        ),
    ] {
        let root = initialize(label);
        super::recovery::add_claim(&root.0);
        pending(
            &root.0,
            "reviews",
            target_id,
            json!({
                "schema_version": 1, "kind": "review", "id": record_id,
                "claim_id": "claim-one", "evidence_ids": [], "decision": decision,
                "rationale": "Rationale", "reviewer": "Reviewer"
            }),
        );
        assert!(!run(&root.0, &["recover", "--json"], None).status.success());
    }
}

fn reject_mismatched_pending_review_graph() {
    let mismatched_graph = initialize("pending-review-graph-mismatch");
    super::recovery::add_claim(&mismatched_graph.0);
    pending(
        &mismatched_graph.0,
        "reviews",
        "review-one",
        json!({
            "schema_version": 1, "kind": "review", "id": "review-one",
            "claim_id": "claim-one", "evidence_ids": ["evidence-missing"],
            "decision": "limited", "rationale": "Rationale", "reviewer": "Reviewer"
        }),
    );
    assert!(
        !run(&mismatched_graph.0, &["recover", "--json"], None)
            .status
            .success()
    );
}

#[test]
fn recovery_plan_propagates_each_storage_transition() {
    let symlink_race = initialize("pending-record-symlink-race");
    assert!(
        !run(
            &symlink_race.0,
            &["recover", "--json"],
            Some("pending record symlink race")
        )
        .status
        .success()
    );

    let preflight = initialize("preflight-target-read");
    let pending_path = pending(
        &preflight.0,
        "sources",
        "source-one",
        pending_source_value("source-one"),
    );
    fs::copy(
        &pending_path,
        preflight.0.join(".research-run/sources/source-one.json"),
    )
    .expect("canonical source");
    assert!(
        !run(
            &preflight.0,
            &["recover", "--json"],
            Some("inspect record#4")
        )
        .status
        .success()
    );

    for fault in [
        "inspect workspace path#3",
        "inspect workspace path#14",
        "inspect record#2",
        "open record",
        "inspect opened record",
        "open record directory for sync#1",
        "open record directory for sync#2",
    ] {
        let root = initialize(fault);
        pending(
            &root.0,
            "sources",
            "source-one",
            pending_source_value("source-one"),
        );
        assert!(
            !run(&root.0, &["recover", "--json"], Some(fault))
                .status
                .success(),
            "recover ignored {fault}"
        );
    }

    let discard = initialize("discard-second-read");
    let pending_path = pending(
        &discard.0,
        "sources",
        "source-one",
        pending_source_value("source-one"),
    );
    fs::copy(
        &pending_path,
        discard.0.join(".research-run/sources/source-one.json"),
    )
    .expect("canonical source");
    assert!(
        !run(&discard.0, &["recover", "--json"], Some("inspect record#5"))
            .status
            .success()
    );
}
