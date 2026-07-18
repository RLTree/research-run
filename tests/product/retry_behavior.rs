use std::fs;

use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn identical_retry_is_idempotent_and_conflicting_identity_fails_closed() {
    let temporary = TempDir::new("retry");
    let project = temporary.0.join("project");
    let project_text = project.to_string_lossy();
    succeeds(
        &temporary.0,
        &["init", &project_text, "--name", "Retry test"],
    );
    let first = [
        "source",
        "add",
        "--id",
        "source-one",
        "--citation",
        "Citation",
        "--locator",
        "doi:10.0000/example",
        "--provenance",
        "human",
    ];
    succeeds(&project, &first);
    succeeds(&project, &first);
    let conflict = cli(
        &project,
        &[
            "source",
            "add",
            "--id",
            "source-one",
            "--citation",
            "Changed citation",
            "--locator",
            "doi:10.0000/example",
            "--provenance",
            "human",
        ],
    );
    assert!(!conflict.status.success());
    assert_eq!(
        fs::read_dir(project.join(".research-run/sources"))
            .expect("sources")
            .count(),
        1
    );
}
