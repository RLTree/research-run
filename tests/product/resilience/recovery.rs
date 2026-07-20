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
    let pending = seed_identical_pending(&conflict.0);
    let output = run(
        &conflict.0,
        &["recover", "--json"],
        Some("recovery target conflict"),
    );
    assert!(!output.status.success());
    assert!(pending.exists());

    let read_failure = initialize("recovery-target-read");
    let pending = seed_identical_pending(&read_failure.0);
    let output = run(
        &read_failure.0,
        &["recover", "--json"],
        Some("inspect record#5"),
    );
    assert!(!output.status.success());
    assert!(pending.exists());

    let cleanup = initialize("recovery-discard-cleanup");
    seed_identical_pending(&cleanup.0);
    let output = run(
        &cleanup.0,
        &["recover", "--json"],
        Some("remove identical pending file"),
    );
    assert!(!output.status.success());

    let unsynced = initialize("recovery-discard-sync");
    let pending = seed_identical_pending(&unsynced.0);
    let output = run(
        &unsynced.0,
        &["recover", "--json"],
        Some("sync record directory"),
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("directory sync failed"));
    assert!(!pending.exists());

    assert_cleanup_sync_is_ambiguous();

    let discard = initialize("recovery-discard-fault");
    let pending = seed_identical_pending(&discard.0);
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
        "create pending record",
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

fn seed_identical_pending(root: &Path) -> PathBuf {
    assert!(add_source(root, "source-one", None).status.success());
    let directory = root.join(".research-run/sources");
    let pending = directory.join(".source-one.json.9.1.tmp");
    fs::copy(directory.join("source-one.json"), &pending).expect("identical pending source");
    pending
}

fn assert_cleanup_sync_is_ambiguous() {
    let root = initialize("recovery-published-cleanup-sync");
    let pending = pending_source(&root.0, "source-one", 1);
    let output = run(
        &root.0,
        &["recover", "--json"],
        Some("sync record directory#3"),
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("directory sync failed"));
    assert!(!pending.exists());
    assert!(
        root.0
            .join(".research-run/sources/source-one.json")
            .exists()
    );
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
    fs::create_dir(root.0.join("artifacts")).expect("artifact directory");
    fs::write(root.0.join("artifacts/result.txt"), b"observation").expect("artifact");
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

    crate::review_test_signing::add_signed_review(
        &root.0,
        "review-template",
        "claim-one",
        "supported",
        "Rationale",
        "Reviewer",
    );
    let reviews = root.0.join(".research-run/reviews");
    let template = reviews.join("review-template.json");
    let first = crate::review_test_signing::signed_review_record(
        &root.0,
        "review-one",
        "claim-one",
        "supported",
        "Rationale",
        "Reviewer",
    );
    let second = crate::review_test_signing::signed_review_record(
        &root.0,
        "review-two",
        "claim-one",
        "supported",
        "Rationale",
        "Reviewer",
    );
    fs::remove_file(template).expect("remove canonical template");
    fs::write(
        reviews.join(".review-one.json.9.1.tmp"),
        serde_json::to_vec_pretty(&first).expect("review one"),
    )
    .expect("pending review one");
    fs::write(
        reviews.join(".review-two.json.9.2.tmp"),
        serde_json::to_vec_pretty(&second).expect("review two"),
    )
    .expect("pending review two");
    let output = run(&root.0, &["recover", "--json"], None);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("duplicate review authority binding"),
        "{stderr}"
    );
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
