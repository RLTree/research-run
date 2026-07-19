use std::fs;

use serde_json::Value;

use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn retrofit_preserves_bytes_and_reconcile_records_a_move() {
    let temporary = TempDir::new("retrofit");
    let project = temporary.0.join("populated-project");
    fs::create_dir_all(project.join("notes")).expect("notes directory");
    fs::create_dir_all(project.join("protocols")).expect("protocol directory");
    let note = project.join("notes/result.md");
    let protocol = project.join("protocols/assay.md");
    fs::write(&note, b"negative result\n").expect("write note");
    fs::write(&protocol, b"safe synthetic protocol\n").expect("write protocol");
    apply_and_repeat_retrofit(&temporary, &project, &note, &protocol);
    let moved = project.join("notes/negative-result.md");
    fs::rename(&note, &moved).expect("move note");
    assert_move_plan(&temporary, &project);
}

fn apply_and_repeat_retrofit(
    temporary: &TempDir,
    project: &std::path::Path,
    note: &std::path::Path,
    protocol: &std::path::Path,
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
        ],
    );
    let plan: Value = serde_json::from_slice(&plan_output.stdout).expect("plan JSON");
    assert_eq!(plan["entries"].as_array().expect("entries").len(), 2);
    let plan_path = temporary.0.join("retrofit-plan.json");
    fs::write(&plan_path, &plan_output.stdout).expect("write plan");

    succeeds(
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
            ],
        );
        assert!(!linked.status.success());
        assert!(String::from_utf8_lossy(&linked.stderr).contains("symlink is forbidden"));
    }
}
