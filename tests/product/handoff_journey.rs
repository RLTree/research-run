use std::fs;

use serde_json::{Value, json};

use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn fresh_process_consumes_handoff_without_source_workspace() {
    let temporary = TempDir::new("handoff");
    let handoff_path = create_handoff_fixture(&temporary);
    let fresh = temporary.0.join("fresh-agent");
    fs::create_dir(&fresh).expect("fresh agent directory");
    let inspected = succeeds(
        &fresh,
        &[
            "handoff",
            "inspect",
            "--input",
            handoff_path.to_str().unwrap(),
        ],
    );
    let inspected: Value = serde_json::from_slice(&inspected.stdout).expect("handoff JSON");
    assert_eq!(inspected["id"], "handoff-current");
    assert_eq!(inspected["context"]["blockers"][0]["id"], "blocker-reagent");
    assert_eq!(inspected["context"]["next_actions"][0]["id"], "next-order");
    let matches = inspected["context"]["matches"].as_array().unwrap();
    assert!(matches.iter().any(|item| item["id"] == "decision-assay"));
    assert!(
        matches
            .iter()
            .any(|item| item["id"] == "observation-negative")
    );
}

fn create_handoff_fixture(temporary: &TempDir) -> std::path::PathBuf {
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Handoff project",
        ],
    );
    for (id, kind, title, at) in [
        (
            "decision-assay",
            "decision",
            "Use assay B",
            "2026-07-18T20:00:00Z",
        ),
        (
            "observation-negative",
            "observation",
            "Replicate was negative",
            "2026-07-18T20:01:00Z",
        ),
        (
            "blocker-reagent",
            "blocker",
            "Reagent is unavailable",
            "2026-07-18T20:02:00Z",
        ),
        (
            "next-order",
            "next-action",
            "Order replacement reagent",
            "2026-07-18T20:03:00Z",
        ),
        (
            "session-current",
            "session-summary",
            "Current session summary",
            "2026-07-18T20:04:00Z",
        ),
    ] {
        add(temporary, &project, knowledge(id, kind, title, at));
    }
    let handoff = succeeds(
        &project,
        &[
            "handoff",
            "create",
            "--id",
            "handoff-current",
            "--generated-at",
            "2026-07-18T20:05:00Z",
            "--limit",
            "20",
        ],
    );
    let handoff_path = temporary.0.join("handoff.json");
    fs::write(&handoff_path, &handoff.stdout).expect("handoff file");
    handoff_path
}

#[test]
fn handoff_inspection_rejects_unknown_versions() {
    let temporary = TempDir::new("handoff-defense");
    let path = temporary.0.join("bad.json");
    fs::write(
        &path,
        br#"{"schema_version":99,"kind":"handoff","id":"bad"}"#,
    )
    .unwrap();
    let output = cli(
        &temporary.0,
        &["handoff", "inspect", "--input", path.to_str().unwrap()],
    );
    assert!(!output.status.success());
}

fn add(temporary: &TempDir, project: &std::path::Path, value: Value) {
    let path = temporary.0.join("knowledge.json");
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).expect("input");
    succeeds(
        project,
        &["knowledge", "add", "--input", path.to_str().unwrap()],
    );
}

fn knowledge(id: &str, record_type: &str, title: &str, occurred_at: &str) -> Value {
    json!({
        "schema_version": 1, "kind": "knowledge", "id": id,
        "record_type": record_type, "title": title, "body": title,
        "occurred_at": occurred_at, "state": "open", "authorship": "human"
    })
}
