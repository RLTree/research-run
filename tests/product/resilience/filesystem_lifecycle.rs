use super::*;

#[test]
fn installed_process_exercises_each_lifecycle_storage_boundary() {
    for occurrence in 1..=17 {
        let root = TempDir::new(&format!("init-path-stage-{occurrence}"));
        let fault = format!("inspect workspace path#{occurrence}");
        let output = run(
            &root.0,
            &["init", "project", "--name", "Lifecycle stages"],
            Some(&fault),
        );
        assert!(!output.status.success(), "init ignored {fault}");
    }
    for occurrence in 1..=7 {
        let root = TempDir::new(&format!("init-directory-stage-{occurrence}"));
        let fault = format!("create project directory#{occurrence}");
        let output = run(
            &root.0,
            &["init", "project", "--name", "Directory stages"],
            Some(&fault),
        );
        assert!(!output.status.success(), "init ignored {fault}");
    }

    let workspace = initialize("discover-path-stages");
    for occurrence in 1..=4 {
        let fault = format!("inspect workspace path#{occurrence}");
        let output = run(&workspace.0, &["status", "--json"], Some(&fault));
        assert!(!output.status.success(), "discovery ignored {fault}");
    }
    for occurrence in 1..=2 {
        let fault = format!("inspect workspace path#{occurrence}");
        let output = run(&workspace.0, &["recover", ".", "--json"], Some(&fault));
        assert!(!output.status.success(), "recovery routing ignored {fault}");
    }

    assert!(
        !run(
            &workspace.0,
            &["status", "--json"],
            Some("inspect workspace root")
        )
        .status
        .success()
    );
    assert!(
        !run(
            &workspace.0,
            &["recover", ".", "--json"],
            Some("inspect workspace root")
        )
        .status
        .success()
    );

    let nested = TempDir::new("nested-directory-propagation");
    assert!(
        !run(
            &nested.0,
            &["init", "parent/project", "--name", "Nested"],
            Some("create project directory#1"),
        )
        .status
        .success()
    );
}

#[test]
fn every_write_command_propagates_lock_path_failure() {
    let root = initialize("command-lock-paths");
    assert!(add_source(&root.0, "source-one", None).status.success());
    super::recovery::add_claim(&root.0);
    for args in write_command_arguments() {
        let output = run(&root.0, &args, Some("open workspace write lock"));
        assert!(
            !output.status.success(),
            "command {args:?} ignored lock failure"
        );
    }
}

fn write_command_arguments() -> Vec<Vec<&'static str>> {
    vec![
        vec![
            "source",
            "add",
            "--id",
            "source-two",
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
            "claim-two",
            "--text",
            "Claim",
            "--scope",
            "Scope",
            "--owner",
            "Owner",
            "--authorship",
            "human",
        ],
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
            "supports",
            "--specific-evidence",
            "Specific",
            "--authorship",
            "human",
        ],
        vec![
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
            "Reviewer",
        ],
    ]
}

#[test]
fn snapshot_reader_propagates_each_typed_record_failure_boundary() {
    let root = initialize("typed-read-stages");
    seed_each_typed_record(&root.0);
    assert_snapshot_read_failures(&root.0);
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
