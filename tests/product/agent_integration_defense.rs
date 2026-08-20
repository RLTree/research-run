use std::fs;

use serde_json::{Value, json};

use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn existing_instructions_are_preserved_and_drift_conflicts() {
    let temporary = TempDir::new("agent-integration-preserve");
    let project = initialize(&temporary);
    let existing = b"# Existing project law\n\nKeep this byte-for-byte.\n";
    fs::write(project.join("AGENTS.md"), existing).unwrap();
    let plan_path = plan(&temporary, &project, "append-plan.json");
    let plan_json: Value = serde_json::from_slice(&fs::read(&plan_path).unwrap()).unwrap();
    assert_eq!(plan_json["operation"], "append");
    apply(&temporary, &project, &plan_path);
    assert!(
        fs::read(project.join("AGENTS.md"))
            .unwrap()
            .starts_with(existing)
    );

    let stale = plan(&temporary, &project, "stale-plan.json");
    let mut changed = fs::read_to_string(project.join("AGENTS.md")).unwrap();
    changed.push_str("\nExternal change.\n");
    fs::write(project.join("AGENTS.md"), changed).unwrap();
    let output = cli(
        &temporary.0,
        &[
            "agent-integration",
            "apply",
            project.to_str().unwrap(),
            "--input",
            stale.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("drifted"));
}

#[test]
fn empty_instruction_file_remains_an_append_target() {
    let temporary = TempDir::new("agent-integration-empty-instructions");
    let project = initialize(&temporary);
    fs::write(project.join("AGENTS.md"), b"").unwrap();
    let plan_path = plan(&temporary, &project, "empty-plan.json");
    let plan_json: Value = serde_json::from_slice(&fs::read(&plan_path).unwrap()).unwrap();
    assert_eq!(plan_json["operation"], "append");
    assert!(plan_json["instruction_sha256"].is_string());

    apply(&temporary, &project, &plan_path);
    assert!(
        fs::read_to_string(project.join("AGENTS.md"))
            .unwrap()
            .starts_with("<!-- research-run:agent-integration:v1:start -->")
    );
}

#[test]
fn conflicting_managed_content_and_symlinks_fail_closed() {
    let temporary = TempDir::new("agent-integration-defense");
    let project = initialize(&temporary);
    fs::write(
        project.join("AGENTS.md"),
        "<!-- research-run:agent-integration:v1:start -->\nmodified\n<!-- research-run:agent-integration:v1:end -->\n",
    )
    .unwrap();
    let conflict = cli(
        &temporary.0,
        &["agent-integration", "plan", project.to_str().unwrap()],
    );
    assert!(!conflict.status.success());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("conflicts"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        fs::remove_file(project.join("AGENTS.md")).unwrap();
        let outside = temporary.0.join("outside.md");
        fs::write(&outside, "outside\n").unwrap();
        symlink(&outside, project.join("AGENTS.md")).unwrap();
        let linked = cli(
            &temporary.0,
            &[
                "agent-integration",
                "status",
                project.to_str().unwrap(),
                "--json",
            ],
        );
        assert!(!linked.status.success());
        assert!(String::from_utf8_lossy(&linked.stderr).contains("symlink is forbidden"));
    }
}

#[test]
fn direct_canonical_mutation_cannot_satisfy_agent_readiness() {
    let temporary = TempDir::new("agent-integration-direct-write");
    let project = initialize(&temporary);
    let direct = json!({
        "schema_version": 1, "kind": "knowledge", "id": "decision-direct",
        "record_type": "decision", "title": "Unsupported direct write",
        "body": "Structurally valid content written outside the supported CLI.",
        "occurred_at": "2026-08-20T18:00:00Z", "state": "active", "authorship": "human"
    });
    fs::write(
        project.join(".research-run/knowledge/decision-direct.json"),
        serde_json::to_vec_pretty(&direct).unwrap(),
    )
    .unwrap();
    let validation: Value =
        serde_json::from_slice(&succeeds(&project, &["validate", "--json"]).stdout).unwrap();
    assert_eq!(validation["valid"], true);
    assert_eq!(
        validation["agent_integration"]["instruction_contract_installed"],
        false
    );
    assert_eq!(
        validation["agent_integration"]["agent_integration_ready"],
        false
    );
}

#[test]
fn override_is_active_and_plan_input_is_not_linkable() {
    let temporary = TempDir::new("agent-integration-override");
    let project = initialize(&temporary);
    fs::write(project.join("AGENTS.md"), "# Base law\n").unwrap();
    let override_bytes = b"# Active override\n";
    fs::write(project.join("AGENTS.override.md"), override_bytes).unwrap();
    let plan_path = plan(&temporary, &project, "override-plan.json");
    let plan_json: Value = serde_json::from_slice(&fs::read(&plan_path).unwrap()).unwrap();
    assert_eq!(plan_json["instruction_path"], "AGENTS.override.md");
    apply(&temporary, &project, &plan_path);
    assert_eq!(
        fs::read(project.join("AGENTS.md")).unwrap(),
        b"# Base law\n"
    );
    assert!(
        fs::read(project.join("AGENTS.override.md"))
            .unwrap()
            .starts_with(override_bytes)
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let linked = temporary.0.join("linked-plan.json");
        symlink(&plan_path, &linked).unwrap();
        let output = cli(
            &temporary.0,
            &[
                "agent-integration",
                "apply",
                project.to_str().unwrap(),
                "--input",
                linked.to_str().unwrap(),
            ],
        );
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("symlink is forbidden"));
    }
}

#[test]
fn tampered_plan_cannot_escape_the_project_instruction_surface() {
    let temporary = TempDir::new("agent-integration-plan-tamper");
    let project = initialize(&temporary);
    let plan_path = plan(&temporary, &project, "tampered-plan.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&plan_path).unwrap()).unwrap();
    value["instruction_path"] = json!("../AGENTS.md");
    fs::write(&plan_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let output = cli(
        &temporary.0,
        &[
            "agent-integration",
            "apply",
            project.to_str().unwrap(),
            "--input",
            plan_path.to_str().unwrap(),
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("instruction_path"));
    assert!(!temporary.0.join("AGENTS.md").exists());
}

fn initialize(temporary: &TempDir) -> std::path::PathBuf {
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Integration defense",
            "--without-review-authority",
        ],
    );
    project
}

fn plan(temporary: &TempDir, project: &std::path::Path, name: &str) -> std::path::PathBuf {
    let output = succeeds(
        &temporary.0,
        &["agent-integration", "plan", project.to_str().unwrap()],
    );
    let path = temporary.0.join(name);
    fs::write(&path, output.stdout).unwrap();
    path
}

fn apply(temporary: &TempDir, project: &std::path::Path, plan: &std::path::Path) -> Value {
    serde_json::from_slice(
        &succeeds(
            &temporary.0,
            &[
                "agent-integration",
                "apply",
                project.to_str().unwrap(),
                "--input",
                plan.to_str().unwrap(),
                "--json",
            ],
        )
        .stdout,
    )
    .unwrap()
}
