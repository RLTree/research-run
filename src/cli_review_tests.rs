use super::*;
use crate::domain::ReviewRequest;

#[test]
fn review_add_propagates_signature_input_and_post_input_discovery_failures() {
    let root = empty_directory();
    let request_path = root.join("request.json");
    let signature_path = root.join("request.json.sig");
    let request = ReviewRequest {
        schema_version: 1,
        kind: "review-request".to_owned(),
        project_id: "project".to_owned(),
        workspace_id: "a".repeat(64),
        id: "review-one".to_owned(),
        claim_id: "claim-one".to_owned(),
        evidence_ids: Vec::new(),
        subject_sha256: "b".repeat(64),
        decision: Assessment::Limited,
        rationale: "Rationale".to_owned(),
        reviewer: "Reviewer".to_owned(),
    };
    std::fs::write(
        &request_path,
        serde_json::to_vec_pretty(&request).expect("request"),
    )
    .expect("request file");
    let command = || Command::Review {
        command: ReviewCommand::Add {
            request: request_path.to_string_lossy().into_owned(),
            signature: signature_path.to_string_lossy().into_owned(),
        },
    };
    assert!(execute(Cli { command: command() }).is_err());
    std::fs::write(&signature_path, b"signature").expect("signature");
    inject_current_directory_failure();
    assert!(execute(Cli { command: command() }).is_err());
    std::fs::remove_dir_all(root).expect("remove fixture");
}
