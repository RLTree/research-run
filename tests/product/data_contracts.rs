use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

use super::researcher_journey::{TempDir, cli, succeeds};

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
        "agent-integration-plan.schema.json",
        "claim.schema.json",
        "contribution-protocol.schema.json",
        "evidence.schema.json",
        "experiment.schema.json",
        "handoff.schema.json",
        "inventory-policy.schema.json",
        "inventory.schema.json",
        "knowledge.schema.json",
        "migration.schema.json",
        "project-manifest.schema.json",
        "projection.schema.json",
        "review-authority.schema.json",
        "review-request.schema.json",
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
fn projection_timestamps_use_the_shared_utc_contract() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/v1");
    let projection: Value = serde_json::from_slice(
        &fs::read(root.join("projection.schema.json")).expect("projection schema"),
    )
    .expect("projection schema JSON");
    let timestamp = &projection["$defs"]["item"]["properties"]["occurred_at"]["oneOf"];
    assert_eq!(timestamp[0]["$ref"], "types.schema.json#/$defs/timestamp");
    assert_eq!(timestamp[1]["type"], "null");
}

#[test]
fn validation_projection_versions_have_distinct_immutable_authority() {
    let schemas = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas");
    let v1: Value = serde_json::from_slice(
        &fs::read(schemas.join("v1/validation.schema.json")).expect("v1 validation schema"),
    )
    .expect("v1 validation schema JSON");
    assert_eq!(v1["required"], json!(["valid", "errors", "counts"]));
    assert_eq!(
        object_keys(&v1["properties"]),
        BTreeSet::from(["counts", "errors", "valid"])
    );
    assert_eq!(
        object_keys(&v1["$defs"]["counts"]["properties"]),
        BTreeSet::from(["claim", "evidence", "experiment", "review", "source"])
    );

    let v2: Value = serde_json::from_slice(
        &fs::read(schemas.join("v2/validation.schema.json")).expect("v2 validation schema"),
    )
    .expect("v2 validation schema JSON");
    assert_eq!(
        v2["required"],
        json!([
            "schema_version",
            "valid",
            "errors",
            "counts",
            "agent_integration"
        ])
    );
    assert_eq!(v2["properties"]["schema_version"]["const"], 2);
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

#[test]
fn every_v2_schema_is_well_formed_json() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/v2");
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(&root).expect("v2 schema directory") {
        let path = entry.expect("v2 schema entry").path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let value: Value = serde_json::from_slice(&fs::read(&path).expect("read v2 schema"))
                .unwrap_or_else(|error| panic!("{} is invalid JSON: {error}", path.display()));
            assert_eq!(
                value["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
            names.insert(
                path.file_name()
                    .expect("v2 schema filename")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    assert_eq!(names, BTreeSet::from(["validation.schema.json".to_owned()]));
}

#[cfg(unix)]
#[test]
fn instruction_inspection_failure_does_not_invalidate_the_canonical_ledger() {
    use std::os::unix::fs::symlink;

    let temporary = TempDir::new("validation-readiness-separation");
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Validation readiness separation",
            "--without-review-authority",
        ],
    );
    let outside = temporary.0.join("outside-agents.md");
    fs::write(&outside, "outside\n").unwrap();
    symlink(&outside, project.join("AGENTS.md")).unwrap();

    let validation: Value =
        serde_json::from_slice(&succeeds(&project, &["validate", "--json"]).stdout).unwrap();
    assert_eq!(validation["schema_version"], 2);
    assert_eq!(validation["valid"], true);
    assert_eq!(validation["errors"], json!([]));
    assert_eq!(
        validation["agent_integration"]["agent_integration_ready"],
        false
    );
    assert_eq!(
        validation["agent_integration"]["ready_scope"],
        "unavailable"
    );
    assert!(
        validation["agent_integration"]["diagnostic"]
            .as_str()
            .unwrap()
            .contains("symlink is forbidden")
    );

    let explicit = cli(
        &temporary.0,
        &[
            "agent-integration",
            "status",
            project.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!explicit.status.success());
    assert!(String::from_utf8_lossy(&explicit.stderr).contains("symlink is forbidden"));
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
    let validation: Value =
        serde_json::from_slice(&validation.stdout).expect("validation projection JSON");
    assert_eq!(validation["schema_version"], 2);
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
