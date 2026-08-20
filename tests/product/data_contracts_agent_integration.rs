use std::fs;
use std::path::Path;

use serde_json::Value;

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
