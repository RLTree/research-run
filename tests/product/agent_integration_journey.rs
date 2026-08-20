use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use super::researcher_journey::{TempDir, succeeds};

#[test]
fn material_human_decision_uses_supported_agent_ready_journey() {
    let temporary = TempDir::new("agent-integration-journey");
    let project = initialize(&temporary, "Agent integration");
    assert_onboarding_and_install(&temporary, &project);

    let before = knowledge_count(&project);
    let context = json_command(&project, &["context", "--query", "buffer"]);
    assert_eq!(
        context["contribution_protocol"]["kind"],
        "contribution-protocol"
    );
    assert_eq!(knowledge_count(&project), before, "retrieval is read-only");

    add_material_decision(&temporary, &project);
    let validation = json_command(&project, &["validate", "--json"]);
    assert_eq!(validation["valid"], true);
    assert_eq!(
        validation["agent_integration"]["agent_integration_ready"],
        true
    );
    let human_validation = succeeds(&project, &["validate"]);
    assert!(
        String::from_utf8_lossy(&human_validation.stdout)
            .contains("Agent integration ready for new-agent-run: true")
    );
    let handoff = json_command(
        &project,
        &[
            "handoff",
            "create",
            "--id",
            "handoff-agent-ready",
            "--generated-at",
            "2026-08-20T18:05:00Z",
            "--query",
            "buffer",
        ],
    );
    assert_eq!(handoff["schema_version"], 2);
    assert!(
        handoff["context"]["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "decision-buffer")
    );
}

#[test]
fn human_integration_output_preserves_the_new_run_boundary() {
    let temporary = TempDir::new("agent-integration-human");
    let project = initialize(&temporary, "Human integration output");
    let plan = succeeds(
        &temporary.0,
        &["agent-integration", "plan", project.to_str().unwrap()],
    );
    let plan_path = temporary.0.join("human-plan.json");
    fs::write(&plan_path, plan.stdout).unwrap();
    let applied = succeeds(
        &temporary.0,
        &[
            "agent-integration",
            "apply",
            project.to_str().unwrap(),
            "--input",
            plan_path.to_str().unwrap(),
        ],
    );
    assert!(String::from_utf8_lossy(&applied.stdout).contains("contract installed"));
    let retry = succeeds(
        &temporary.0,
        &[
            "agent-integration",
            "apply",
            project.to_str().unwrap(),
            "--input",
            plan_path.to_str().unwrap(),
        ],
    );
    assert!(String::from_utf8_lossy(&retry.stdout).contains("already installed"));
    let status = succeeds(
        &temporary.0,
        &["agent-integration", "status", project.to_str().unwrap()],
    );
    let text = String::from_utf8_lossy(&status.stdout);
    assert!(text.contains("Ready scope: new-agent-run"));
    assert!(text.contains("Current session load: unverified"));
}

fn assert_onboarding_and_install(temporary: &TempDir, project: &Path) {
    let status = json_command(
        &temporary.0,
        &[
            "agent-integration",
            "status",
            project.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(status["protocol_installed"], true);
    assert_eq!(status["instruction_contract_installed"], false);
    assert_eq!(status["agent_integration_ready"], false);

    let plan = succeeds(
        &temporary.0,
        &["agent-integration", "plan", project.to_str().unwrap()],
    );
    let plan_json: Value = serde_json::from_slice(&plan.stdout).expect("plan JSON");
    assert_eq!(plan_json["operation"], "create");
    assert_eq!(plan_json["instruction_path"], "AGENTS.md");
    let plan_path = temporary.0.join("agent-plan.json");
    fs::write(&plan_path, &plan.stdout).expect("write plan fixture");

    let applied = json_command(
        &temporary.0,
        &[
            "agent-integration",
            "apply",
            project.to_str().unwrap(),
            "--input",
            plan_path.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(applied["changed"], true);
    assert_eq!(applied["ready_for_new_agent_run"], true);
    assert_eq!(applied["current_session_loaded"], "unverified");
    assert_eq!(applied["fresh_session_required"], true);
    let instructions = fs::read_to_string(project.join("AGENTS.md")).expect("instructions");
    assert_instruction_contract(&instructions);

    let retry = json_command(
        &temporary.0,
        &[
            "agent-integration",
            "apply",
            project.to_str().unwrap(),
            "--input",
            plan_path.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(retry["changed"], false);
}

fn initialize(temporary: &TempDir, name: &str) -> std::path::PathBuf {
    let project = temporary.0.join("project");
    let output = succeeds(
        &temporary.0,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            name,
            "--without-review-authority",
        ],
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Agent integration ready: false"));
    assert!(text.contains("fresh agent run/session"));
    project
}

fn assert_instruction_contract(instructions: &str) {
    for required in [
        "retrieve bounded context",
        "supported typed Research Run CLI",
        "human authorship",
        "no new material, do not write",
        "Never create or edit canonical `.research-run` files",
        "research-run validate --json",
        "actual CLI invocation",
        "Research Run v2 handoff",
        "signed human `ReviewDecision`",
        "fresh agent run/session",
    ] {
        assert!(instructions.contains(required), "missing: {required}");
    }
}

fn add_material_decision(temporary: &TempDir, project: &Path) {
    let decision = json!({
        "schema_version": 1, "kind": "knowledge", "id": "decision-buffer",
        "record_type": "decision", "title": "Use the selected buffer",
        "body": "The researcher selected the recorded buffer condition.",
        "occurred_at": "2026-08-20T18:00:00Z", "state": "active", "authorship": "human"
    });
    let next = json!({
        "schema_version": 1, "kind": "knowledge", "id": "next-buffer-check",
        "record_type": "next-action", "title": "Check the buffer condition",
        "body": "Verify the selected buffer condition in the next bounded run.",
        "occurred_at": "2026-08-20T18:01:00Z", "state": "open", "authorship": "human"
    });
    let relationship = json!({
        "schema_version": 1, "kind": "relationship", "id": "relationship-buffer-next",
        "relationship": "depends-on",
        "from": {"kind": "knowledge", "id": "next-buffer-check"},
        "to": {"kind": "knowledge", "id": "decision-buffer"},
        "rationale": "The next action implements the human decision.",
        "occurred_at": "2026-08-20T18:02:00Z", "authorship": "human"
    });
    for (name, command, value) in [
        ("decision.json", "knowledge", decision),
        ("next.json", "knowledge", next),
        ("relationship.json", "relationship", relationship),
    ] {
        let input = temporary.0.join(name);
        fs::write(&input, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        succeeds(
            project,
            &[command, "add", "--input", input.to_str().unwrap()],
        );
    }
    let canonical: Value = serde_json::from_slice(
        &fs::read(project.join(".research-run/knowledge/decision-buffer.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(canonical["authorship"], "human");
}

fn knowledge_count(project: &Path) -> usize {
    fs::read_dir(project.join(".research-run/knowledge"))
        .unwrap()
        .count()
}

fn json_command(cwd: &Path, args: &[&str]) -> Value {
    serde_json::from_slice(&succeeds(cwd, args).stdout).expect("command JSON")
}
