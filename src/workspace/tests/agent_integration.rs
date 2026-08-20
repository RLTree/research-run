use std::fs;

use crate::domain::AgentIntegrationOperation;

use super::super::agent_integration_content::{
    InstructionAssessment, assess_instruction, compose_instruction, managed_block,
};
use super::super::agent_integration_publication::publish_instruction;
use super::{Workspace, inject_storage_failure, temporary};

#[test]
fn instruction_content_covers_empty_markers_newlines_and_legacy_identity() {
    assert!(matches!(
        assess_instruction(b"", true, "expected").unwrap(),
        InstructionAssessment::Unmanaged
    ));
    assert!(matches!(
        assess_instruction(
            b"<!-- research-run:agent-integration:v1:start -->",
            true,
            "expected"
        )
        .unwrap(),
        InstructionAssessment::Conflict(_)
    ));
    assert_eq!(
        compose_instruction(b"existing", "managed\n", AgentIntegrationOperation::Append).unwrap(),
        b"existing\n\nmanaged\n"
    );

    let root = temporary();
    let workspace = Workspace::initialize(&root, "Legacy block").unwrap();
    let mut snapshot = workspace.load_snapshot().unwrap();
    snapshot.manifest.workspace_id.clear();
    let block = managed_block(&snapshot, &"a".repeat(64), &"b".repeat(64));
    assert!(block.contains("workspace `legacy-unanchored`"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn agent_status_and_apply_propagate_owned_boundary_failures() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Integration failures").unwrap();
    for point in [
        "read agent integration manifest digest",
        "read agent integration protocol digest",
        "inspect agent instruction",
    ] {
        inject_storage_failure(point);
        assert!(workspace.agent_integration_status().is_err(), "{point}");
    }

    let plan = Workspace::plan_agent_integration(&root).unwrap();
    inject_storage_failure("agent instruction drift after plan");
    assert!(Workspace::apply_agent_integration(&root, plan).is_err());
    fs::remove_file(root.join("AGENTS.md")).unwrap();

    let plan = Workspace::plan_agent_integration(&root).unwrap();
    inject_storage_failure("create pending record");
    assert!(Workspace::apply_agent_integration(&root, plan).is_err());

    fs::create_dir(root.join("AGENTS.md")).unwrap();
    assert!(workspace.agent_integration_status().is_err());
    fs::remove_dir(root.join("AGENTS.md")).unwrap();
    fs::write(
        root.join("AGENTS.md"),
        "<!-- research-run:agent-integration:v1:start -->\nmodified\n<!-- research-run:agent-integration:v1:end -->\n",
    )
    .unwrap();
    assert!(
        !workspace
            .agent_integration_status()
            .unwrap()
            .agent_integration_ready
    );
    fs::remove_file(root.join("AGENTS.md")).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(root.join("outside"), root.join("AGENTS.md")).unwrap();
        let status = workspace.agent_integration_onboarding_status();
        assert_eq!(status.ready_scope, "unavailable");
        fs::remove_file(root.join("AGENTS.md")).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn instruction_publication_covers_noop_races_and_atomic_failures() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    assert!(!publish_instruction(&target, b"same", AgentIntegrationOperation::NoOp).unwrap());

    fs::write(&target, b"same").unwrap();
    assert!(!publish_instruction(&target, b"same", AgentIntegrationOperation::Create).unwrap());
    assert!(publish_instruction(&target, b"different", AgentIntegrationOperation::Create).is_err());
    assert!(
        publish_instruction(&target, b"replacement", AgentIntegrationOperation::Append).unwrap()
    );

    for point in [
        "create pending record",
        "inspect project instruction permissions",
        "replace project instructions",
        "open record directory for sync",
    ] {
        fs::write(&target, b"before").unwrap();
        inject_storage_failure(point);
        assert!(
            publish_instruction(&target, b"after", AgentIntegrationOperation::Append).is_err(),
            "{point}"
        );
        remove_pending(&root);
    }
    fs::remove_file(&target).unwrap();
    inject_storage_failure("create project instructions");
    assert!(publish_instruction(&target, b"new", AgentIntegrationOperation::Create).is_err());
    remove_pending(&root);
    fs::remove_dir_all(root).unwrap();
}

fn remove_pending(root: &std::path::Path) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with(".AGENTS.md.")
        {
            fs::remove_file(path).unwrap();
        }
    }
}
