use super::*;

#[test]
fn installed_process_exercises_directory_race_and_current_directory_faults() {
    let root = TempDir::new("directory-race");
    let nested = root.0.join("nested/workspace");
    let output = run(
        &root.0,
        &[
            "init",
            nested.to_str().expect("UTF-8 fixture"),
            "--name",
            "Directory race",
            "--without-review-authority",
        ],
        Some("directory already exists"),
    );
    assert!(output.status.success());
    assert!(nested.join(".research-run/manifest.json").is_file());

    let output = run(&root.0, &["status"], Some("current directory"));
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("current directory"));

    let boundary = initialize("workspace-boundary-fault");
    assert!(
        !run(
            &boundary.0,
            &["status", "--json"],
            Some("inspect workspace boundary")
        )
        .status
        .success()
    );

    for fault in [
        "inspect workspace root",
        "canonicalize workspace root",
        "inspect workspace ancestor",
        "canonicalize workspace ancestor",
    ] {
        let root = TempDir::new(fault);
        let target = if fault.contains("ancestor") {
            root.0.join("missing/workspace")
        } else {
            root.0.clone()
        };
        let output = run(
            &root.0,
            &[
                "init",
                target.to_str().expect("UTF-8 fixture"),
                "--name",
                "Fault",
                "--without-review-authority",
            ],
            Some(fault),
        );
        assert!(!output.status.success(), "fault {fault} was ignored");
    }

    let root = TempDir::new("storage-current-directory");
    let output = run(
        &root.0,
        &[
            "init",
            "relative",
            "--name",
            "Fault",
            "--without-review-authority",
        ],
        Some("read current directory"),
    );
    assert!(!output.status.success());
}

#[test]
fn every_installed_current_workspace_command_propagates_discovery_failure() {
    let root = TempDir::new("command-discovery-faults");
    for args in current_workspace_commands() {
        let output = run(&root.0, &args, Some("current directory"));
        assert!(
            !output.status.success(),
            "command {args:?} ignored discovery failure"
        );
    }
}

fn current_workspace_commands() -> Vec<Vec<&'static str>> {
    let mut commands = source_claim_and_evidence_commands();
    commands.extend(experiment_review_and_read_commands());
    commands
}

fn source_claim_and_evidence_commands() -> Vec<Vec<&'static str>> {
    vec![
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
    ]
}

fn experiment_review_and_read_commands() -> Vec<Vec<&'static str>> {
    vec![
        vec![
            "experiment",
            "add",
            "--id",
            "experiment-one",
            "--question",
            "Question?",
            "--method-ref",
            "method",
            "--observation",
            "Observed",
            "--interpretation",
            "Interpretation",
            "--limitation",
            "Limited",
            "--outcome",
            "positive",
            "--next-move",
            "Repeat",
        ],
        vec![
            "review",
            "prepare",
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
        vec!["validate", "--json"],
        vec!["status", "--json"],
        vec!["list"],
        vec!["show", "--kind", "knowledge", "--id", "knowledge-one"],
        vec!["search", "query"],
        vec!["recent"],
        vec!["timeline"],
        vec!["related", "--kind", "knowledge", "--id", "knowledge-one"],
        vec!["unresolved"],
        vec!["blockers"],
        vec!["next"],
        vec!["context"],
        vec![
            "handoff",
            "create",
            "--id",
            "handoff-one",
            "--generated-at",
            "2026-07-18T20:00:00Z",
        ],
    ]
}

#[cfg(unix)]
#[test]
fn installed_process_rejects_symlink_ancestors_and_non_utf8_entries() {
    use std::os::unix::fs::symlink;

    let root = TempDir::new("path-shapes");
    let outside = TempDir::new("path-shapes-outside");
    let link = root.0.join("linked");
    symlink(&outside.0, &link).expect("symlink ancestor");
    let output = run(
        &root.0,
        &[
            "init",
            link.join("missing").to_str().expect("UTF-8 fixture"),
            "--name",
            "Unsafe",
            "--without-review-authority",
        ],
        None,
    );
    assert!(!output.status.success());

    #[cfg(not(target_os = "macos"))]
    {
        use std::os::unix::ffi::OsStringExt;
        let workspace = initialize("non-utf8-entry");
        let name = std::ffi::OsString::from_vec(vec![b'b', b'a', b'd', 0xff]);
        fs::write(workspace.0.join(".research-run/sources").join(name), b"x")
            .expect("non-UTF-8 entry");
        let output = run(&workspace.0, &["status", "--json"], None);
        assert!(!output.status.success());
    }
}
