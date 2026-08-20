use crate::Error;
use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan, digest};

type PlanMutation = (fn(&mut AgentIntegrationPlan), &'static str, bool);

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
    let cases: [PlanMutation; 7] = [
        (
            |value| value.workspace_id = "INVALID".to_owned(),
            "agent integration workspace_id",
            true,
        ),
        (
            |value| value.manifest_sha256 = "a".repeat(63),
            "agent integration manifest digest",
            true,
        ),
        (
            |value| value.protocol_sha256 = "A".repeat(64),
            "agent integration protocol digest",
            true,
        ),
        (
            |value| value.instruction_sha256 = Some("bad".to_owned()),
            "agent integration instruction digest",
            true,
        ),
        (
            |value| value.managed_block_sha256 = "bad".to_owned(),
            "agent integration managed block digest",
            true,
        ),
        (
            |value| value.prospective_sha256 = "bad".to_owned(),
            "agent integration prospective digest",
            true,
        ),
        (
            |value| value.plan_sha256 = "bad".to_owned(),
            "agent integration plan digest",
            false,
        ),
    ];
    for (mutate, context, reseal) in cases {
        let mut value = plan(AgentIntegrationOperation::Append);
        mutate(&mut value);
        if reseal {
            value = value.seal();
        }
        assert_invalid_context(value, context);
    }
}

#[test]
fn agent_integration_plan_rejects_inconsistent_and_modified_content() {
    let mut create_with_file = plan(AgentIntegrationOperation::Create);
    create_with_file.instruction_sha256 = Some(digest(b"unexpected"));
    assert_invalid_context(create_with_file.seal(), "agent integration operation");

    let mut append_without_file = plan(AgentIntegrationOperation::Append);
    append_without_file.instruction_sha256 = None;
    assert_invalid_context(append_without_file.seal(), "agent integration operation");

    let mut create_with_bytes = plan(AgentIntegrationOperation::Create);
    create_with_bytes.instruction_bytes = 1;
    assert_invalid_context(
        create_with_bytes.seal(),
        "agent integration instruction identity",
    );

    let mut modified_block = plan(AgentIntegrationOperation::Append);
    modified_block.managed_block_sha256 = digest(b"different block");
    assert_invalid_context(modified_block.seal(), "agent integration managed block");

    let mut modified_plan = plan(AgentIntegrationOperation::Append);
    modified_plan.project_id = "project-two".to_owned();
    assert_invalid_context(modified_plan, "agent integration plan");
}

fn assert_invalid_context(plan: AgentIntegrationPlan, expected: &str) {
    match plan.validate() {
        Err(Error::Invalid { context, .. }) => assert_eq!(context, expected),
        result => panic!("expected invalid {expected}, found {result:?}"),
    }
}
