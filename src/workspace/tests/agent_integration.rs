use std::fs;

use crate::domain::AgentIntegrationOperation;
use crate::workspace::AgentIntegrationStatus;

use super::super::agent_integration_content::{
    InstructionAssessment, assess_instruction, compose_instruction, managed_block,
};
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
    inject_storage_failure("inspect agent instruction");
    let instruction_failure = workspace.agent_integration_onboarding_status();
    assert!(instruction_failure.protocol_installed);
    assert!(instruction_failure.is_consistent());

    inject_storage_failure("read agent integration protocol digest");
    let protocol_digest_failure = workspace.agent_integration_onboarding_status();
    assert!(!protocol_digest_failure.protocol_installed);
    assert!(protocol_digest_failure.is_consistent());

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
        assert!(status.protocol_installed);
        fs::remove_file(root.join("AGENTS.md")).unwrap();
    }
    fs::remove_file(root.join(".research-run/contribution-protocols/agent-contribution.json"))
        .unwrap();
    let protocol_failure = workspace.agent_integration_onboarding_status();
    assert!(!protocol_failure.protocol_installed);
    assert!(protocol_failure.is_consistent());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn public_agent_integration_status_validator_rejects_cross_field_drift() {
    let status = AgentIntegrationStatus {
        protocol_installed: true,
        instruction_contract_installed: false,
        agent_integration_ready: false,
        ready_scope: "new-agent-run".to_owned(),
        instruction_path: "AGENTS.md".to_owned(),
        current_session_loaded: "unverified".to_owned(),
        fresh_session_required: false,
        diagnostic: "Plan and apply integration.".to_owned(),
    };
    assert!(status.is_consistent());
    let mut inconsistent = status.clone();
    inconsistent.agent_integration_ready = true;
    assert!(!inconsistent.is_consistent());

    let cases = [
        ("empty", String::new(), false),
        ("whitespace", "  \n".to_owned(), true),
        ("control", "\0".to_owned(), true),
        ("ascii maximum", "x".repeat(65_536), true),
        ("ascii overflow", "x".repeat(65_537), false),
        ("multibyte maximum", "é".repeat(65_536), true),
        ("multibyte overflow", "é".repeat(65_537), false),
        ("supplementary maximum", "🛡".repeat(65_536), true),
        ("supplementary overflow", "🛡".repeat(65_537), false),
    ];
    for (name, diagnostic, expected) in cases {
        let mut candidate = status.clone();
        candidate.diagnostic = diagnostic;
        assert_eq!(candidate.is_consistent(), expected, "{name}");
    }
}
