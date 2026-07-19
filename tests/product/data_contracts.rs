use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

#[test]
fn every_versioned_schema_is_well_formed_json() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/v1");
    let mut schemas = Vec::new();
    for entry in fs::read_dir(&root).expect("schema directory") {
        let path = entry.expect("schema entry").path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let value: Value = serde_json::from_slice(&fs::read(&path).expect("read schema"))
                .unwrap_or_else(|error| panic!("{} is invalid JSON: {error}", path.display()));
            assert_eq!(
                value["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
            schemas.push(path);
        }
    }
    let names = schemas
        .iter()
        .map(|path| {
            path.file_name()
                .expect("schema filename")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<BTreeSet<_>>();
    let expected = [
        "claim.schema.json",
        "evidence.schema.json",
        "experiment.schema.json",
        "inventory.schema.json",
        "knowledge.schema.json",
        "project-manifest.schema.json",
        "projection.schema.json",
        "review.schema.json",
        "relationship.schema.json",
        "source.schema.json",
        "status.schema.json",
        "types.schema.json",
        "validation.schema.json",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    assert_eq!(names, expected, "v1 schema authority changed");
}

#[test]
fn checked_in_synthetic_example_is_valid_and_reports_limited_claim() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/synthetic-assay");
    let binary = std::env::var("CARGO_BIN_EXE_research-run").expect("binary path");
    let validation = Command::new(&binary)
        .current_dir(&example)
        .args(["validate", "--json"])
        .output()
        .expect("validate example");
    assert!(
        validation.status.success(),
        "{}",
        String::from_utf8_lossy(&validation.stderr)
    );
    let status = Command::new(binary)
        .current_dir(&example)
        .args(["status", "--json"])
        .output()
        .expect("status example");
    assert!(status.status.success());
    let status: Value = serde_json::from_slice(&status.stdout).expect("status JSON");
    assert_eq!(status["claims"][0]["assessment"], "limited");
    assert_eq!(status["experiments"][0]["outcome"], "negative");
}
