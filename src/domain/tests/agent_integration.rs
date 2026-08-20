use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan, digest};

fn plan(operation: AgentIntegrationOperation) -> AgentIntegrationPlan {
    let instruction_sha256 =
        (operation != AgentIntegrationOperation::Create).then(|| digest(b"instructions"));
    AgentIntegrationPlan {
        schema_version: 1,
        kind: "agent-integration-plan".to_owned(),
        project_id: "project-one".to_owned(),
        workspace_id: digest(b"workspace"),
        manifest_sha256: digest(b"manifest"),
        protocol_sha256: digest(b"protocol"),
        instruction_path: "AGENTS.md".to_owned(),
        instruction_sha256,
        instruction_bytes: usize::from(operation != AgentIntegrationOperation::Create) as u64,
        managed_block: "managed block".to_owned(),
        managed_block_sha256: digest(b"managed block"),
        prospective_sha256: digest(b"prospective"),
        operation,
        plan_sha256: String::new(),
    }
    .seal()
}

#[test]
fn agent_integration_plan_validates_each_digest_and_operation_boundary() {
    for operation in [
        AgentIntegrationOperation::Create,
        AgentIntegrationOperation::Append,
        AgentIntegrationOperation::NoOp,
    ] {
        plan(operation).validate().expect("valid operation");
    }
    let mut legacy = plan(AgentIntegrationOperation::Create);
    legacy.workspace_id.clear();
    legacy = legacy.seal();
    legacy
        .validate()
        .expect("legacy workspace identity may be empty");
    let cases: [fn(&mut AgentIntegrationPlan); 7] = [
        |value| value.workspace_id = "INVALID".to_owned(),
        |value| value.manifest_sha256 = "a".repeat(63),
        |value| value.protocol_sha256 = "A".repeat(64),
        |value| value.instruction_sha256 = Some("bad".to_owned()),
        |value| value.managed_block_sha256 = "bad".to_owned(),
        |value| value.prospective_sha256 = "bad".to_owned(),
        |value| value.plan_sha256 = "bad".to_owned(),
    ];
    for mutate in cases {
        let mut value = plan(AgentIntegrationOperation::Append);
        mutate(&mut value);
        assert!(value.validate().is_err());
    }
}

#[test]
fn agent_integration_plan_rejects_inconsistent_and_modified_content() {
    let mut create_with_file = plan(AgentIntegrationOperation::Create);
    create_with_file.instruction_sha256 = Some(digest(b"unexpected"));
    assert!(create_with_file.validate().is_err());

    let mut append_without_file = plan(AgentIntegrationOperation::Append);
    append_without_file.instruction_sha256 = None;
    assert!(append_without_file.validate().is_err());

    let mut create_with_bytes = plan(AgentIntegrationOperation::Create);
    create_with_bytes.instruction_bytes = 1;
    assert!(create_with_bytes.validate().is_err());

    let mut modified_block = plan(AgentIntegrationOperation::Append);
    modified_block.managed_block_sha256 = digest(b"different block");
    assert!(modified_block.validate().is_err());

    let mut modified_plan = plan(AgentIntegrationOperation::Append);
    modified_plan.project_id = "project-two".to_owned();
    assert!(modified_plan.validate().is_err());
}
