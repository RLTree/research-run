use super::*;

#[test]
fn identical_pending_claim_experiment_and_evidence_are_durable_cleanup() {
    let root = initialize("identical-typed-recovery");
    add_claim(&root.0);
    assert!(
        run(
            &root.0,
            &[
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
            None,
        )
        .status
        .success()
    );
    assert!(
        run(
            &root.0,
            &[
                "evidence",
                "add",
                "--id",
                "evidence-one",
                "--claim",
                "claim-one",
                "--experiment",
                "experiment-one",
                "--stance",
                "limits",
                "--specific-evidence",
                "Specific",
                "--authorship",
                "human",
            ],
            None,
        )
        .status
        .success()
    );
    for (directory, id) in [
        ("claims", "claim-one"),
        ("experiments", "experiment-one"),
        ("evidence", "evidence-one"),
    ] {
        let directory = root.0.join(".research-run").join(directory);
        fs::copy(
            directory.join(format!("{id}.json")),
            directory.join(format!(".{id}.json.9.1.tmp")),
        )
        .expect("identical pending record");
    }
    let output = run(&root.0, &["recover", "--json"], None);
    assert!(output.status.success());
    let recovery: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("recovery JSON");
    assert_eq!(recovery["discarded_identical"].as_array().unwrap().len(), 3);
}
