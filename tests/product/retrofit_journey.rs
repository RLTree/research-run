use std::fs;

use serde_json::Value;

use super::researcher_journey::{TempDir, cli, succeeds};
use super::review_test_signing::write_test_public_key;

#[test]
fn retrofit_preserves_bytes_and_reconcile_records_a_move() {
    let temporary = TempDir::new("retrofit");
    let project = temporary.0.join("populated-project");
    fs::create_dir_all(project.join("notes")).expect("notes directory");
    fs::create_dir_all(project.join("protocols")).expect("protocol directory");
    let note = project.join("notes/result.md");
    let protocol = project.join("protocols/assay.md");
    let instructions = project.join("AGENTS.md");
    fs::write(&note, b"negative result\n").expect("write note");
    fs::write(&protocol, b"safe synthetic protocol\n").expect("write protocol");
    fs::write(&instructions, b"# Existing agent law\n").expect("write instructions");
    let public_key = write_test_public_key(&temporary.0);
    apply_and_repeat_retrofit(&temporary, &project, &note, &protocol, &public_key);
    assert_eq!(
        fs::read(&instructions).expect("instructions after retrofit"),
        b"# Existing agent law\n"
    );
    let moved = project.join("notes/negative-result.md");
    fs::rename(&note, &moved).expect("move note");
    assert_move_plan(&temporary, &project);
}

fn apply_and_repeat_retrofit(
    temporary: &TempDir,
    project: &std::path::Path,
    note: &std::path::Path,
    protocol: &std::path::Path,
    public_key: &std::path::Path,
) {
    let before_note = fs::read(note).expect("read note");
    let before_protocol = fs::read(protocol).expect("read protocol");
    let plan_output = succeeds(
        &temporary.0,
        &[
            "retrofit",
            "plan",
            project.to_str().expect("project path"),
            "--name",
            "Populated project",
            "--id",
            "inventory-initial",
            "--observed-at",
            "2026-07-18T20:00:00Z",
            "--review-authority-id",
            "test-human",
            "--review-authority-public-key",
            public_key.to_str().expect("public key path"),
        ],
    );
    let plan: Value = serde_json::from_slice(&plan_output.stdout).expect("plan JSON");
    assert_eq!(plan["entries"].as_array().expect("entries").len(), 3);
    assert_eq!(plan["review_authority"]["id"], "test-human");
    let plan_path = temporary.0.join("retrofit-plan.json");
    fs::write(&plan_path, &plan_output.stdout).expect("write plan");

    let applied = succeeds(
        &temporary.0,
        &[
            "retrofit",
            "apply",
            project.to_str().expect("project path"),
            "--input",
            plan_path.to_str().expect("plan path"),
            "--json",
        ],
    );
    let applied: Value = serde_json::from_slice(&applied.stdout).expect("apply JSON");
    assert_eq!(applied["review_authority"]["mode"], "anchored");
    assert_eq!(applied["review_authority"]["promotion_capable"], true);
    assert_retrofit_onboarding(&applied);
    assert!(
        project
            .join(".research-run/contribution-protocols/agent-contribution.json")
            .is_file()
    );
    assert_eq!(fs::read(note).expect("note after"), before_note);
    assert_eq!(fs::read(protocol).expect("protocol after"), before_protocol);
    let repeated = succeeds(
        &temporary.0,
        &[
            "retrofit",
            "apply",
            project.to_str().expect("project path"),
            "--input",
            plan_path.to_str().expect("plan path"),
            "--json",
        ],
    );
    let repeated: Value = serde_json::from_slice(&repeated.stdout).expect("repeat JSON");
    assert_eq!(repeated["created"], false);
    let human = succeeds(
        &temporary.0,
        &[
            "retrofit",
            "apply",
            project.to_str().expect("project path"),
            "--input",
            plan_path.to_str().expect("plan path"),
        ],
    );
    assert!(String::from_utf8_lossy(&human.stdout).contains("Already present"));
}

fn assert_retrofit_onboarding(applied: &Value) {
    assert_eq!(
        applied["agent_integration"]["agent_integration_ready"],
        false
    );
}

fn assert_move_plan(temporary: &TempDir, project: &std::path::Path) {
    let reconcile_output = succeeds(
        &temporary.0,
        &[
            "reconcile",
            "plan",
            project.to_str().expect("project path"),
            "--name",
            "Populated project",
            "--id",
            "inventory-second",
            "--observed-at",
            "2026-07-18T20:01:00Z",
        ],
    );
    let reconcile: Value =
        serde_json::from_slice(&reconcile_output.stdout).expect("reconcile JSON");
    assert!(
        reconcile["changes"]
            .as_array()
            .expect("changes")
            .iter()
            .any(|change| {
                change["kind"] == "moved"
                    && change["before"] == "notes/result.md"
                    && change["after"] == "notes/negative-result.md"
            })
    );
    let plan_path = temporary.0.join("reconcile-plan.json");
    fs::write(&plan_path, &reconcile_output.stdout).expect("reconcile plan");
    let wrong_command = cli(
        &temporary.0,
        &[
            "retrofit",
            "apply",
            project.to_str().expect("project path"),
            "--input",
            plan_path.to_str().expect("plan path"),
        ],
    );
    assert!(!wrong_command.status.success());
    succeeds(
        &temporary.0,
        &[
            "reconcile",
            "apply",
            project.to_str().expect("project path"),
            "--input",
            plan_path.to_str().expect("plan path"),
            "--json",
        ],
    );
    let listed = succeeds(project, &["list", "--kind", "material", "--limit", "10"]);
    let listed: Value = serde_json::from_slice(&listed.stdout).expect("material list JSON");
    let materials = listed["items"].as_array().expect("material items");
    assert!(
        materials
            .iter()
            .any(|item| { item["id"] == "notes/result.md" && item["stale"] == true })
    );
    assert!(
        materials
            .iter()
            .any(|item| { item["id"] == "notes/negative-result.md" && item["stale"] == false })
    );
}

#[test]
fn retrofit_fails_closed_on_stale_plan_and_symlink() {
    let temporary = TempDir::new("retrofit-defense");
    let project = temporary.0.join("project");
    fs::create_dir(&project).expect("project directory");
    fs::write(project.join("notes.md"), b"first\n").expect("note");
    let plan = succeeds(
        &temporary.0,
        &[
            "retrofit",
            "plan",
            project.to_str().expect("project path"),
            "--name",
            "Defense project",
            "--id",
            "inventory-defense",
            "--observed-at",
            "2026-07-18T20:00:00Z",
            "--without-review-authority",
        ],
    );
    let plan_path = temporary.0.join("plan.json");
    fs::write(&plan_path, &plan.stdout).expect("plan file");
    fs::write(project.join("notes.md"), b"changed\n").expect("change note");
    let stale = cli(
        &temporary.0,
        &[
            "retrofit",
            "apply",
            project.to_str().expect("project path"),
            "--input",
            plan_path.to_str().expect("plan path"),
        ],
    );
    assert!(!stale.status.success());
    assert!(!project.join(".research-run").exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(&plan_path, project.join("linked-plan")).expect("symlink");
        let linked = cli(
            &temporary.0,
            &[
                "retrofit",
                "plan",
                project.to_str().expect("project path"),
                "--name",
                "Defense project",
                "--id",
                "inventory-linked",
                "--observed-at",
                "2026-07-18T20:00:00Z",
                "--without-review-authority",
            ],
        );
        assert!(!linked.status.success());
        assert!(String::from_utf8_lossy(&linked.stderr).contains("symlink is forbidden"));
    }
}
