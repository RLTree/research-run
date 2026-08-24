use std::fs;

use crate::domain::{AgentIntegrationOperation, AgentIntegrationPlan};
use crate::{Error, workspace::Workspace};

use super::super::super::agent_integration_content::compose_planned_instruction;
use super::super::super::agent_integration_publication::publish_instruction;
use super::super::super::{inject_storage_failure, tests::temporary};

#[test]
fn every_post_exchange_cleanup_fault_retains_retry_authority() {
    for point in [
        "sync installed project instructions after exchange",
        "sync exchanged project instruction witness after exchange",
        "inspect installed project instruction after exchange",
        "create project instruction completion receipt",
        "write project instruction completion receipt",
        "sync project instruction completion receipt",
        "sync project instruction transaction after completion staging",
        "publish project instruction completion receipt",
        "sync published project instruction completion receipt",
        "sync project instruction root after completion receipt",
        "remove project instruction witness",
        "remove project instruction witness#2",
        "remove project instruction witness#3",
        "remove project instruction witness#4",
        "remove project instruction witness#5",
        "sync project instruction transaction during cleanup",
        "remove project instruction transaction",
        "sync project instruction root after cleanup",
        "remove project instruction completion receipt",
        "sync project instruction root after receipt cleanup",
    ] {
        let (root, plan, original, planned) = fixture(point);
        inject_storage_failure(point);

        let error = publish_instruction(
            &root.join("AGENTS.md"),
            &planned,
            AgentIntegrationOperation::Append,
            &original,
            &plan.plan_sha256,
        )
        .expect_err(point);

        assert!(
            matches!(error, Error::AmbiguousEffect(_)),
            "{point}: {error:?}"
        );
        assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), planned);
        if point != "sync project instruction root after receipt cleanup" {
            assert!(!completion_artifacts(&root).is_empty(), "{point}");
        }

        let result = Workspace::apply_agent_integration(&root, plan).unwrap();
        assert!(!result.changed, "{point}");
        assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), planned);
        assert!(completion_artifacts(&root).is_empty(), "{point}");
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn recovery_redurabilizes_the_receipt_before_deleting_any_witness() {
    for point in [
        "open committed project instruction root",
        "sync recovered project instruction completion receipt",
        "sync project instruction root before recovered cleanup",
    ] {
        let (root, plan, original, planned) = fixture(point);
        inject_storage_failure("sync project instruction root after completion receipt");
        assert!(
            publish_instruction(
                &root.join("AGENTS.md"),
                &planned,
                AgentIntegrationOperation::Append,
                &original,
                &plan.plan_sha256,
            )
            .is_err()
        );
        inject_storage_failure(point);
        let result = Workspace::apply_agent_integration(&root, plan.clone());
        assert!(
            matches!(result, Err(Error::AmbiguousEffect(_))),
            "{point}: {result:?}"
        );
        assert_eq!(transaction_names(&root).len(), 5, "{point}");

        assert!(
            !Workspace::apply_agent_integration(&root, plan)
                .unwrap()
                .changed
        );
        assert!(completion_artifacts(&root).is_empty());
        fs::remove_dir_all(root).unwrap();
    }
}

fn fixture(name: &str) -> (std::path::PathBuf, AgentIntegrationPlan, Vec<u8>, Vec<u8>) {
    let root = temporary();
    Workspace::initialize(&root, name).unwrap();
    let original = b"existing instructions\n".to_vec();
    fs::write(root.join("AGENTS.md"), &original).unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let planned = compose_planned_instruction(
        &original,
        &plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    (root, plan, original, planned)
}

fn completion_artifacts(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy();
            name.starts_with(".AGENTS.md.") && (name.ends_with(".txn") || name.ends_with(".done"))
        })
        .collect()
}

fn transaction_names(root: &std::path::Path) -> Vec<String> {
    let transaction = completion_artifacts(root)
        .into_iter()
        .find(|path| path.extension().is_some_and(|extension| extension == "txn"))
        .unwrap();
    fs::read_dir(transaction)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}
