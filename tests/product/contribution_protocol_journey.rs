use std::fs;

use serde_json::{Value, json};

use crate::review_test_signing::initialize_with_test_authority;

use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn activated_project_confronts_agent_with_retrieve_capture_validate_contract() {
    let temporary = TempDir::new("contribution-protocol");
    let project = temporary.0.join("project");
    initialize_unanchored(&temporary, &project, "Contribution protocol");
    let protocol = read_protocol(&project);
    assert_protocol_contract(&protocol);
    assert_context_is_read_only(&project, &protocol);
    capture_observation(&temporary, &project);
    assert_handoff_ready(&project, &protocol);
}

fn assert_protocol_contract(protocol: &Value) {
    assert_eq!(protocol["schema_version"], 1);
    assert_eq!(protocol["kind"], "contribution-protocol");
    assert_eq!(
        protocol["sequence"],
        json!([
            "retrieve-bounded-context",
            "classify-new-material",
            "append-typed-records",
            "validate-workspace",
            "answer-or-handoff"
        ])
    );
    assert!(
        protocol["triggers"]
            .as_array()
            .unwrap()
            .contains(&json!("human-observation"))
    );
    assert_eq!(protocol["human_input_authorship"], "human");
    assert_eq!(protocol["no_new_material_effect"], "no-write");
    assert_eq!(protocol["claim_promotion"], "signed-human-review-only");
}

fn assert_context_is_read_only(project: &std::path::Path, protocol: &Value) {
    let context = json_output(project, &["context", "--query", "transformation"]);
    assert_eq!(context["contribution_protocol"], *protocol);
    let human = succeeds(
        project,
        &["context", "--query", "transformation", "--human"],
    );
    let human = String::from_utf8_lossy(&human.stdout);
    for required in [
        "human-observation",
        "human-correction",
        "negative-result",
        "human input authorship: human",
        "identical retry: no-op",
        "claim promotion: signed-human-review-only",
    ] {
        assert!(
            human.contains(required),
            "missing human protocol term: {required}"
        );
    }
    assert_eq!(
        fs::read_dir(project.join(".research-run/knowledge"))
            .unwrap()
            .count(),
        0,
        "retrieval with no new material must not write"
    );
}

fn capture_observation(temporary: &TempDir, project: &std::path::Path) {
    let observation = json!({
        "schema_version": 1,
        "kind": "knowledge",
        "id": "observation-transformation",
        "record_type": "observation",
        "title": "Transformation did not produce growth",
        "body": "The researcher reported no growth in the repeated transformation.",
        "occurred_at": "2026-07-22T17:00:00Z",
        "state": "open",
        "authorship": "human"
    });
    let input = temporary.0.join("observation.json");
    fs::write(&input, serde_json::to_vec_pretty(&observation).unwrap()).expect("input");
    succeeds(
        project,
        &["knowledge", "add", "--input", input.to_str().unwrap()],
    );
    let repeated = succeeds(
        project,
        &["knowledge", "add", "--input", input.to_str().unwrap()],
    );
    assert!(String::from_utf8_lossy(&repeated.stdout).contains("Already present"));
    let validation = json_output(project, &["validate", "--json"]);
    assert_eq!(validation["valid"], true);
}

fn assert_handoff_ready(project: &std::path::Path, protocol: &Value) {
    let handoff = json_output(
        project,
        &[
            "handoff",
            "create",
            "--id",
            "handoff-ready",
            "--generated-at",
            "2026-07-22T17:05:00Z",
            "--query",
            "transformation",
        ],
    );
    assert_eq!(handoff["schema_version"], 2);
    assert_eq!(handoff["context"]["contribution_protocol"], *protocol);
    assert_eq!(
        handoff["context"]["matches"][0]["id"],
        "observation-transformation"
    );
}

#[test]
fn activation_policy_fails_closed_and_cannot_promote_agent_claims() {
    let temporary = TempDir::new("contribution-protocol-defense");
    let unanchored = temporary.0.join("unanchored");
    initialize_unanchored(&temporary, &unanchored, "Protocol defense");
    assert_agent_claim_cannot_promote(&unanchored);
    assert_malformed_and_missing_policy_fail(&unanchored);

    let anchored = temporary.0.join("anchored");
    initialize_with_test_authority(&temporary.0, &anchored, "Anchored protocol defense");
    assert_agent_claim_cannot_promote(&anchored);
}

fn assert_agent_claim_cannot_promote(project: &std::path::Path) {
    succeeds(
        project,
        &[
            "claim",
            "add",
            "--id",
            "claim-agent",
            "--text",
            "Agent-authored candidate claim",
            "--scope",
            "Activation defense fixture",
            "--owner",
            "Research agent",
            "--authorship",
            "ai",
        ],
    );
    let status = json_output(project, &["status", "--json"]);
    assert_eq!(status["claims"][0]["assessment"], "unreviewed");
    assert!(
        !cli(
            project,
            &[
                "review",
                "prepare",
                "--id",
                "review-forged",
                "--claim",
                "claim-agent",
                "--decision",
                "supported",
                "--rationale",
                "An agent cannot authorize this promotion.",
                "--reviewer",
                "Research agent",
            ],
        )
        .status
        .success()
    );
}

fn assert_malformed_and_missing_policy_fail(project: &std::path::Path) {
    let protocol = project.join(".research-run/contribution-protocols/agent-contribution.json");
    let canonical: Value = serde_json::from_slice(&fs::read(&protocol).unwrap()).unwrap();
    let mut malformed = canonical.clone();
    malformed["schema_version"] = json!(2);
    fs::write(&protocol, serde_json::to_vec_pretty(&malformed).unwrap()).unwrap();
    let invalid = cli(project, &["validate", "--json"]);
    assert!(!invalid.status.success());
    let validation: Value = serde_json::from_slice(&invalid.stdout).expect("validation JSON");
    assert_eq!(validation["valid"], false);
    assert!(
        validation["errors"][0]
            .as_str()
            .unwrap()
            .contains("schema_version")
    );

    let mut modified = canonical.clone();
    modified["claim_promotion"] = json!("agent-may-promote");
    fs::write(&protocol, serde_json::to_vec_pretty(&modified).unwrap()).unwrap();
    let invalid = cli(project, &["validate", "--json"]);
    assert!(!invalid.status.success());
    let validation: Value = serde_json::from_slice(&invalid.stdout).expect("validation JSON");
    assert!(
        validation["errors"][0]
            .as_str()
            .unwrap()
            .contains("modified activation contract")
    );

    let mut invalid_id = canonical;
    invalid_id["id"] = json!("INVALID");
    fs::write(&protocol, serde_json::to_vec_pretty(&invalid_id).unwrap()).unwrap();
    let invalid = cli(project, &["validate", "--json"]);
    assert!(!invalid.status.success());

    fs::remove_file(&protocol).unwrap();
    let missing = cli(project, &["validate", "--json"]);
    assert!(!missing.status.success());
    let validation: Value = serde_json::from_slice(&missing.stdout).expect("validation JSON");
    assert!(
        validation["errors"][0]
            .as_str()
            .unwrap()
            .contains("activation")
    );
}

fn initialize_unanchored(temporary: &TempDir, project: &std::path::Path, name: &str) {
    succeeds(
        &temporary.0,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            name,
            "--without-review-authority",
        ],
    );
}

fn read_protocol(project: &std::path::Path) -> Value {
    let path = project.join(".research-run/contribution-protocols/agent-contribution.json");
    serde_json::from_slice(&fs::read(path).expect("protocol")).expect("protocol JSON")
}

fn json_output(project: &std::path::Path, args: &[&str]) -> Value {
    serde_json::from_slice(&succeeds(project, args).stdout).expect("command JSON")
}
