use super::*;

#[path = "recovery/identical_records.rs"]
mod identical_records;
#[path = "recovery/reference_publication.rs"]
mod reference_publication;

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
        assert!(pending.exists(), "fault {fault} discarded pending evidence");
        assert!(
            !root
                .0
                .join(".research-run/sources/source-one.json")
                .exists(),
            "fault {fault} published canonical state"
        );
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

    let unsynced = initialize("recovery-discard-sync");
    let pending = pending_source(&unsynced.0, "source-one", 1);
    let canonical = unsynced.0.join(".research-run/sources/source-one.json");
    fs::copy(&pending, &canonical).expect("copy canonical source");
    let output = run(
        &unsynced.0,
        &["recover", "--json"],
        Some("sync record directory"),
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
        "hard link recovered record",
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
            "evidence_ids": ["evidence-artifact"],
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
    assert!(String::from_utf8_lossy(&output.stderr).contains("multiple v0.1 reviews"));
    assert!(reviews.join(".review-one.json.9.1.tmp").exists());
    assert!(reviews.join(".review-two.json.9.2.tmp").exists());
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
