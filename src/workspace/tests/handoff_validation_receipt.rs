use std::fs;

use crate::Error;
use crate::domain::digest;

use super::{Workspace, temporary};

#[test]
fn handoff_validation_receipt_binds_result_context_and_project() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Receipt binding").unwrap();
    let bundle = workspace
        .handoff("handoff-receipt", "2026-08-20T20:00:00Z", None, 10)
        .unwrap();
    assert!(bundle.validate().is_ok());

    let mut changed_context = bundle.clone();
    changed_context.context.scope = "changed scope".to_owned();
    assert!(changed_context.validate().is_err());

    let mut changed_result = bundle.clone();
    changed_result
        .validation
        .as_mut()
        .unwrap()
        .result
        .counts
        .insert("knowledge".to_owned(), 1);
    assert!(changed_result.validate().is_err());

    let other_root = temporary();
    let other = Workspace::initialize(&other_root, "Other receipt workspace").unwrap();
    let mut cross_workspace = bundle;
    let other_bundle = other
        .handoff("handoff-other", "2026-08-20T20:01:00Z", None, 10)
        .unwrap();
    cross_workspace.context.project_id = other_bundle.context.project_id.clone();
    cross_workspace.validation = other_bundle.validation;
    assert!(cross_workspace.validate().is_err());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(other_root).unwrap();
}

#[test]
fn legacy_handoff_rejects_an_unverified_validation_receipt() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Legacy receipt rejection").unwrap();
    let mut bundle = workspace
        .handoff("handoff-legacy-receipt", "2026-08-20T20:00:30Z", None, 10)
        .unwrap();
    bundle.schema_version = 1;

    assert!(bundle.validate().is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn handoff_receipt_rejects_invalid_error_text() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Receipt error text").unwrap();

    for invalid in [String::new(), "x".repeat(65_537), "é".repeat(65_537)] {
        let mut bundle = workspace
            .handoff("handoff-error-text", "2026-08-20T20:00:40Z", None, 10)
            .unwrap();
        let receipt = bundle.validation.as_mut().unwrap();
        receipt.result.valid = false;
        receipt.result.errors = vec![invalid];
        receipt.result_sha256 = digest(&serde_json::to_vec(&receipt.result).unwrap());
        assert!(bundle.validate().is_err());
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn handoff_receipt_diagnostic_uses_the_validation_schema_character_boundaries() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Receipt diagnostic schema parity").unwrap();
    let baseline = workspace
        .handoff(
            "handoff-diagnostic-schema-text",
            "2026-08-20T20:00:45Z",
            None,
            10,
        )
        .unwrap();
    for (name, text, expected) in validation_schema_text_cases() {
        let mut bundle = baseline.clone();
        let receipt = bundle.validation.as_mut().unwrap();
        receipt.result.agent_integration.diagnostic = text;
        receipt.result_sha256 = digest(&serde_json::to_vec(&receipt.result).unwrap());
        assert_eq!(bundle.validate().is_ok(), expected, "diagnostic: {name}");
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn handoff_receipt_errors_use_the_validation_schema_character_boundaries() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Receipt error schema parity").unwrap();
    let baseline = workspace
        .handoff(
            "handoff-error-schema-text",
            "2026-08-20T20:00:46Z",
            None,
            10,
        )
        .unwrap();
    for (name, text, expected) in validation_schema_text_cases() {
        let mut bundle = baseline.clone();
        let receipt = bundle.validation.as_mut().unwrap();
        receipt.result.valid = false;
        receipt.result.errors = vec![text];
        receipt.result_sha256 = digest(&serde_json::to_vec(&receipt.result).unwrap());
        assert_eq!(bundle.validate().is_ok(), expected, "error: {name}");
    }
    fs::remove_dir_all(root).unwrap();
}

fn validation_schema_text_cases() -> [(&'static str, String, bool); 9] {
    [
        ("empty", String::new(), false),
        ("whitespace", " \n".to_owned(), true),
        ("control", "\0".to_owned(), true),
        ("ascii maximum", "x".repeat(65_536), true),
        ("ascii overflow", "x".repeat(65_537), false),
        ("multibyte maximum", "é".repeat(65_536), true),
        ("multibyte overflow", "é".repeat(65_537), false),
        ("supplementary maximum", "🛡".repeat(65_536), true),
        ("supplementary overflow", "🛡".repeat(65_537), false),
    ]
}

#[test]
fn handoff_validation_rejects_output_larger_than_the_inspection_budget() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Handoff output budget").unwrap();
    let mut bundle = workspace
        .handoff("handoff-output-budget", "2026-08-20T20:00:50Z", None, 10)
        .unwrap();
    let receipt = bundle.validation.as_mut().unwrap();
    receipt.result.valid = false;
    receipt.result.errors = vec!["x".repeat(4_096); 256];
    receipt.result_sha256 = digest(&serde_json::to_vec(&receipt.result).unwrap());

    assert!(matches!(bundle.validate(), Err(Error::Budget(_))));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn handoff_v2_rejects_empty_workspace_identity_even_with_recomputed_receipt() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Empty workspace receipt").unwrap();
    let mut bundle = workspace
        .handoff("handoff-empty-workspace", "2026-08-20T20:02:00Z", None, 10)
        .unwrap();

    bundle.context.workspace_id = Some(String::new());
    let validation = bundle.validation.as_mut().unwrap();
    validation.result.authority.as_mut().unwrap().workspace_id = String::new();
    validation.result_sha256 = digest(&serde_json::to_vec(&validation.result).unwrap());
    validation.context_sha256 = digest(&serde_json::to_vec(&bundle.context).unwrap());

    let error = bundle.validate().unwrap_err().to_string();
    assert!(error.contains("non-empty immutable workspace identity"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn handoff_rejects_new_agent_status_without_installed_protocol() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Missing protocol status").unwrap();
    let mut bundle = workspace
        .handoff("handoff-missing-protocol", "2026-08-20T20:03:00Z", None, 10)
        .unwrap();

    let validation = bundle.validation.as_mut().unwrap();
    let status = &mut validation.result.agent_integration;
    status.protocol_installed = false;
    status.instruction_contract_installed = false;
    status.agent_integration_ready = false;
    status.ready_scope = "new-agent-run".to_owned();
    status.instruction_path = "AGENTS.md".to_owned();
    status.fresh_session_required = false;
    validation
        .result
        .authority
        .as_mut()
        .unwrap()
        .instruction_sha256 = None;
    validation.result_sha256 = digest(&serde_json::to_vec(&validation.result).unwrap());

    assert!(bundle.validate().is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn handoff_deserialization_rejects_unknown_nested_validation_fields() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Strict validation receipt").unwrap();
    let bundle = workspace
        .handoff("handoff-strict-receipt", "2026-08-20T20:04:00Z", None, 10)
        .unwrap();
    let serialized = serde_json::to_value(bundle).unwrap();

    for nested_object in ["result", "authority", "agent_integration"] {
        let mut tampered = serialized.clone();
        let result = tampered["validation"]["result"]
            .as_object_mut()
            .expect("validation result object");
        let object = match nested_object {
            "result" => result,
            "authority" => result["authority"]
                .as_object_mut()
                .expect("validation authority object"),
            "agent_integration" => result["agent_integration"]
                .as_object_mut()
                .expect("agent integration object"),
            _ => unreachable!(),
        };
        object.insert("unexpected_field".to_owned(), serde_json::Value::Bool(true));

        let error = serde_json::from_value::<crate::workspace::HandoffBundle>(tampered)
            .unwrap_err()
            .to_string();
        assert!(error.contains("unknown field"), "{nested_object}: {error}");
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn handoff_rejects_mismatched_protocol_authority_with_recomputed_result_digest() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Protocol authority binding").unwrap();
    let mut bundle = workspace
        .handoff("handoff-protocol-binding", "2026-08-20T20:05:00Z", None, 10)
        .unwrap();

    let validation = bundle.validation.as_mut().unwrap();
    validation
        .result
        .authority
        .as_mut()
        .unwrap()
        .protocol_sha256 = "0".repeat(64);
    validation.result_sha256 = digest(&serde_json::to_vec(&validation.result).unwrap());

    assert!(bundle.validate().is_err());
    fs::remove_dir_all(root).unwrap();
}
