use super::*;

#[test]
fn mutations_reject_damaged_authority_and_unbound_reviews() {
    reject_mutation_over_damaged_references();
    reject_unbound_review_authority();
}

fn reject_mutation_over_damaged_references() {
    let root = initialize("damaged-mutation-authority");
    fs::write(
        root.0.join(".research-run/evidence/damaged.json"),
        serde_json::to_vec_pretty(&evidence("damaged")).expect("evidence JSON"),
    )
    .expect("write damaged reference");
    let workspace = Workspace::discover(&root.0).expect("discover workspace");
    assert!(workspace.add_source(&source("source-two")).is_err());
    assert!(workspace.add_claim(&claim("claim-two")).is_err());
    assert!(
        workspace
            .add_experiment(&experiment("experiment-two"))
            .is_err()
    );
    assert!(workspace.add_evidence(&evidence("evidence-two")).is_err());
    assert!(workspace.add_review(&review("review-two")).is_err());
}

fn reject_unbound_review_authority() {
    let root = initialize("unbound-review-authority");
    let workspace = Workspace::discover(&root.0).expect("discover workspace");
    assert!(workspace.add_claim(&claim("claim-one")).is_ok());

    let mut supported_without_evidence = review("supported-without-evidence");
    supported_without_evidence.decision = Assessment::Supported;
    assert!(workspace.add_review(&supported_without_evidence).is_err());

    let mut mismatched = review("mismatched-review");
    mismatched.decision = Assessment::Limited;
    mismatched.evidence_ids = vec!["evidence-missing".to_owned()];
    assert!(workspace.add_review(&mismatched).is_err());

    let mut missing_claim = review("missing-claim-review");
    missing_claim.claim_id = "claim-missing".to_owned();
    missing_claim.decision = Assessment::Limited;
    assert!(workspace.add_review(&missing_claim).is_err());
    assert!(workspace.claim_evidence_ids("claim-missing").is_err());

    fs::write(root.0.join(".research-run/manifest.json"), b"not-json")
        .expect("corrupt manifest after discovery");
    assert!(workspace.claim_evidence_ids("claim-one").is_err());
}

#[test]
fn identical_record_reads_propagate_storage_failures_for_every_writer() {
    let root = initialize("identical-read-failures");
    assert!(add_source(&root.0, "source-one", None).status.success());
    super::super::recovery::add_claim(&root.0);
    for command in retry_commands() {
        assert!(run(&root.0, &command, None).status.success());
    }
    for command in retry_commands() {
        let fault = if command.first() == Some(&"review") {
            "inspect record#14"
        } else {
            "inspect record#8"
        };
        assert!(
            !run(&root.0, &command, Some(fault)).status.success(),
            "retry {command:?} ignored its identity-read failure"
        );
    }
    let signed = crate::review_test_signing::prepare_signed_review(
        &root.0,
        "review-one",
        "claim-one",
        "limited",
        "Rationale",
        "Reviewer",
    );
    let request = signed.request.to_string_lossy().into_owned();
    let signature = signed.signature.to_string_lossy().into_owned();
    let args = [
        "review",
        "add",
        "--request",
        &request,
        "--signature",
        &signature,
    ];
    assert!(run(&root.0, &args, None).status.success());
    assert!(
        !run(&root.0, &args, Some("inspect record#14"))
            .status
            .success()
    );
}

fn retry_commands() -> Vec<Vec<&'static str>> {
    vec![
        vec![
            "experiment",
            "add",
            "--id",
            "experiment-one",
            "--question",
            "Question?",
            "--method-ref",
            "method.md",
            "--observation",
            "Observed",
            "--interpretation",
            "Interpretation",
            "--limitation",
            "Limited",
            "--outcome",
            "negative",
            "--next-move",
            "Repeat",
        ],
        vec![
            "evidence",
            "add",
            "--id",
            "evidence-one",
            "--claim",
            "claim-one",
            "--source",
            "source-one",
            "--stance",
            "context",
            "--specific-evidence",
            "Specific",
            "--authorship",
            "human",
        ],
        vec![
            "source",
            "add",
            "--id",
            "source-one",
            "--citation",
            "Citation",
            "--locator",
            "local:source",
            "--provenance",
            "human",
        ],
        vec![
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
    ]
}

#[test]
fn maximum_record_budget_remains_idempotent() {
    let root = initialize("maximum-record-budget");
    let workspace = Workspace::discover(&root.0).expect("discover workspace");
    let record = maximum_record_sized_experiment();
    assert_eq!(
        serde_json::to_vec_pretty(&record)
            .expect("experiment JSON")
            .len()
            + 1,
        1_048_576
    );
    assert_eq!(workspace.add_experiment(&record).expect("first add"), true);
    assert_eq!(workspace.add_experiment(&record).expect("retry"), false);
}

#[test]
fn manifest_and_record_directory_errors_propagate_at_their_owner() {
    let invalid_discovery = initialize("manifest-validation-discovery");
    fs::write(
        invalid_discovery.0.join(".research-run/manifest.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": 1,
            "kind": "project-manifest",
            "project_id": "bad id",
            "name": "Name",
            "declared_roots": ["."]
        }))
        .expect("manifest JSON"),
    )
    .expect("write invalid manifest");
    assert!(Workspace::discover(&invalid_discovery.0).is_err());
    let direct = Workspace::for_recovery(&invalid_discovery.0).expect("recovery workspace");
    assert!(direct.status().is_err());

    let invalid_record = initialize("record-validation-load");
    let mut stored = source("source-one");
    stored.citation.clear();
    fs::write(
        invalid_record
            .0
            .join(".research-run/sources/source-one.json"),
        serde_json::to_vec_pretty(&stored).expect("source JSON"),
    )
    .expect("write invalid source");
    let workspace = Workspace::for_recovery(&invalid_record.0).expect("recovery workspace");
    assert!(workspace.status().is_err());

    let directory = initialize("record-directory-faults");
    assert!(
        add_source(&directory.0, "source-one", None)
            .status
            .success()
    );
    for fault in ["read record directory", "read record entry"] {
        assert!(
            !run(&directory.0, &["status", "--json"], Some(fault))
                .status
                .success()
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let symlinked = initialize("record-directory-symlink");
        let outside = TempDir::new("record-directory-symlink-outside");
        fs::remove_dir(symlinked.0.join(".research-run/sources")).expect("remove sources");
        symlink(&outside.0, symlinked.0.join(".research-run/sources")).expect("symlink sources");
        let workspace = Workspace::for_recovery(&symlinked.0).expect("recovery workspace");
        assert!(workspace.status().is_err());
    }
}
