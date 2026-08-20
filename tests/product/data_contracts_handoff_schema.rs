use std::fs;
use std::path::Path;

use jsonschema::Resource;
use serde_json::{Value, json};

#[test]
fn handoff_schema_limits_empty_workspace_identity_to_v1() {
    let schemas = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas");
    let handoff = read_json(&schemas.join("v1/handoff.schema.json"));
    let validator = jsonschema::draft202012::options()
        .with_resources(
            [
                resource(&schemas, "v1/types.schema.json"),
                resource(&schemas, "v1/contribution-protocol.schema.json"),
                resource(&schemas, "v1/projection.schema.json"),
                resource(&schemas, "v2/validation.schema.json"),
            ]
            .into_iter(),
        )
        .build(&handoff)
        .expect("handoff schema");

    let anchored = handoff_v2();
    assert!(validator.is_valid(&anchored));
    let mut empty_v2 = anchored.clone();
    empty_v2["context"]["workspace_id"] = Value::String(String::new());
    assert!(!validator.is_valid(&empty_v2));

    let mut empty_v2_authority = anchored.clone();
    empty_v2_authority["validation"]["result"]["authority"]["workspace_id"] =
        Value::String(String::new());
    assert!(!validator.is_valid(&empty_v2_authority));

    let mut legacy_v1 = anchored;
    legacy_v1["schema_version"] = Value::from(1);
    legacy_v1
        .as_object_mut()
        .expect("handoff object")
        .remove("validation");
    legacy_v1["context"]["workspace_id"] = Value::String(String::new());
    assert!(validator.is_valid(&legacy_v1));
}

fn resource(schemas: &Path, relative: &str) -> (String, Resource) {
    let path = schemas.join(relative);
    let value = read_json(&path);
    (
        resource_id(relative),
        Resource::from_contents(value).expect("schema resource"),
    )
}

fn resource_id(relative: &str) -> String {
    format!("https://research-run.local/schemas/{relative}")
}

fn handoff_v2() -> Value {
    let digest = "a".repeat(64);
    json!({
        "schema_version": 2,
        "kind": "handoff",
        "id": "schema-handoff",
        "generated_at": "2026-08-20T00:00:00Z",
        "context": {
            "kind": "context",
            "project_id": "schema-project",
            "workspace_id": digest,
            "project_name": "Schema project",
            "claim_ceiling": "Assessments describe reviewed support within this workspace; they do not establish scientific truth or real-world validity.",
            "scope": "schema contract",
            "contribution_protocol": {
                "schema_version": 1,
                "kind": "contribution-protocol",
                "id": "agent-contribution",
                "sequence": ["retrieve-bounded-context", "classify-new-material", "append-typed-records", "validate-workspace", "answer-or-handoff"],
                "triggers": [
                    "human-observation", "human-correction", "human-decision",
                    "negative-result", "ambiguous-result", "blocker", "next-action"
                ],
                "human_input_authorship": "human",
                "agent_analysis_authorship": "ai",
                "no_new_material_effect": "no-write",
                "identical_retry_effect": "no-op",
                "claim_promotion": "signed-human-review-only"
            },
            "matches": [], "unresolved": [], "blockers": [], "next_actions": [], "relationships": []
        },
        "validation": {
            "result": {
                "schema_version": 2,
                "authority": {
                    "project_id": "schema-project", "workspace_id": "a".repeat(64),
                    "manifest_sha256": "b".repeat(64), "protocol_sha256": "c".repeat(64),
                    "instruction_sha256": "d".repeat(64)
                },
                "valid": true, "errors": [],
                "counts": {
                    "source": 0, "claim": 0, "evidence": 0, "experiment": 0,
                    "review": 0, "review-authority": 0, "inventory": 0,
                    "knowledge": 0, "relationship": 0, "migration": 0,
                    "contribution-protocol": 1
                },
                "agent_integration": {
                    "protocol_installed": true, "instruction_contract_installed": true,
                    "agent_integration_ready": true, "ready_scope": "new-agent-run",
                    "instruction_path": "AGENTS.md", "current_session_loaded": "unverified",
                    "fresh_session_required": true, "diagnostic": "Start a fresh agent run."
                }
            },
            "result_sha256": "e".repeat(64), "context_sha256": "f".repeat(64)
        }
    })
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read schema")).expect("valid schema JSON")
}
