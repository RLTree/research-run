use super::*;

#[test]
fn snapshot_reader_propagates_each_typed_record_failure_boundary() {
    let root = initialize("typed-read-stages");
    seed_each_typed_record(&root.0);
    assert_snapshot_read_failures(&root.0);
}

#[test]
fn expanded_snapshot_directories_propagate_malformed_records() {
    for directory in ["inventories", "knowledge", "relationships", "migrations"] {
        let root = initialize(directory);
        fs::write(
            root.0
                .join(".research-run")
                .join(directory)
                .join("bad.json"),
            b"{}",
        )
        .expect("bad expanded record");
        assert!(!run(&root.0, &["status", "--json"], None).status.success());
    }
}

fn seed_each_typed_record(root: &Path) {
    assert!(add_source(root, "source-one", None).status.success());
    super::recovery::add_claim(root);
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
                "Repeat"
            ],
            None
        )
        .status
        .success()
    );
    assert!(
        run(
            root,
            &[
                "evidence",
                "add",
                "--id",
                "evidence-one",
                "--claim",
                "claim-one",
                "--source",
                "source-one",
                "--stance",
                "supports",
                "--specific-evidence",
                "Specific",
                "--authorship",
                "human"
            ],
            None
        )
        .status
        .success()
    );
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
                "Reviewer"
            ],
            None
        )
        .status
        .success()
    );
}

fn assert_snapshot_read_failures(root: &Path) {
    for occurrence in 1..=7 {
        let fault = format!("inspect record#{occurrence}");
        let output = run(root, &["status", "--json"], Some(&fault));
        assert!(!output.status.success(), "snapshot ignored {fault}");
    }
    assert!(run(root, &["status", "--json"], None).status.success());
    assert!(
        !run(
            root,
            &["status", "--json"],
            Some("inspect workspace path#5")
        )
        .status
        .success()
    );
    assert!(
        !run(root, &["status", "--json"], Some("reinspect record"))
            .status
            .success()
    );
}

#[test]
fn publication_and_pending_inspection_propagate_storage_failures() {
    let root = initialize("publication-read-stages");
    assert!(add_source(&root.0, "source-one", None).status.success());
    assert!(
        !add_source(&root.0, "source-one", Some("inspect record#2"))
            .status
            .success()
    );

    let unreadable = initialize("unreadable-publication-race");
    assert!(
        !add_source(&unreadable.0, "source-one", Some("publish unreadable race"))
            .status
            .success()
    );

    for fault in ["inspect pending effects", "inspect pending effect"] {
        let pending = initialize(fault);
        assert!(
            !add_source(&pending.0, "source-one", Some(fault))
                .status
                .success()
        );
    }
}
