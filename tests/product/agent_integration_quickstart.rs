use std::fs;
use std::path::Path;

use serde_json::Value;

use super::researcher_journey::{TempDir, succeeds};
use crate::review_test_signing::initialize_with_test_authority;

#[test]
fn source_checkout_quickstart_keeps_the_reviewed_plan_outside_the_workspace() {
    assert_documented_commands();
    let temporary = TempDir::new("agent-integration-quickstart");
    let project = temporary.0.join("my-study");
    initialize_with_test_authority(&temporary.0, &project, "My study");
    let plan_directory = temporary.0.join("research-run-plans");
    fs::create_dir(&plan_directory).expect("create external reviewed-plan directory");
    let plan_path = plan_directory.join("agent-integration.json");
    let plan = succeeds(&project, &["agent-integration", "plan", "."]);
    fs::write(&plan_path, plan.stdout).expect("write external reviewed plan");

    let applied = json_command(
        &project,
        &[
            "agent-integration",
            "apply",
            ".",
            "--input",
            "../research-run-plans/agent-integration.json",
            "--json",
        ],
    );
    assert_eq!(applied["ready_for_new_agent_run"], true);
    assert_eq!(applied["current_session_loaded"], "unverified");
    assert_eq!(applied["fresh_session_required"], true);

    let status = json_command(&project, &["agent-integration", "status", ".", "--json"]);
    assert_eq!(status["agent_integration_ready"], true);
    assert_eq!(status["ready_scope"], "new-agent-run");
    assert_eq!(status["current_session_loaded"], "unverified");
    assert!(plan_path.is_file());
    assert!(!project.join("research-run-plans").exists());
}

fn assert_documented_commands() {
    let readme = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"))
        .expect("read source-checkout quickstart");
    let documented_integration = concat!(
        "mkdir -p ../research-run-plans\n",
        "research-run agent-integration plan . > ../research-run-plans/agent-integration.json\n",
        "research-run agent-integration apply . \\\n",
        "  --input ../research-run-plans/agent-integration.json --json\n",
        "research-run agent-integration status . --json",
    );
    assert!(readme.contains(documented_integration));
    assert!(!readme.contains("agent-integration plan my-study >"));
    assert!(!readme.contains("agent-integration apply my-study"));
}

fn json_command(cwd: &Path, args: &[&str]) -> Value {
    serde_json::from_slice(&succeeds(cwd, args).stdout).expect("command JSON")
}
