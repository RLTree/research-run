use std::fs;

use crate::Error;
use crate::domain::{AgentIntegrationOperation, digest};

use super::super::agent_integration_content::compose_planned_instruction;
use super::super::agent_integration_publication::publish_instruction;
use super::super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::{Workspace, inject_storage_failure, temporary};

const TEST_PLAN_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[test]
fn instruction_size_budget_accepts_the_boundary_and_rejects_one_byte_over() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    let boundary = vec![b'a'; MAX_INSTRUCTION_BYTES as usize];
    assert!(
        !publish_instruction(
            &target,
            &boundary,
            AgentIntegrationOperation::NoOp,
            b"",
            TEST_PLAN_SHA256,
        )
        .unwrap()
    );
    let over = vec![b'a'; MAX_INSTRUCTION_BYTES as usize + 1];
    assert!(
        publish_instruction(
            &target,
            &over,
            AgentIntegrationOperation::NoOp,
            b"",
            TEST_PLAN_SHA256,
        )
        .is_err()
    );

    Workspace::initialize(&root, "Instruction budget").unwrap();
    let missing_plan = Workspace::plan_agent_integration(&root).unwrap();
    let prospective_len = |length| {
        let current = vec![b'a'; length];
        compose_planned_instruction(
            &current,
            &missing_plan.managed_block,
            AgentIntegrationOperation::Append,
        )
        .unwrap()
        .len()
    };
    let maximum = MAX_INSTRUCTION_BYTES as usize;
    let mut largest_fitting = 0;
    let mut first_rejected = maximum + 1;
    while largest_fitting + 1 < first_rejected {
        let length = largest_fitting + (first_rejected - largest_fitting) / 2;
        if prospective_len(length) <= maximum {
            largest_fitting = length;
        } else {
            first_rejected = length;
        }
    }
    assert_eq!(first_rejected, largest_fitting + 1);
    assert!(prospective_len(largest_fitting) <= maximum);
    assert!(prospective_len(first_rejected) > maximum);

    let largest = vec![b'a'; largest_fitting];
    fs::write(&target, &largest).unwrap();
    assert_eq!(
        Workspace::plan_agent_integration(&root)
            .unwrap()
            .instruction_bytes,
        largest.len() as u64
    );
    let next = vec![b'a'; first_rejected];
    fs::write(&target, next).unwrap();
    assert!(Workspace::plan_agent_integration(&root).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pending_instruction_recovery_is_plan_bound_and_retryable() {
    let root = temporary();
    Workspace::initialize(&root, "Pending recovery").unwrap();
    let unrelated = root.join(".AGENTS.md.race.1.tmp");
    fs::write(&unrelated, b"not a Research Run pending shape").unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let prospective =
        compose_planned_instruction(b"", &plan.managed_block, AgentIntegrationOperation::Create)
            .unwrap();
    let pending = root.join(".AGENTS.md.123.1.tmp");
    fs::write(&pending, &prospective).unwrap();
    let workspace = Workspace::for_recovery(&root).unwrap();
    assert!(workspace.agent_integration_status().is_err());
    assert!(Workspace::plan_agent_integration(&root).is_err());

    inject_storage_failure("remove abandoned pending record");
    assert!(Workspace::apply_agent_integration(&root, plan.clone()).is_err());
    assert!(pending.exists());
    inject_storage_failure("sync abandoned pending directory");
    assert!(Workspace::apply_agent_integration(&root, plan.clone()).is_err());
    assert!(!pending.exists());

    fs::write(&pending, &prospective).unwrap();
    assert!(
        Workspace::apply_agent_integration(&root, plan.clone())
            .unwrap()
            .changed
    );
    assert!(!pending.exists());
    assert!(unrelated.exists());
    assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), prospective);
    assert!(
        !Workspace::apply_agent_integration(&root, plan)
            .unwrap()
            .changed
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn completed_create_alias_and_append_rename_reach_identical_retry() {
    let root = temporary();
    Workspace::initialize(&root, "Completed create recovery").unwrap();
    let create_plan = Workspace::plan_agent_integration(&root).unwrap();
    let create_bytes = compose_planned_instruction(
        b"",
        &create_plan.managed_block,
        AgentIntegrationOperation::Create,
    )
    .unwrap();
    let create_pending = root.join(".AGENTS.md.456.2.tmp");
    fs::write(&create_pending, create_bytes).unwrap();
    fs::hard_link(&create_pending, root.join("AGENTS.md")).unwrap();
    assert!(
        !Workspace::apply_agent_integration(&root, create_plan)
            .unwrap()
            .changed
    );
    assert!(!create_pending.exists());

    fs::remove_dir_all(&root).unwrap();
    fs::create_dir(&root).unwrap();
    Workspace::initialize(&root, "Completed append recovery").unwrap();
    let original = b"# Existing\n";
    fs::write(root.join("AGENTS.md"), original).unwrap();
    let append_plan = Workspace::plan_agent_integration(&root).unwrap();
    let append_bytes = compose_planned_instruction(
        original,
        &append_plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    let append_pending = root.join(".AGENTS.md.456.3.tmp");
    fs::write(&append_pending, append_bytes).unwrap();
    fs::rename(&append_pending, root.join("AGENTS.md")).unwrap();
    assert!(
        !Workspace::apply_agent_integration(&root, append_plan)
            .unwrap()
            .changed
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pending_instruction_mismatch_multiplicity_and_symlink_fail_closed() {
    let root = temporary();
    Workspace::initialize(&root, "Pending conflicts").unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let first = root.join(".AGENTS.md.789.3.tmp");
    fs::write(&first, b"mismatch").unwrap();
    assert!(Workspace::apply_agent_integration(&root, plan.clone()).is_err());
    assert!(first.exists());
    fs::remove_file(&first).unwrap();

    let prospective =
        compose_planned_instruction(b"", &plan.managed_block, AgentIntegrationOperation::Create)
            .unwrap();
    fs::write(&first, &prospective).unwrap();
    let second = root.join(".AGENTS.md.789.4.tmp");
    fs::write(&second, &prospective).unwrap();
    assert!(Workspace::apply_agent_integration(&root, plan.clone()).is_err());
    assert!(first.exists() && second.exists());
    fs::remove_file(&first).unwrap();
    fs::remove_file(&second).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = root.join("outside");
        fs::write(&outside, prospective).unwrap();
        symlink(&outside, &first).unwrap();
        assert!(Workspace::apply_agent_integration(&root, plan).is_err());
        assert!(first.exists());
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn installed_retry_rejects_resealed_transition_tamper() {
    let root = temporary();
    Workspace::initialize(&root, "Retry tamper").unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    Workspace::apply_agent_integration(&root, plan.clone()).unwrap();
    let no_op = Workspace::plan_agent_integration(&root).unwrap();
    assert_eq!(no_op.operation, AgentIntegrationOperation::NoOp);
    assert!(
        !Workspace::apply_agent_integration(&root, no_op)
            .unwrap()
            .changed
    );

    let mut altered_operation = plan;
    altered_operation.operation = AgentIntegrationOperation::Append;
    altered_operation.instruction_sha256 = Some(digest(b""));
    altered_operation = altered_operation.seal();
    assert!(matches!(
        Workspace::apply_agent_integration(&root, altered_operation),
        Err(Error::Conflict(_))
    ));

    fs::remove_dir_all(&root).unwrap();
    fs::create_dir(&root).unwrap();
    Workspace::initialize(&root, "Retry append tamper").unwrap();
    let original = b"# Existing\n";
    fs::write(root.join("AGENTS.md"), original).unwrap();
    let append = Workspace::plan_agent_integration(&root).unwrap();
    Workspace::apply_agent_integration(&root, append.clone()).unwrap();

    let mut altered_digest = append.clone();
    altered_digest.instruction_sha256 = Some(digest(b"# Changed\n"));
    altered_digest = altered_digest.seal();
    assert!(matches!(
        Workspace::apply_agent_integration(&root, altered_digest),
        Err(Error::Conflict(_))
    ));

    let mut altered_bytes = append;
    altered_bytes.instruction_bytes -= 1;
    altered_bytes.instruction_sha256 = Some(digest(&original[..original.len() - 1]));
    altered_bytes = altered_bytes.seal();
    assert!(matches!(
        Workspace::apply_agent_integration(&root, altered_bytes),
        Err(Error::Conflict(_))
    ));
    fs::remove_dir_all(root).unwrap();
}
