use super::*;

#[path = "recovery_transitions/manifest_validation.rs"]
mod manifest_validation;
#[path = "recovery_transitions/review_publication.rs"]
mod review_publication;

use review_publication::reject_invalid_pending_reviews;

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
fn recovery_batch_propagates_collection_manifest_and_pending_budget_failures() {
    for occurrence in 2..=6 {
        let root = initialize("recovery-collection-fault");
        let fault = format!("read recovery directory#{occurrence}");
        assert!(
            !run(&root.0, &["recover", "--json"], Some(&fault))
                .status
                .success()
        );
    }

    let missing = initialize("recovery-missing-manifest");
    fs::remove_file(missing.0.join(".research-run/manifest.json")).expect("remove manifest");
    assert!(
        !run(&missing.0, &["recover", "--json"], None)
            .status
            .success()
    );

    let conflict = initialize("recovery-manifest-conflict");
    let state = conflict.0.join(".research-run");
    let manifest = fs::read(state.join("manifest.json")).expect("manifest");
    fs::write(state.join(".manifest.json.11.1.tmp"), &manifest).expect("first pending manifest");
    let mut changed: serde_json::Value = serde_json::from_slice(&manifest).expect("manifest JSON");
    changed["name"] = json!("Changed");
    fs::write(
        state.join(".manifest.json.11.2.tmp"),
        serde_json::to_vec_pretty(&changed).expect("changed manifest"),
    )
    .expect("second pending manifest");
    assert!(
        !run(&conflict.0, &["recover", "--json"], None)
            .status
            .success()
    );

    let publication = initialize("recovery-manifest-publication-fault");
    let state = publication.0.join(".research-run");
    let pending_manifest = state.join(".manifest.json.11.1.tmp");
    fs::rename(state.join("manifest.json"), &pending_manifest).expect("pending manifest");
    assert!(
        !run(
            &publication.0,
            &["recover", "--json"],
            Some("publish recovered record")
        )
        .status
        .success()
    );
    assert!(pending_manifest.exists());

    let budget = initialize("recovery-pending-budget-fault");
    pending(
        &budget.0,
        "sources",
        "source-one",
        pending_source_value("source-one"),
    );
    assert!(
        !run(
            &budget.0,
            &["recover", "--json"],
            Some("snapshot byte budget#3")
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
