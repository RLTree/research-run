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

#[test]
fn compact_reordered_pending_json_publishes_canonical_state() {
    use research_run::domain::{FORMAT_VERSION, SourceProvenance, SourceRecord};

    let root = initialize("canonical-pending-json");
    assert!(add_source(&root.0, "source-one", None).status.success());
    let sources = root.0.join(".research-run/sources");
    let canonical_one = fs::read(sources.join("source-one.json")).expect("canonical source one");
    let identical_pending = sources.join(".source-one.json.9.1.tmp");
    fs::write(
        &identical_pending,
        br#"{"notes":"","provenance":"human","locator":"local:source","citation":"Citation","id":"source-one","kind":"source","schema_version":1}"#,
    )
    .expect("compact reordered pending source");
    assert!(run(&root.0, &["recover", "--json"], None).status.success());
    assert!(!identical_pending.exists());
    assert_eq!(
        fs::read(sources.join("source-one.json")).expect("source one after recovery"),
        canonical_one
    );

    let pending_two = sources.join(".source-two.json.9.2.tmp");
    fs::write(
        &pending_two,
        br#"{"provenance":"human","schema_version":1,"notes":"","kind":"source","locator":"local:source","id":"source-two","citation":"Citation"}"#,
    )
    .expect("reordered new pending source");
    assert!(run(&root.0, &["recover", "--json"], None).status.success());
    let expected = SourceRecord {
        schema_version: FORMAT_VERSION,
        kind: "source".to_owned(),
        id: "source-two".to_owned(),
        citation: "Citation".to_owned(),
        locator: "local:source".to_owned(),
        provenance: SourceProvenance::Human,
        notes: String::new(),
    };
    let mut expected_bytes = serde_json::to_vec_pretty(&expected).expect("canonical source JSON");
    expected_bytes.push(b'\n');
    assert_eq!(
        fs::read(sources.join("source-two.json")).expect("canonical source two"),
        expected_bytes
    );
}
