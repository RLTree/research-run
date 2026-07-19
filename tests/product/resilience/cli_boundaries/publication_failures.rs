use super::*;

#[test]
fn installed_cli_rejects_lifecycle_and_publication_failures() {
    exercise_workspace_lifecycle_failures();
    exercise_record_size_limits();
    exercise_interrupted_publication();
    exercise_noncanonical_pending_names();
    exercise_direct_workspace_symlink();
}

fn exercise_workspace_lifecycle_failures() {
    let no_workspace = TempDir::new("no-workspace");
    assert_fails(
        &run(&no_workspace.0, &["status", "--json"], None),
        "workspace discovery without a manifest",
    );

    let invalid_name = TempDir::new("invalid-name");
    assert_fails(
        &run(&invalid_name.0, &["init", "project", "--name", ""], None),
        "empty project name",
    );

    for fault in [
        "create project directory",
        "acquire workspace write lock",
        "inspect workspace write lock",
        "reinspect workspace write lock",
        "write pending record",
        "sync pending record",
    ] {
        let root = TempDir::new(fault);
        let output = run(
            &root.0,
            &["init", "project", "--name", "Lifecycle"],
            Some(fault),
        );
        assert_fails(&output, fault);
    }
}

fn exercise_record_size_limits() {
    let large = initialize("large-record");
    let huge = "x".repeat(1_048_577);
    let workspace = Workspace::discover(&large.0).expect("discover large-record workspace");
    let oversized = SourceRecord {
        schema_version: FORMAT_VERSION,
        kind: "source".to_owned(),
        id: "source-large".to_owned(),
        citation: huge,
        locator: "local:source".to_owned(),
        provenance: SourceProvenance::Human,
        notes: String::new(),
    };
    assert!(workspace.add_source(&oversized).is_err());

    let bounded = "a".repeat(65_536);
    let oversized_experiment = ExperimentReceipt {
        schema_version: FORMAT_VERSION,
        kind: "experiment".to_owned(),
        id: "experiment-large".to_owned(),
        question: "Question?".to_owned(),
        method_ref: "method.md".to_owned(),
        observations: vec!["Observed".to_owned()],
        interpretation: "Interpretation".to_owned(),
        limitations: vec!["Limited".to_owned()],
        outcome: Outcome::Inconclusive,
        next_move: "Repeat".to_owned(),
        artifacts: (0..8)
            .map(|_| ArtifactPointer {
                locator_type: ArtifactLocatorType::External,
                locator: bounded.clone(),
                description: bounded.clone(),
                digest: None,
            })
            .collect(),
    };
    assert!(workspace.add_experiment(&oversized_experiment).is_err());
}

fn exercise_interrupted_publication() {
    let interrupted = initialize("pending-effect");
    let pending = pending_source(&interrupted.0, "source-one", 1);
    let output = add_source(&interrupted.0, "source-one", None);
    assert_fails(&output, "pending publication");
    assert!(pending.exists());
}

fn exercise_noncanonical_pending_names() {
    for name in [
        "plain.tmp",
        ".plain",
        ".plain.tmp",
        ".target.1.tmp",
        ".source-one.json.9.not-a-sequence.tmp",
        ".source-one.json.not-a-process.1.tmp",
        ".source-one.txt.9.1.tmp",
    ] {
        let malformed = initialize(name);
        let path = malformed.0.join(".research-run/sources").join(name);
        fs::write(&path, b"x").expect("write ignored temporary shape");
        assert_fails(
            &run(&malformed.0, &["recover", "--json"], None),
            "noncanonical temporary name",
        );
        assert!(path.exists(), "noncanonical temporary file was consumed");
    }
}

#[cfg(unix)]
fn exercise_direct_workspace_symlink() {
    {
        use std::os::unix::fs::symlink;
        let root = TempDir::new("direct-root-symlink");
        let outside = TempDir::new("direct-root-symlink-target");
        let link = root.0.join("workspace-link");
        symlink(&outside.0, &link).expect("workspace root symlink");
        assert_fails(
            &run(
                &root.0,
                &[
                    "init",
                    link.to_str().expect("UTF-8 link"),
                    "--name",
                    "Unsafe",
                ],
                None,
            ),
            "direct workspace root symlink",
        );
    }
}

#[cfg(not(unix))]
fn exercise_direct_workspace_symlink() {}

#[test]
fn installed_cli_rejects_corrupt_manifest_and_record_shapes() {
    let malformed_manifest = initialize("malformed-manifest");
    fs::write(
        malformed_manifest.0.join(".research-run/manifest.json"),
        b"not-json",
    )
    .expect("replace manifest");
    assert_fails(
        &run(&malformed_manifest.0, &["status", "--json"], None),
        "malformed manifest",
    );

    let invalid_manifest = initialize("invalid-manifest");
    fs::write(
        invalid_manifest.0.join(".research-run/manifest.json"),
        br#"{"schema_version":1,"kind":"project","project_id":"bad id","name":"Name"}"#,
    )
    .expect("replace manifest");
    assert_fails(
        &run(&invalid_manifest.0, &["status", "--json"], None),
        "invalid manifest",
    );

    for directory in ["sources", "claims", "evidence", "experiments", "reviews"] {
        let root = initialize(directory);
        fs::write(
            root.0
                .join(".research-run")
                .join(directory)
                .join("bad.json"),
            b"not-json",
        )
        .expect("write malformed record");
        assert_fails(
            &run(&root.0, &["status", "--json"], None),
            "malformed typed record",
        );
    }

    let unexpected = initialize("unexpected-entry");
    fs::write(
        unexpected.0.join(".research-run/sources/README.txt"),
        b"unexpected",
    )
    .expect("write unexpected entry");
    assert_fails(
        &run(&unexpected.0, &["status", "--json"], None),
        "unexpected record entry",
    );
}
