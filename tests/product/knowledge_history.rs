use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use super::researcher_journey::{TempDir, cli, succeeds};

const KINDS: &[&str] = &[
    "goal",
    "research-question",
    "hypothesis",
    "protocol",
    "method",
    "observation",
    "measurement",
    "analysis",
    "interpretation",
    "decision",
    "risk",
    "blocker",
    "uncertainty",
    "contradiction",
    "next-action",
    "plan",
    "presentation",
    "session-summary",
];

#[test]
fn every_knowledge_class_is_typed_and_history_is_append_only() {
    let temporary = TempDir::new("knowledge");
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Knowledge project",
        ],
    );
    for (index, kind) in KINDS.iter().enumerate() {
        let id = format!("record-{index:02}");
        add_json(&temporary, &project, "knowledge", knowledge(&id, kind));
    }
    add_json(
        &temporary,
        &project,
        "relationship",
        relationship("revision-one", "revises", "record-07", "record-05"),
    );
    add_json(
        &temporary,
        &project,
        "relationship",
        relationship("invalidation-one", "invalidates", "record-09", "record-08"),
    );
    assert!(
        project
            .join(".research-run/knowledge/record-05.json")
            .exists()
    );
    assert!(
        project
            .join(".research-run/knowledge/record-07.json")
            .exists()
    );
    assert!(
        project
            .join(".research-run/relationships/revision-one.json")
            .exists()
    );
}

#[test]
fn relationship_references_and_history_cycles_fail_before_effect() {
    let temporary = TempDir::new("knowledge-defense");
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &["init", project.to_str().unwrap(), "--name", "Defense"],
    );
    add_json(
        &temporary,
        &project,
        "knowledge",
        knowledge("record-old", "observation"),
    );
    add_json(
        &temporary,
        &project,
        "knowledge",
        knowledge("record-new", "analysis"),
    );
    add_json(
        &temporary,
        &project,
        "relationship",
        relationship("revision-forward", "revises", "record-new", "record-old"),
    );
    let cycle = relationship("revision-cycle", "revises", "record-old", "record-new");
    assert_add_fails(&temporary, &project, "relationship", cycle, "acyclic");
    let unknown = relationship(
        "unknown-target",
        "related-to",
        "record-new",
        "missing-record",
    );
    assert_add_fails(&temporary, &project, "relationship", unknown, "unknown");
}

#[test]
fn structured_stdin_is_supported_without_promoting_claims() {
    let temporary = TempDir::new("knowledge-stdin");
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &["init", project.to_str().unwrap(), "--name", "Stdin"],
    );
    let binary = std::env::var("CARGO_BIN_EXE_research-run").expect("binary path");
    let mut child = Command::new(binary)
        .current_dir(&project)
        .args(["knowledge", "add", "--input", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn knowledge command");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            knowledge("stdin-note", "session-summary")
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    assert!(
        child
            .wait_with_output()
            .expect("stdin output")
            .status
            .success()
    );
}

fn add_json(temporary: &TempDir, project: &std::path::Path, command: &str, value: Value) {
    let path = temporary.0.join(format!("{command}-input.json"));
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).expect("write input");
    succeeds(
        project,
        &[command, "add", "--input", path.to_str().unwrap()],
    );
}

fn assert_add_fails(
    temporary: &TempDir,
    project: &std::path::Path,
    command: &str,
    value: Value,
    expected: &str,
) {
    let path = temporary.0.join(format!("{command}-failure.json"));
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).expect("write input");
    let output = cli(
        project,
        &[command, "add", "--input", path.to_str().unwrap()],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
}

fn knowledge(id: &str, record_type: &str) -> Value {
    json!({
        "schema_version": 1,
        "kind": "knowledge",
        "id": id,
        "record_type": record_type,
        "title": format!("Title for {id}"),
        "body": format!("Body for {id}"),
        "occurred_at": "2026-07-18T20:00:00Z",
        "state": "open",
        "authorship": "human"
    })
}

fn relationship(id: &str, kind: &str, from: &str, to: &str) -> Value {
    json!({
        "schema_version": 1,
        "kind": "relationship",
        "id": id,
        "relationship": kind,
        "from": { "kind": "knowledge", "id": from },
        "to": { "kind": "knowledge", "id": to },
        "rationale": "Explicit append-only history",
        "occurred_at": "2026-07-18T20:01:00Z",
        "authorship": "human"
    })
}
