use super::*;

#[test]
fn installed_commands_reject_missing_references_and_duplicate_reviews() {
    let root = initialize("command-reference-errors");
    reject_missing_evidence_references(&root.0);
    reject_missing_and_duplicate_review_references(&root.0);
}

fn reject_missing_evidence_references(root: &Path) {
    let evidence_missing_claim = run(
        root,
        &[
            "evidence",
            "add",
            "--id",
            "evidence-one",
            "--claim",
            "claim-missing",
            "--source",
            "source-missing",
            "--stance",
            "context",
            "--specific-evidence",
            "Specific",
            "--authorship",
            "human",
        ],
        None,
    );
    assert_fails(evidence_missing_claim, "missing claim");

    claim(root, "claim-one");
    let evidence_missing_experiment = run(
        root,
        &[
            "evidence",
            "add",
            "--id",
            "evidence-two",
            "--claim",
            "claim-one",
            "--experiment",
            "experiment-missing",
            "--stance",
            "context",
            "--specific-evidence",
            "Specific",
            "--authorship",
            "human",
        ],
        None,
    );
    assert_fails(evidence_missing_experiment, "missing experiment");
}

fn reject_missing_and_duplicate_review_references(root: &Path) {
    let missing_review_claim = run(
        root,
        &[
            "review",
            "prepare",
            "--id",
            "review-missing",
            "--claim",
            "claim-missing",
            "--decision",
            "supported",
            "--rationale",
            "Rationale",
            "--reviewer",
            "Reviewer",
        ],
        None,
    );
    assert_fails(missing_review_claim, "missing review claim");

    crate::review_test_signing::add_signed_review(
        root,
        "review-one",
        "claim-one",
        "limited",
        "Rationale",
        "Reviewer",
    );
    let signed = crate::review_test_signing::prepare_signed_review(
        root,
        "review-two",
        "claim-one",
        "limited",
        "Second",
        "Reviewer",
    );
    let duplicate = run(
        root,
        &[
            "review",
            "add",
            "--request",
            &signed.request.to_string_lossy(),
            "--signature",
            &signed.signature.to_string_lossy(),
        ],
        None,
    );
    assert_fails(duplicate, "duplicate review");
}

#[test]
fn installed_reader_rejects_filename_type_and_snapshot_budgets() {
    let mismatch = initialize("filename-mismatch");
    write_json(
        &mismatch.0.join(".research-run/sources/wrong.json"),
        json!({
            "schema_version": 1,
            "kind": "source",
            "id": "source-one",
            "citation": "Citation",
            "locator": "local:source",
            "provenance": "human",
            "notes": ""
        }),
    );
    assert_fails(
        run(&mismatch.0, &["status", "--json"], None),
        "filename mismatch",
    );

    let non_file = initialize("non-file-record");
    fs::create_dir(non_file.0.join(".research-run/sources/directory.json"))
        .expect("record-shaped directory");
    assert_fails(
        run(&non_file.0, &["status", "--json"], None),
        "non-file record",
    );

    let too_large = initialize("oversized-read");
    fs::write(
        too_large.0.join(".research-run/sources/large.json"),
        vec![b'x'; 1_048_577],
    )
    .expect("oversized record");
    assert_fails(
        run(&too_large.0, &["status", "--json"], None),
        "oversized stored record",
    );

    let snapshot = initialize("snapshot-budget");
    assert_fails(
        run(
            &snapshot.0,
            &["status", "--json"],
            Some("snapshot byte budget"),
        ),
        "snapshot byte budget",
    );
}
