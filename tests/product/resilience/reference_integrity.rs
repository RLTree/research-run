use super::*;

#[path = "reference_integrity/command_references.rs"]
mod command_references;

fn assert_fails(output: Output, context: &str) {
    assert!(
        !output.status.success(),
        "{context} unexpectedly succeeded\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_json(path: &Path, value: serde_json::Value) {
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).expect("fixture JSON"),
    )
    .expect("write JSON fixture");
}

fn claim(root: &Path, id: &str) {
    write_json(
        &root.join(".research-run/claims").join(format!("{id}.json")),
        json!({
            "schema_version": 1,
            "kind": "claim",
            "id": id,
            "text": "Claim",
            "scope": "Scope",
            "owner": "Owner",
            "authorship": "human"
        }),
    );
}

fn evidence(
    root: &Path,
    id: &str,
    claim_id: &str,
    source_id: Option<&str>,
    experiment_id: Option<&str>,
    artifact: Option<&str>,
) {
    write_json(
        &root
            .join(".research-run/evidence")
            .join(format!("{id}.json")),
        json!({
            "schema_version": 1,
            "kind": "evidence",
            "id": id,
            "claim_id": claim_id,
            "source_id": source_id,
            "experiment_id": experiment_id,
            "artifact": artifact,
            "stance": "context",
            "specific_evidence": "Specific",
            "authorship": "human"
        }),
    );
}

fn review(root: &Path, id: &str, claim_id: &str) {
    write_json(
        &root
            .join(".research-run/reviews")
            .join(format!("{id}.json")),
        json!({
            "schema_version": 1,
            "kind": "review",
            "id": id,
            "claim_id": claim_id,
            "evidence_ids": [],
            "decision": "limited",
            "rationale": "Rationale",
            "reviewer": "Reviewer"
        }),
    );
}

#[test]
fn installed_validation_reports_each_cross_record_reference_failure() {
    reject_unknown_and_duplicate_record_references();
    reject_symlinked_artifact_references();
}

fn reject_unknown_and_duplicate_record_references() {
    let unknown_claim = initialize("unknown-claim-reference");
    evidence(
        &unknown_claim.0,
        "evidence-one",
        "claim-missing",
        Some("source-missing"),
        None,
        None,
    );
    assert_fails(
        run(&unknown_claim.0, &["status", "--json"], None),
        "unknown claim and source",
    );
    assert_fails(
        run(&unknown_claim.0, &["validate", "--json"], None),
        "validate unknown claim and source",
    );

    let unknown_experiment = initialize("unknown-experiment-reference");
    claim(&unknown_experiment.0, "claim-one");
    evidence(
        &unknown_experiment.0,
        "evidence-one",
        "claim-one",
        None,
        Some("experiment-missing"),
        None,
    );
    assert_fails(
        run(&unknown_experiment.0, &["status", "--json"], None),
        "unknown experiment",
    );

    let unknown_review = initialize("unknown-review-reference");
    review(&unknown_review.0, "review-one", "claim-missing");
    assert_fails(
        run(&unknown_review.0, &["status", "--json"], None),
        "unknown reviewed claim",
    );

    let duplicate_review = initialize("duplicate-review-reference");
    claim(&duplicate_review.0, "claim-one");
    review(&duplicate_review.0, "review-one", "claim-one");
    review(&duplicate_review.0, "review-two", "claim-one");
    assert_fails(
        run(&duplicate_review.0, &["status", "--json"], None),
        "duplicate reviewed claim",
    );

    let cross_claim = initialize("cross-claim-review-reference");
    claim(&cross_claim.0, "claim-one");
    claim(&cross_claim.0, "claim-two");
    evidence(
        &cross_claim.0,
        "evidence-two",
        "claim-two",
        None,
        None,
        Some("artifact.txt"),
    );
    write_json(
        &cross_claim.0.join(".research-run/reviews/review-one.json"),
        json!({
            "schema_version": 1, "kind": "review", "id": "review-one",
            "claim_id": "claim-one", "evidence_ids": ["evidence-two"],
            "decision": "limited", "rationale": "Rationale", "reviewer": "Reviewer"
        }),
    );
    let output = run(&cross_claim.0, &["validate", "--json"], None);
    assert_fails(output.clone(), "cross-claim review evidence");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("belongs to claim claim-two"),
        "cross-claim diagnostic was not canonical: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[cfg(unix)]
fn reject_symlinked_artifact_references() {
    {
        use std::os::unix::fs::symlink;
        let unsafe_artifact = initialize("unsafe-artifact-reference");
        let outside = TempDir::new("unsafe-artifact-outside");
        symlink(&outside.0, unsafe_artifact.0.join("artifact-link")).expect("artifact symlink");
        claim(&unsafe_artifact.0, "claim-one");
        evidence(
            &unsafe_artifact.0,
            "evidence-one",
            "claim-one",
            None,
            None,
            Some("artifact-link/result.txt"),
        );
        assert_fails(
            run(&unsafe_artifact.0, &["status", "--json"], None),
            "symlinked artifact",
        );

        let unsafe_experiment = initialize("unsafe-experiment-artifact");
        let outside = TempDir::new("unsafe-experiment-outside");
        symlink(&outside.0, unsafe_experiment.0.join("artifact-link"))
            .expect("experiment artifact symlink");
        write_json(
            &unsafe_experiment
                .0
                .join(".research-run/experiments/experiment-one.json"),
            json!({
                "schema_version": 1, "kind": "experiment", "id": "experiment-one",
                "question": "Question?", "method_ref": "method.md", "observations": ["Observed"],
                "interpretation": "Interpretation", "limitations": ["Limited"],
                "outcome": "inconclusive", "next_move": "Repeat",
                "artifacts": [{
                    "locator_type": "workspace", "locator": "artifact-link/result.txt",
                    "description": "Result", "digest": null
                }]
            }),
        );
        assert_fails(
            run(&unsafe_experiment.0, &["status", "--json"], None),
            "symlinked experiment artifact",
        );
    }
}

#[cfg(not(unix))]
fn reject_symlinked_artifact_references() {}
