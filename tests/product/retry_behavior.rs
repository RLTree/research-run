use std::fs;

use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn identical_retry_is_idempotent_and_conflicting_identity_fails_closed() {
    let temporary = TempDir::new("retry");
    let project = temporary.0.join("project");
    for _ in 0..2 {
        crate::review_test_signing::initialize_with_test_authority(
            &temporary.0,
            &project,
            "Retry test",
        );
    }
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
        "Researcher",
        "--authorship",
        "human",
    ];
    succeeds(&project, &claim);
    succeeds(&project, &claim);
    crate::review_test_signing::add_signed_review(
        &project,
        "review-one",
        "claim-one",
        "limited",
        "Bounded assessment",
        "Researcher",
    );
    crate::review_test_signing::add_signed_review(
        &project,
        "review-one",
        "claim-one",
        "limited",
        "Bounded assessment",
        "Researcher",
    );
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
