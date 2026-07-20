use std::fs;

use serde_json::{Value, json};

use super::researcher_journey::{TempDir, succeeds};

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
    let next_actions = inspected["context"]["next_actions"].as_array().unwrap();
    assert!(next_actions.iter().any(|item| item["id"] == "next-order"));
    assert!(next_actions.iter().any(|item| {
        item["kind"] == "claim" && item["id"] == "claim-assay" && item["state"] == "limited"
    }));
    let matches = inspected["context"]["matches"].as_array().unwrap();
    for (kind, id) in [
        ("claim", "claim-assay"),
        ("evidence", "evidence-assay"),
        ("review", "review-assay"),
    ] {
        assert!(
            matches
                .iter()
                .any(|item| item["kind"] == kind && item["id"] == id)
        );
    }
    assert!(matches.iter().any(|item| item["id"] == "decision-assay"));
    assert!(
        matches
            .iter()
            .any(|item| item["id"] == "observation-negative")
    );
    let human_inspect = succeeds(
        &fresh,
        &[
            "handoff",
            "inspect",
            "--input",
            handoff_path.to_str().unwrap(),
            "--human",
        ],
    );
    assert!(String::from_utf8_lossy(&human_inspect.stdout).contains("Research Run context"));
    let human_create = succeeds(
        &temporary.0.join("project"),
        &[
            "handoff",
            "create",
            "--id",
            "handoff-human",
            "--generated-at",
            "2026-07-18T20:06:00Z",
            "--human",
        ],
    );
    assert!(String::from_utf8_lossy(&human_create.stdout).contains("Research Run context"));
}

pub(super) fn create_handoff_fixture(temporary: &TempDir) -> std::path::PathBuf {
    let project = temporary.0.join("project");
    crate::review_test_signing::initialize_with_test_authority(
        &temporary.0,
        &project,
        "Handoff project",
    );
    add_research_graph(&project);
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

fn add_research_graph(project: &std::path::Path) {
    succeeds(
        project,
        &[
            "source",
            "add",
            "--id",
            "source-assay",
            "--citation",
            "Synthetic assay note",
            "--locator",
            "local:assay",
            "--provenance",
            "human",
        ],
    );
    succeeds(
        project,
        &[
            "claim",
            "add",
            "--id",
            "claim-assay",
            "--text",
            "Assay B is preferred.",
            "--scope",
            "Synthetic fixture",
            "--owner",
            "Example Researcher",
            "--authorship",
            "human",
        ],
    );
    succeeds(
        project,
        &[
            "evidence",
            "add",
            "--id",
            "evidence-assay",
            "--claim",
            "claim-assay",
            "--source",
            "source-assay",
            "--stance",
            "limits",
            "--specific-evidence",
            "Synthetic evidence is bounded.",
            "--authorship",
            "human",
        ],
    );
    crate::review_test_signing::add_signed_review(
        project,
        "review-assay",
        "claim-assay",
        "limited",
        "Reviewed within the synthetic fixture.",
        "Example Researcher",
    );
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
