use std::fs;

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
