use std::fs;
use std::path::Path;

use jsonschema::Resource;
use serde_json::Value;
use serde_json::json;

#[test]
fn agent_integration_plan_workspace_identity_preserves_legacy_shape() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/v1");
    let schema: Value = serde_json::from_slice(
        &fs::read(root.join("agent-integration-plan.schema.json"))
            .expect("agent integration plan schema"),
    )
    .expect("agent integration plan schema JSON");
    assert!(
        !schema["required"]
            .as_array()
            .expect("required array")
            .iter()
            .any(|field| field == "workspace_id")
    );
    assert_eq!(
        schema["properties"]["workspace_id"]["oneOf"][0]["const"],
        ""
    );
    assert_eq!(
        schema["properties"]["workspace_id"]["oneOf"][1]["$ref"],
        "types.schema.json#/$defs/sha256"
    );
}

#[test]
fn agent_integration_plan_schema_enforces_operation_identity_invariants() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/v1");
    let schema: Value = serde_json::from_slice(
        &fs::read(root.join("agent-integration-plan.schema.json")).expect("plan schema"),
    )
    .expect("plan schema JSON");
    let types: Value =
        serde_json::from_slice(&fs::read(root.join("types.schema.json")).expect("types schema"))
            .expect("types schema JSON");
    let validator = jsonschema::draft202012::options()
        .with_resource(
            "types.schema.json",
            Resource::from_contents(types).expect("types resource"),
        )
        .build(&schema)
        .expect("plan validator");
    let digest = "a".repeat(64);
    let mut plan = json!({
        "schema_version": 1,
        "kind": "agent-integration-plan",
        "project_id": "project",
        "manifest_sha256": digest,
        "protocol_sha256": "b".repeat(64),
        "instruction_path": "AGENTS.md",
        "instruction_sha256": null,
        "instruction_bytes": 0,
        "managed_block": "managed",
        "managed_block_sha256": "c".repeat(64),
        "prospective_sha256": "d".repeat(64),
        "operation": "create",
        "plan_sha256": "e".repeat(64)
    });
    assert!(validator.is_valid(&plan));

    plan["instruction_sha256"] = Value::String("f".repeat(64));
    assert!(!validator.is_valid(&plan));
    plan["instruction_sha256"] = Value::Null;
    plan["instruction_bytes"] = Value::from(1);
    assert!(!validator.is_valid(&plan));

    for operation in ["append", "no-op"] {
        plan["operation"] = Value::String(operation.to_owned());
        plan["instruction_bytes"] = Value::from(0);
        plan["instruction_sha256"] = Value::Null;
        assert!(!validator.is_valid(&plan), "{operation}");
        plan["instruction_sha256"] = Value::String("f".repeat(64));
        assert!(validator.is_valid(&plan), "{operation}");
    }
}
