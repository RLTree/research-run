use std::fs;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::handoff_journey::create_handoff_fixture;
use super::researcher_journey::{TempDir, succeeds};

#[test]
fn legacy_v1_handoff_without_activation_protocol_remains_inspectable() {
    let temporary = TempDir::new("handoff-v1-compatibility");
    let current_path = create_handoff_fixture(&temporary);
    let legacy_path = temporary.0.join("handoff-v1.json");
    let mut legacy: Value =
        serde_json::from_slice(&fs::read(current_path).expect("read current handoff")).unwrap();
    legacy["schema_version"] = json!(1);
    legacy["context"]
        .as_object_mut()
        .expect("context object")
        .remove("contribution_protocol");
    legacy
        .as_object_mut()
        .expect("handoff object")
        .remove("validation");
    fs::write(&legacy_path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

    let fresh = temporary.0.join("legacy-agent");
    fs::create_dir(&fresh).expect("legacy agent directory");
    let inspected = succeeds(
        &fresh,
        &[
            "handoff",
            "inspect",
            "--input",
            legacy_path.to_str().unwrap(),
        ],
    );
    let inspected: Value = serde_json::from_slice(&inspected.stdout).expect("legacy handoff JSON");
    assert_eq!(inspected["schema_version"], 1);
    assert!(inspected["context"].get("contribution_protocol").is_none());
    let human = succeeds(
        &fresh,
        &[
            "handoff",
            "inspect",
            "--input",
            legacy_path.to_str().unwrap(),
            "--human",
        ],
    );
    let human = String::from_utf8_lossy(&human.stdout);
    assert!(human.contains("Validation receipt"));
    assert!(human.contains(
        "Not embedded in this legacy v1 handoff; workspace validity and agent integration are unverified"
    ));
    assert!(human.contains("Contribution protocol"));
    assert!(human.contains("Not embedded in this legacy v1 handoff; migrate the workspace"));
}

#[test]
fn human_inspection_preserves_failed_validation_and_unavailable_integration() {
    let temporary = TempDir::new("handoff-human-failed-validation");
    let fixture = create_handoff_fixture(&temporary);
    let path = temporary.0.join("handoff-failed-validation.json");
    let mut handoff: research_run::workspace::HandoffBundle =
        serde_json::from_slice(&fs::read(fixture).expect("read handoff fixture")).unwrap();
    let result = &mut handoff.validation.as_mut().unwrap().result;
    let hostile_error = "ledger invalid\u{1b}]52;hidden\u{7}";
    let hostile_diagnostic = "instruction inspection unavailable\u{1b}]52;hidden\u{7}";
    result.valid = false;
    result.errors = vec![hostile_error.to_owned()];
    result.agent_integration = research_run::workspace::AgentIntegrationStatus {
        protocol_installed: true,
        instruction_contract_installed: false,
        agent_integration_ready: false,
        ready_scope: "unavailable".to_owned(),
        instruction_path: "unavailable".to_owned(),
        current_session_loaded: "unverified".to_owned(),
        fresh_session_required: false,
        diagnostic: hostile_diagnostic.to_owned(),
    };
    handoff.validation.as_mut().unwrap().result_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(result).expect("validation result JSON"))
    );
    fs::write(&path, serde_json::to_vec_pretty(&handoff).unwrap()).unwrap();

    let inspected = succeeds(
        &temporary.0,
        &[
            "handoff",
            "inspect",
            "--input",
            path.to_str().unwrap(),
            "--human",
        ],
    );
    let text = String::from_utf8_lossy(&inspected.stdout);
    assert!(text.contains("Workspace valid: false"));
    assert!(text.contains("Validation errors:"));
    assert!(text.contains("ledger invalid\\u{1b}]52;hidden\\u{7}"));
    assert!(text.contains("Agent integration: NOT READY (scope: unavailable)"));
    assert!(text.contains("Current session load: unverified; fresh session required: false"));
    assert!(text.contains("instruction inspection unavailable\\u{1b}]52;hidden\\u{7}"));
    assert!(!inspected.stdout.contains(&0x1b));
    assert!(!inspected.stdout.contains(&0x07));
}
