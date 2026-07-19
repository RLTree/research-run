use std::fs;

use serde_json::{Value, json};

use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn bounded_retrieval_commands_explain_matches_and_preserve_history() {
    let temporary = TempDir::new("retrieval");
    let project = setup(&temporary);
    assert_search_and_show(&project);
    assert_time_and_relationships(&project);
    assert_work_projections(&project);
    assert_context_and_human(&project);
}

#[test]
fn retrieval_rejects_empty_queries_and_invalid_limits() {
    let temporary = TempDir::new("retrieval-defense");
    let project = setup(&temporary);
    for args in [
        vec!["search", "   "],
        vec!["list", "--limit", "0"],
        vec!["recent", "--limit", "257"],
    ] {
        assert!(!cli(&project, &args).status.success(), "{args:?}");
    }
    let missing = cli(
        &project,
        &["show", "--kind", "knowledge", "--id", "not-present"],
    );
    assert_eq!(missing.status.code(), Some(3));
}

fn setup(temporary: &TempDir) -> std::path::PathBuf {
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &["init", project.to_str().unwrap(), "--name", "Retrieval"],
    );
    for (id, kind, title, at) in [
        (
            "question-open",
            "research-question",
            "Does signal reproduce?",
            "2026-07-18T20:00:00Z",
        ),
        (
            "observation-old",
            "observation",
            "Original signal",
            "2026-07-18T20:01:00Z",
        ),
        (
            "analysis-new",
            "analysis",
            "Revised signal analysis",
            "2026-07-18T20:02:00Z",
        ),
        (
            "blocker-open",
            "blocker",
            "Missing replicate",
            "2026-07-18T20:03:00Z",
        ),
        (
            "next-repeat",
            "next-action",
            "Run independent replicate",
            "2026-07-18T20:04:00Z",
        ),
    ] {
        add(
            temporary,
            &project,
            "knowledge",
            knowledge(id, kind, title, at),
        );
    }
    add(
        temporary,
        &project,
        "relationship",
        relationship("revision-signal", "analysis-new", "observation-old"),
    );
    project
}

fn assert_search_and_show(project: &std::path::Path) {
    let search = json_output(project, &["search", "signal", "--limit", "10"]);
    assert!(search["total_matches"].as_u64().unwrap() >= 2);
    assert!(search["items"].as_array().unwrap().iter().all(|item| {
        !item["matched_by"].as_array().unwrap().is_empty()
            && item["authority_path"]
                .as_str()
                .unwrap()
                .starts_with(".research-run/")
    }));
    let old = json_output(
        project,
        &["show", "--kind", "knowledge", "--id", "observation-old"],
    );
    assert_eq!(old["stale"], true);
    assert_eq!(old["invalidated"], false);
}

fn assert_time_and_relationships(project: &std::path::Path) {
    let recent = json_output(project, &["recent", "--limit", "2"]);
    assert_eq!(recent["items"][0]["id"], "next-repeat");
    let timeline = json_output(project, &["timeline", "--limit", "2"]);
    assert_eq!(timeline["items"][0]["id"], "question-open");
    let related = json_output(
        project,
        &["related", "--kind", "knowledge", "--id", "analysis-new"],
    );
    assert_eq!(related[0]["relationship"], "revises");
    assert_eq!(related[0]["to_id"], "observation-old");
}

fn assert_work_projections(project: &std::path::Path) {
    let unresolved = json_output(project, &["unresolved"]);
    assert!(
        unresolved["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "question-open")
    );
    let blockers = json_output(project, &["blockers"]);
    assert_eq!(blockers["items"][0]["id"], "blocker-open");
    let next = json_output(project, &["next"]);
    assert_eq!(next["items"][0]["id"], "next-repeat");
    let list = json_output(project, &["list", "--kind", "analysis"]);
    assert_eq!(list["items"][0]["id"], "analysis-new");
}

fn assert_context_and_human(project: &std::path::Path) {
    let context = json_output(project, &["context", "--query", "signal", "--limit", "10"]);
    assert_eq!(context["kind"], "context");
    assert!(!context["matches"].as_array().unwrap().is_empty());
    assert_eq!(context["blockers"][0]["id"], "blocker-open");
    assert_eq!(context["next_actions"][0]["id"], "next-repeat");
    let human = succeeds(project, &["context", "--query", "signal", "--human"]);
    let human = String::from_utf8(human.stdout).expect("human UTF-8");
    assert!(human.contains("Claim ceiling:"));
    assert!(human.contains("[stale]"));
}

fn json_output(project: &std::path::Path, args: &[&str]) -> Value {
    serde_json::from_slice(&succeeds(project, args).stdout).expect("command JSON")
}

fn add(temporary: &TempDir, project: &std::path::Path, command: &str, value: Value) {
    let path = temporary.0.join(format!("{command}.json"));
    fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).expect("input");
    succeeds(
        project,
        &[command, "add", "--input", path.to_str().unwrap()],
    );
}

fn knowledge(id: &str, record_type: &str, title: &str, occurred_at: &str) -> Value {
    json!({
        "schema_version": 1, "kind": "knowledge", "id": id,
        "record_type": record_type, "title": title,
        "body": format!("Detailed context for {title}"), "occurred_at": occurred_at,
        "state": "open", "authorship": "human"
    })
}

fn relationship(id: &str, from: &str, to: &str) -> Value {
    json!({
        "schema_version": 1, "kind": "relationship", "id": id,
        "relationship": "revises",
        "from": { "kind": "knowledge", "id": from },
        "to": { "kind": "knowledge", "id": to },
        "rationale": "New analysis revises the earlier observation",
        "occurred_at": "2026-07-18T20:05:00Z", "authorship": "human"
    })
}
