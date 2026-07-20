use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::researcher_journey::{TempDir, cli, succeeds};
use crate::review_test_signing::{enroll_test_authority, prepare_signed_review};

#[test]
fn only_an_exact_signature_from_the_enrolled_authority_promotes() {
    let (temporary, project) = signed_project("review-authorization", "Signed review", "ai");
    assert_unsigned_review_is_rejected(&project);
    enroll_test_authority(&project);
    assert_modified_requests_are_rejected(&project);
    crate::review_test_signing::add_signed_review(
        &project,
        "review-one",
        "claim-one",
        "limited",
        "Human decision",
        "Researcher",
    );
    assert_eq!(assessment(&project), "limited");
    drop(temporary);
}

#[test]
fn a_signed_request_cannot_replay_into_a_same_name_workspace() {
    let temporary = TempDir::new("review-replay");
    let first = initialize_project(&temporary, "first", "Same project", "human");
    let second = initialize_project(&temporary, "second", "Same project", "human");
    for project in [&first, &second] {
        enroll_test_authority(project);
    }
    let signed = prepare_signed_review(
        &first,
        "review-one",
        "claim-one",
        "limited",
        "Project-bound decision",
        "Researcher",
    );
    let replay = cli(
        &second,
        &[
            "review",
            "add",
            "--request",
            &signed.request.to_string_lossy(),
            "--signature",
            &signed.signature.to_string_lossy(),
        ],
    );
    assert!(!replay.status.success());
    assert_eq!(assessment(&second), "unreviewed");
}

#[test]
fn post_initialization_caller_cannot_enroll_a_self_selected_authority() {
    let temporary = TempDir::new("review-bootstrap");
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &["init", &project.to_string_lossy(), "--name", "No authority"],
    );
    let rejected = cli(
        &project,
        &[
            "review",
            "authority",
            "add",
            "--id",
            "caller-key",
            "--public-key",
            "caller.pub",
        ],
    );
    assert!(!rejected.status.success());
    assert!(
        !project
            .join(".research-run/review-authorities/caller-key.json")
            .exists()
    );
}

fn signed_project(label: &str, name: &str, authorship: &str) -> (TempDir, PathBuf) {
    let temporary = TempDir::new(label);
    let project = initialize_project(&temporary, "project", name, authorship);
    (temporary, project)
}

fn initialize_project(
    temporary: &TempDir,
    directory: &str,
    name: &str,
    authorship: &str,
) -> PathBuf {
    let project = temporary.0.join(directory);
    crate::review_test_signing::initialize_with_test_authority(&temporary.0, &project, name);
    succeeds(
        &project,
        &[
            "claim",
            "add",
            "--id",
            "claim-one",
            "--text",
            "Synthetic claim",
            "--scope",
            "Synthetic only",
            "--owner",
            "Researcher",
            "--authorship",
            authorship,
        ],
    );
    project
}

fn assert_unsigned_review_is_rejected(project: &Path) {
    let unsigned = cli(
        project,
        &[
            "review",
            "add",
            "--id",
            "forged-review",
            "--claim",
            "claim-one",
            "--decision",
            "supported",
            "--rationale",
            "Automated caller chose this",
            "--reviewer",
            "Definitely a human",
        ],
    );
    assert!(!unsigned.status.success());
    assert!(
        !project
            .join(".research-run/reviews/forged-review.json")
            .exists()
    );
    assert_eq!(assessment(project), "unreviewed");
}

fn assert_modified_requests_are_rejected(project: &Path) {
    assert_modified_request_is_rejected(project, "review-rationale", |request| {
        request["rationale"] = Value::String("Agent changed the decision".to_owned());
    });
    assert_modified_request_is_rejected(project, "review-workspace", |request| {
        request["workspace_id"] = Value::String("0".repeat(64));
    });
}

fn assert_modified_request_is_rejected(project: &Path, id: &str, modify: impl FnOnce(&mut Value)) {
    let signed = prepare_signed_review(
        project,
        id,
        "claim-one",
        "limited",
        "Human decision",
        "Researcher",
    );
    let mut request: Value =
        serde_json::from_slice(&std::fs::read(&signed.request).expect("request")).expect("JSON");
    modify(&mut request);
    std::fs::write(
        &signed.request,
        serde_json::to_vec_pretty(&request).expect("modified request"),
    )
    .expect("write modified request");
    let modified = cli(
        project,
        &[
            "review",
            "add",
            "--request",
            &signed.request.to_string_lossy(),
            "--signature",
            &signed.signature.to_string_lossy(),
        ],
    );
    assert!(!modified.status.success());
    assert!(
        !project
            .join(format!(".research-run/reviews/{id}.json"))
            .exists()
    );
    assert_eq!(assessment(project), "unreviewed");
}

fn assessment(project: &Path) -> String {
    let status: Value = serde_json::from_slice(&succeeds(project, &["status", "--json"]).stdout)
        .expect("status JSON");
    status["claims"][0]["assessment"]
        .as_str()
        .expect("assessment")
        .to_owned()
}
