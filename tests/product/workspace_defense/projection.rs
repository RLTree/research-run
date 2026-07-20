use super::*;

#[test]
fn status_projection_is_byte_deterministic() {
    let project = workspace("determinism");
    succeeds(
        &project.0,
        &[
            "claim",
            "add",
            "--id",
            "claim-one",
            "--text",
            "Synthetic claim.",
            "--scope",
            "Synthetic only",
            "--owner",
            "Example Researcher",
            "--authorship",
            "human",
        ],
    );
    let first = succeeds(&project.0, &["status", "--json"]).stdout;
    let second = succeeds(&project.0, &["status", "--json"]).stdout;
    assert_eq!(first, second);
}

#[test]
fn human_output_neutralizes_record_and_filename_controls() {
    let temporary = TempDir::new("terminal-controls");
    let hostile = "visible\u{1b}]2;changed\u{7}";
    succeeds(
        &temporary.0,
        &["init", ".", "--name", hostile, "--without-review-authority"],
    );
    succeeds(
        &temporary.0,
        &[
            "claim",
            "add",
            "--id",
            "claim-safe",
            "--text",
            hostile,
            "--scope",
            hostile,
            "--owner",
            "Researcher",
            "--authorship",
            "human",
        ],
    );
    succeeds(
        &temporary.0,
        &[
            "experiment",
            "add",
            "--id",
            "experiment-safe",
            "--question",
            "Question?",
            "--method-ref",
            "protocol.md",
            "--observation",
            hostile,
            "--interpretation",
            hostile,
            "--limitation",
            "Bounded",
            "--outcome",
            "ambiguous",
            "--next-move",
            hostile,
        ],
    );
    let status = succeeds(&temporary.0, &["status"]);
    assert!(!status.stdout.contains(&0x1b));
    assert!(!status.stdout.contains(&0x07));
    assert!(String::from_utf8_lossy(&status.stdout).contains("\\u{1b}"));

    let hostile_name = ".research-run/claims/hostile\u{1b}]2;changed\u{7}.txt";
    fs::write(temporary.0.join(hostile_name), "not a record").expect("write hostile filename");
    let diagnostic = cli(&temporary.0, &["validate"]);
    assert!(!diagnostic.status.success());
    assert!(!diagnostic.stdout.contains(&0x1b));
    assert!(!diagnostic.stderr.contains(&0x1b));
    assert!(!diagnostic.stdout.contains(&0x07));
    assert!(!diagnostic.stderr.contains(&0x07));
}
