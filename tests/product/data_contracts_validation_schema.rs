use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use jsonschema::Resource;
use research_run::workspace::AgentIntegrationStatus;
use serde_json::{Value, json};

#[test]
fn validation_v2_schema_rejects_contradictory_readiness_receipt() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema = read_json(&root.join("schemas/v2/validation.schema.json"));
    let types = read_json(&root.join("schemas/v1/types.schema.json"));
    let validator = jsonschema::draft202012::options()
        .with_resource(
            "https://research-run.local/schemas/v1/types.schema.json",
            Resource::from_contents(types).expect("v1 types resource"),
        )
        .build(&schema)
        .expect("validation v2 schema");
    let contradictory =
        read_json(&root.join("fixtures/schemas/red/validation-v2-contradictory-readiness.json"));
    assert!(!validator.is_valid(&contradictory));

    let mut protocol_unavailable_for_new_agent_run = contradictory;
    let status = protocol_unavailable_for_new_agent_run["agent_integration"]
        .as_object_mut()
        .expect("agent integration object");
    status.insert(
        "instruction_contract_installed".to_owned(),
        Value::Bool(false),
    );
    status.insert("fresh_session_required".to_owned(), Value::Bool(false));
    assert!(!validator.is_valid(&protocol_unavailable_for_new_agent_run));

    let mut valid_loaded = protocol_unavailable_for_new_agent_run;
    valid_loaded["agent_integration"]["protocol_installed"] = Value::Bool(true);
    assert!(validator.is_valid(&valid_loaded));

    assert_authority_contract(&validator, &valid_loaded);

    let mut load_failure = valid_loaded.clone();
    load_failure["authority"] = Value::Null;
    load_failure["valid"] = Value::Bool(false);
    load_failure["errors"] = json!(["workspace load failed"]);
    load_failure["counts"] = json!({});
    load_failure["agent_integration"] = unavailable_status();
    assert!(validator.is_valid(&load_failure));

    let mut valid_with_empty_counts = load_failure.clone();
    valid_with_empty_counts["valid"] = Value::Bool(true);
    valid_with_empty_counts["errors"] = json!([]);
    assert!(!validator.is_valid(&valid_with_empty_counts));

    let mut partial_counts = valid_loaded.clone();
    partial_counts["counts"]
        .as_object_mut()
        .expect("counts object")
        .remove("source");
    assert!(!validator.is_valid(&partial_counts));

    let mut extra_counts = valid_loaded.clone();
    extra_counts["counts"]
        .as_object_mut()
        .expect("counts object")
        .insert("unexpected".to_owned(), Value::from(0));
    assert!(!validator.is_valid(&extra_counts));

    let mut bounded_errors = valid_loaded;
    bounded_errors["valid"] = Value::Bool(false);
    bounded_errors["errors"] = Value::Array(
        (0..256)
            .map(|index| Value::String(format!("validation error {index}")))
            .collect(),
    );
    assert!(validator.is_valid(&bounded_errors));
    bounded_errors["errors"]
        .as_array_mut()
        .expect("errors array")
        .push(Value::String("overflow".to_owned()));
    assert!(!validator.is_valid(&bounded_errors));
}

#[test]
fn validation_v2_receipt_text_schema_uses_character_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let schema = read_json(&root.join("schemas/v2/validation.schema.json"));
    let types = read_json(&root.join("schemas/v1/types.schema.json"));
    let validator = jsonschema::draft202012::options()
        .with_resource(
            "https://research-run.local/schemas/v1/types.schema.json",
            Resource::from_contents(types).expect("v1 types resource"),
        )
        .build(&schema)
        .expect("validation v2 schema");
    let mut receipt =
        read_json(&root.join("fixtures/schemas/red/validation-v2-contradictory-readiness.json"));
    receipt["agent_integration"]["protocol_installed"] = Value::Bool(true);
    receipt["agent_integration"]["instruction_contract_installed"] = Value::Bool(false);
    receipt["agent_integration"]["fresh_session_required"] = Value::Bool(false);

    let cases = [
        ("empty", String::new(), false),
        ("whitespace", " \n".to_owned(), true),
        ("control", "\0".to_owned(), true),
        ("ascii maximum", "x".repeat(65_536), true),
        ("ascii overflow", "x".repeat(65_537), false),
        ("multibyte maximum", "é".repeat(65_536), true),
        ("multibyte overflow", "é".repeat(65_537), false),
        ("supplementary maximum", "🛡".repeat(65_536), true),
        ("supplementary overflow", "🛡".repeat(65_537), false),
    ];

    for (name, text, expected) in cases {
        let mut diagnostic_receipt = receipt.clone();
        diagnostic_receipt["agent_integration"]["diagnostic"] = Value::String(text.clone());
        let schema_accepts = validator.is_valid(&diagnostic_receipt);
        let status: AgentIntegrationStatus =
            serde_json::from_value(diagnostic_receipt["agent_integration"].clone())
                .expect("agent integration status");
        assert_eq!(schema_accepts, expected, "diagnostic schema: {name}");
        assert_eq!(status.is_consistent(), schema_accepts, "status: {name}");

        let mut error_receipt = receipt.clone();
        error_receipt["valid"] = Value::Bool(false);
        error_receipt["errors"] = json!([text]);
        assert_eq!(
            validator.is_valid(&error_receipt),
            expected,
            "error schema: {name}"
        );
    }
}

fn assert_authority_contract(validator: &jsonschema::Validator, valid: &Value) {
    let mut explicit_empty_legacy_identity = valid.clone();
    explicit_empty_legacy_identity["authority"]["workspace_id"] = Value::String(String::new());
    assert!(validator.is_valid(&explicit_empty_legacy_identity));

    let mut uppercase_identity = valid.clone();
    uppercase_identity["authority"]["workspace_id"] = Value::String("A".repeat(64));
    assert!(!validator.is_valid(&uppercase_identity));

    let mut ready = valid.clone();
    ready["agent_integration"]["instruction_contract_installed"] = Value::Bool(true);
    ready["agent_integration"]["agent_integration_ready"] = Value::Bool(true);
    ready["agent_integration"]["fresh_session_required"] = Value::Bool(true);
    assert!(validator.is_valid(&ready));

    let mut missing_authority = ready.clone();
    missing_authority["authority"] = Value::Null;
    assert!(!validator.is_valid(&missing_authority));

    ready["authority"]["instruction_sha256"] = Value::Null;
    assert!(!validator.is_valid(&ready));
}

fn unavailable_status() -> Value {
    json!({
        "protocol_installed": false,
        "instruction_contract_installed": false,
        "agent_integration_ready": false,
        "ready_scope": "unavailable",
        "instruction_path": "unavailable",
        "current_session_loaded": "unverified",
        "fresh_session_required": false,
        "diagnostic": "Workspace load failed."
    })
}

#[test]
fn validation_projection_versions_have_distinct_immutable_authority() {
    let schemas = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas");
    let v1 = read_json(&schemas.join("v1/validation.schema.json"));
    assert_eq!(v1["required"], json!(["valid", "errors", "counts"]));
    assert_eq!(
        object_keys(&v1["properties"]),
        BTreeSet::from(["counts", "errors", "valid"])
    );
    assert_eq!(
        object_keys(&v1["$defs"]["counts"]["properties"]),
        BTreeSet::from(["claim", "evidence", "experiment", "review", "source"])
    );

    let v2 = read_json(&schemas.join("v2/validation.schema.json"));
    assert_eq!(
        v2["required"],
        json!([
            "schema_version",
            "authority",
            "valid",
            "errors",
            "counts",
            "agent_integration"
        ])
    );
    assert_eq!(v2["properties"]["schema_version"]["const"], 2);
    let v2_count_keys = BTreeSet::from([
        "source",
        "claim",
        "evidence",
        "experiment",
        "review",
        "review-authority",
        "inventory",
        "knowledge",
        "relationship",
        "migration",
        "contribution-protocol",
    ]);
    assert_eq!(
        object_keys(&v2["$defs"]["completeCounts"]["properties"]),
        v2_count_keys
    );
    assert_eq!(
        string_set(&v2["$defs"]["completeCounts"]["required"]),
        v2_count_keys
    );
    assert_eq!(v2["properties"]["errors"]["maxItems"], 256);
    assert_eq!(
        v2["properties"]["agent_integration"]["$ref"],
        "#/$defs/agentIntegrationStatus"
    );
    assert_eq!(
        v2["properties"]["errors"]["items"]["$ref"],
        "../v1/types.schema.json#/$defs/text"
    );
}

fn object_keys(value: &Value) -> BTreeSet<&str> {
    value
        .as_object()
        .expect("schema object")
        .keys()
        .map(String::as_str)
        .collect()
}

fn string_set(value: &Value) -> BTreeSet<&str> {
    value
        .as_array()
        .expect("schema string array")
        .iter()
        .map(|value| value.as_str().expect("schema string"))
        .collect()
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read JSON fixture")).expect("valid JSON")
}
