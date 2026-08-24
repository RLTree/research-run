use std::fs;
use std::path::{Path, PathBuf};

use research_run::workspace::Workspace;
use serde_json::Value;

use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn direct_managed_block_write_cannot_satisfy_agent_readiness() {
    let temporary = TempDir::new("agent-integration-direct-instruction");
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Direct instruction defense",
            "--without-review-authority",
        ],
    );
    let plan = succeeds(
        &temporary.0,
        &["agent-integration", "plan", project.to_str().unwrap()],
    );
    let plan: Value = serde_json::from_slice(&plan.stdout).unwrap();
    fs::write(
        project.join("AGENTS.md"),
        plan["managed_block"].as_str().unwrap(),
    )
    .unwrap();

    let status = succeeds(
        &temporary.0,
        &[
            "agent-integration",
            "status",
            project.to_str().unwrap(),
            "--json",
        ],
    );
    let status: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status["agent_integration_ready"], false);
    let plan = cli(
        &temporary.0,
        &["agent-integration", "plan", project.to_str().unwrap()],
    );
    assert!(!plan.status.success());
    assert!(String::from_utf8_lossy(&plan.stderr).contains("installation transition"));
}

#[test]
fn pending_recovery_paths_stay_out_of_portable_receipts() {
    let temporary = TempDir::new("agent-integration-private-diagnostic");
    let (project, pending) = initialize_with_pending_transaction(&temporary);
    assert_local_status_retains_recovery_path(&temporary, &project, &pending);

    let workspace = Workspace::discover(&project).expect("discover pending workspace");
    let onboarding = workspace.agent_integration_onboarding_status();
    assert!(!onboarding.agent_integration_ready);
    assert_eq!(onboarding.ready_scope, "unavailable");
    assert_private_diagnostic(&onboarding.diagnostic, &project);

    let validation = succeeds(&project, &["validate", "--json"]);
    let validation_json: Value = serde_json::from_slice(&validation.stdout).unwrap();
    assert_eq!(validation_json["valid"], true);
    assert_unavailable(&validation_json["agent_integration"], &project);
    assert_portable(&validation.stdout, &project);

    let handoff = succeeds(
        &project,
        &[
            "handoff",
            "create",
            "--id",
            "handoff-private-integration-diagnostic",
            "--generated-at",
            "2026-08-23T20:00:00Z",
        ],
    );
    let handoff_json: Value = serde_json::from_slice(&handoff.stdout).unwrap();
    assert_eq!(handoff_json["schema_version"], 2);
    assert_unavailable(
        &handoff_json["validation"]["result"]["agent_integration"],
        &project,
    );
    assert_portable(&handoff.stdout, &project);
    assert!(pending.is_dir(), "recovery evidence must remain intact");
}

#[cfg(unix)]
#[test]
fn invalid_reference_paths_abort_before_a_portable_handoff_exists() {
    use std::os::unix::fs::symlink;

    let temporary = TempDir::new("handoff-private-validation-errors");
    let project = temporary.0.join("private-project");
    succeeds(
        &temporary.0,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Portable validation privacy",
            "--without-review-authority",
        ],
    );
    let outside = temporary.0.join("private-outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, project.join("artifact-link")).unwrap();
    let experiment = serde_json::json!({
        "schema_version": 1, "kind": "experiment", "id": "experiment-private",
        "question": "Question?", "method_ref": "method.md", "observations": ["Observed"],
        "interpretation": "Interpretation", "limitations": ["Limited"],
        "outcome": "inconclusive", "next_move": "Repeat",
        "artifacts": [{"locator_type": "workspace", "locator": "artifact-link/result.txt", "description": "Result", "digest": null}]
    });
    fs::write(
        project.join(".research-run/experiments/experiment-private.json"),
        serde_json::to_vec_pretty(&experiment).unwrap(),
    )
    .unwrap();

    let local = cli(&project, &["validate", "--json"]);
    assert!(!local.status.success());
    assert!(String::from_utf8_lossy(&local.stdout).contains(&*project.to_string_lossy()));

    let handoff = cli(
        &project,
        &[
            "handoff",
            "create",
            "--id",
            "handoff-private-validation-errors",
            "--generated-at",
            "2026-08-24T03:00:00Z",
        ],
    );
    assert!(!handoff.status.success());
    assert!(
        handoff.stdout.is_empty(),
        "no portable artifact may be emitted"
    );
    assert_portable(&handoff.stderr, &project);
    assert!(!String::from_utf8_lossy(&handoff.stderr).contains(&*outside.to_string_lossy()));
    assert!(project.join("artifact-link").is_symlink());
}

fn initialize_with_pending_transaction(temporary: &TempDir) -> (PathBuf, PathBuf) {
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Private integration diagnostic",
            "--without-review-authority",
        ],
    );
    let pending = project.join(".AGENTS.md.1.1.txn");
    fs::create_dir(&pending).expect("create pending transaction evidence");
    (project, pending)
}

fn assert_local_status_retains_recovery_path(temporary: &TempDir, project: &Path, pending: &Path) {
    let explicit = cli(
        &temporary.0,
        &[
            "agent-integration",
            "status",
            project.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(explicit.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&explicit.stderr).contains(&*pending.to_string_lossy()));
}

fn assert_unavailable(status: &Value, project: &Path) {
    assert_eq!(status["agent_integration_ready"], false);
    assert_eq!(status["ready_scope"], "unavailable");
    assert_private_diagnostic(status["diagnostic"].as_str().unwrap(), project);
}

fn assert_private_diagnostic(diagnostic: &str, project: &Path) {
    assert_eq!(
        diagnostic,
        "Agent integration inspection is unavailable. Run 'research-run agent-integration status' locally for details and preserve any pending publication evidence."
    );
    assert!(!diagnostic.contains(&*project.to_string_lossy()));
}

fn assert_portable(output: &[u8], project: &Path) {
    let output = String::from_utf8_lossy(output);
    assert!(!output.contains(&*project.to_string_lossy()));
    if let Some(home) = std::env::var_os("HOME") {
        assert!(!output.contains(&*home.to_string_lossy()));
    }
}
