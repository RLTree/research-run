#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use super::agent_integration_defense::{apply, initialize, plan};
#[cfg(unix)]
use super::researcher_journey::{TempDir, cli};

#[cfg(unix)]
#[test]
fn multiply_linked_append_source_is_rejected_before_effect_and_retryable() {
    use std::os::unix::fs::MetadataExt;

    let temporary = TempDir::new("agent-integration-hard-link");
    let project = initialize(&temporary);
    let target = project.join("AGENTS.md");
    let alias = temporary.0.join("instruction-alias.md");
    let original = b"# Existing instructions\n";
    fs::write(&target, original).unwrap();
    let plan_path = plan(&temporary, &project, "hard-link-plan.json");
    let original_mode = fs::metadata(&target).unwrap().mode();
    fs::hard_link(&target, &alias).unwrap();

    let rejected = cli(
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
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("hard links"));
    assert_eq!(fs::read(&target).unwrap(), original);
    assert_eq!(fs::read(&alias).unwrap(), original);
    assert_eq!(fs::metadata(&target).unwrap().mode(), original_mode);
    assert!(!fs::read_dir(&project).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".AGENTS.md.")
    }));

    fs::remove_file(&alias).unwrap();
    assert_eq!(apply(&temporary, &project, &plan_path)["changed"], true);
    assert_eq!(apply(&temporary, &project, &plan_path)["changed"], false);
}
