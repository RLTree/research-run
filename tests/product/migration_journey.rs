use std::collections::BTreeMap;
use std::fs;

use serde_json::Value;

use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn accepted_v01_workspace_migrates_without_rewriting_authority() {
    let temporary = TempDir::new("migration");
    let project = legacy_workspace(&temporary);
    let before = canonical_bytes(&project);
    let plan = succeeds(
        &temporary.0,
        &[
            "migrate",
            "plan",
            project.to_str().unwrap(),
            "--id",
            "migration-v01",
            "--migrated-at",
            "2026-07-18T21:00:00Z",
        ],
    );
    let plan_path = temporary.0.join("migration-plan.json");
    fs::write(&plan_path, &plan.stdout).expect("migration plan");
    let applied = succeeds(
        &temporary.0,
        &[
            "migrate",
            "apply",
            project.to_str().unwrap(),
            "--input",
            plan_path.to_str().unwrap(),
            "--json",
        ],
    );
    let applied: Value = serde_json::from_slice(&applied.stdout).expect("apply JSON");
    assert_eq!(applied["created"], true);
    for (path, bytes) in before {
        assert_eq!(fs::read(path).expect("authority after migration"), bytes);
    }
    for directory in ["inventories", "knowledge", "relationships", "migrations"] {
        assert!(project.join(".research-run").join(directory).is_dir());
    }
    let repeated = succeeds(
        &temporary.0,
        &[
            "migrate",
            "apply",
            project.to_str().unwrap(),
            "--input",
            plan_path.to_str().unwrap(),
            "--json",
        ],
    );
    let repeated: Value = serde_json::from_slice(&repeated.stdout).expect("repeat JSON");
    assert_eq!(repeated["created"], false);
    let human = succeeds(
        &temporary.0,
        &[
            "migrate",
            "apply",
            project.to_str().unwrap(),
            "--input",
            plan_path.to_str().unwrap(),
        ],
    );
    assert!(String::from_utf8_lossy(&human.stdout).contains("Already present"));
}

#[test]
fn migration_rejects_stale_authority_before_effect() {
    let temporary = TempDir::new("migration-defense");
    let project = legacy_workspace(&temporary);
    let plan = succeeds(
        &temporary.0,
        &[
            "migrate",
            "plan",
            project.to_str().unwrap(),
            "--id",
            "migration-stale",
            "--migrated-at",
            "2026-07-18T21:00:00Z",
        ],
    );
    let plan_path = temporary.0.join("migration-plan.json");
    fs::write(&plan_path, &plan.stdout).expect("migration plan");
    fs::write(
        project.join(".research-run/sources/source-one.json"),
        b"{}\n",
    )
    .unwrap();
    let output = cli(
        &temporary.0,
        &[
            "migrate",
            "apply",
            project.to_str().unwrap(),
            "--input",
            plan_path.to_str().unwrap(),
        ],
    );
    assert!(!output.status.success());
    assert!(!project.join(".research-run/migrations").exists());
}

fn legacy_workspace(temporary: &TempDir) -> std::path::PathBuf {
    let project = temporary.0.join("project");
    succeeds(
        &temporary.0,
        &["init", project.to_str().unwrap(), "--name", "Legacy"],
    );
    succeeds(
        &project,
        &[
            "source",
            "add",
            "--id",
            "source-one",
            "--citation",
            "Example source",
            "--locator",
            "doi:example",
            "--provenance",
            "human",
        ],
    );
    for directory in ["inventories", "knowledge", "relationships", "migrations"] {
        fs::remove_dir(project.join(".research-run").join(directory)).expect("legacy shape");
    }
    project
}

fn canonical_bytes(project: &std::path::Path) -> BTreeMap<std::path::PathBuf, Vec<u8>> {
    let state = project.join(".research-run");
    let mut files = Vec::new();
    collect_json(&state, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).expect("canonical bytes");
            (path, bytes)
        })
        .collect()
}

fn collect_json(directory: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("authority directory") {
        let path = entry.expect("authority entry").path();
        if path.is_dir() {
            collect_json(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            files.push(path);
        }
    }
}
