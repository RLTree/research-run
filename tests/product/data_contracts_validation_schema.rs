use std::fs;
use std::path::Path;

use jsonschema::Resource;
use serde_json::Value;

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
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).expect("read JSON fixture")).expect("valid JSON")
}
