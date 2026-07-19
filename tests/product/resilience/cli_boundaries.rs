use super::*;

#[path = "cli_boundaries/publication_failures.rs"]
mod publication_failures;
use research_run::domain::{
    ArtifactLocatorType, ArtifactPointer, ExperimentReceipt, FORMAT_VERSION, Outcome,
    SourceProvenance, SourceRecord,
};
use research_run::workspace::Workspace;

fn assert_fails(output: &Output, context: &str) {
    assert!(
        !output.status.success(),
        "{context} unexpectedly succeeded\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn installed_cli_covers_empty_projection_artifacts_and_command_effect_failures() {
    let root = initialize("empty-human-status");
    assert_empty_human_status(&root.0);
    exercise_artifact_argument_boundaries(&root.0);
    exercise_recovery_and_conflicting_claim(&root.0);
}

fn assert_empty_human_status(root: &Path) {
    let output = run(root, &["status"], None);
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Claims\n- None"));
    assert!(text.contains("Experiments\n- None"));
}

fn exercise_artifact_argument_boundaries(root: &Path) {
    let valid = run(
        root,
        &[
            "experiment",
            "add",
            "--id",
            "experiment-artifact",
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
            "inconclusive",
            "--next-move",
            "Repeat",
            "--artifact",
            "external:https://example.invalid/result:Public result",
        ],
        None,
    );
    assert!(valid.status.success());
    let stored: ExperimentReceipt = serde_json::from_slice(
        &fs::read(root.join(".research-run/experiments/experiment-artifact.json"))
            .expect("read stored experiment"),
    )
    .expect("stored experiment JSON");
    assert_eq!(
        stored.artifacts[0].locator,
        "https://example.invalid/result"
    );
    assert_eq!(stored.artifacts[0].description, "Public result");
    for artifact in [
        "missing-separators",
        "external:missing-description",
        "unknown:locator:description",
        "external::description",
        "external:locator:",
    ] {
        let output = run(
            root,
            &[
                "experiment",
                "add",
                "--id",
                "experiment-invalid",
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
                "--artifact",
                artifact,
            ],
            None,
        );
        assert_fails(&output, artifact);
    }
}

fn exercise_recovery_and_conflicting_claim(root: &Path) {
    let missing = TempDir::new("recover-missing");
    assert_fails(
        &run(&missing.0, &["recover", "missing", "--json"], None),
        "recover missing workspace",
    );

    let claim = [
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
    ];
    assert!(run(root, &claim, None).status.success());
    let mut conflicting = claim;
    conflicting[5] = "Changed claim";
    assert_fails(&run(root, &conflicting, None), "conflicting claim");
}
