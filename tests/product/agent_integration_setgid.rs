use std::fs;

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn setgid_project_root_supports_lossless_append_and_identical_retry() {
    use std::os::unix::fs::PermissionsExt;

    let temporary = TempDir::new("agent-integration-setgid");
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &[
            "init",
            project.to_str().unwrap(),
            "--name",
            "Setgid agent integration",
            "--without-review-authority",
        ],
    );
    let target = project.join("AGENTS.md");
    let original = b"# Existing shared-project instructions\n";
    fs::write(&target, original).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
    fs::set_permissions(&project, fs::Permissions::from_mode(0o2770)).unwrap();
    let root_mode = fs::metadata(&project).unwrap().permissions().mode() & 0o7777;
    assert_eq!(root_mode & 0o2000, 0o2000, "setgid precondition missing");

    let plan_path = temporary.0.join("setgid-append-plan.json");
    let plan_bytes = succeeds(
        &temporary.0,
        &["agent-integration", "plan", project.to_str().unwrap()],
    )
    .stdout;
    let plan: Value = serde_json::from_slice(&plan_bytes).unwrap();
    fs::write(&plan_path, plan_bytes).unwrap();
    let first = apply(&temporary, &project, &plan_path);
    assert_eq!(first["changed"], true);
    let installed = fs::read(&target).unwrap();
    assert!(installed.starts_with(original));
    assert_eq!(
        format!("{:x}", Sha256::digest(&installed)),
        plan["prospective_sha256"]
    );
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o7777,
        0o640
    );
    assert!(transaction_artifacts(&project).is_empty());

    let retry = apply(&temporary, &project, &plan_path);
    assert_eq!(retry["changed"], false);
    assert_eq!(fs::read(&target).unwrap(), installed);
    assert!(transaction_artifacts(&project).is_empty());
}

fn apply(temporary: &TempDir, project: &std::path::Path, plan: &std::path::Path) -> Value {
    let output = cli(
        &temporary.0,
        &[
            "agent-integration",
            "apply",
            project.to_str().unwrap(),
            "--input",
            plan.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        output.status.success(),
        "setgid Append failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn transaction_artifacts(project: &std::path::Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(project)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy();
            name.starts_with(".AGENTS.md.") && (name.ends_with(".txn") || name.ends_with(".done"))
        })
        .collect()
}
