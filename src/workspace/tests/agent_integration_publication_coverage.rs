use std::ffi::OsStr;
use std::fs;

use crate::Error;
use crate::domain::AgentIntegrationOperation;

use super::*;
use crate::workspace::tests::temporary;
use crate::workspace::{Workspace, inject_storage_failure};

#[test]
fn create_post_effect_failures_and_pending_discovery_are_explicit() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    inject_storage_failure("open record directory for sync");
    let error = publish_instruction(&target, b"created", AgentIntegrationOperation::Create, b"")
        .expect_err("post-create sync failure");
    assert!(matches!(error, Error::AmbiguousEffect(_)));
    remove_artifacts(&root);
    fs::remove_file(&target).unwrap();

    inject_storage_failure("remove abandoned pending record");
    let error = publish_instruction(&target, b"created", AgentIntegrationOperation::Create, b"")
        .expect_err("post-create cleanup failure");
    assert!(matches!(error, Error::AmbiguousEffect(_)));
    remove_artifacts(&root);
    fs::remove_file(&target).unwrap();

    inject_storage_failure("inspect pending project instructions");
    assert!(ensure_no_instruction_pending(&target).is_err());
    fs::write(root.join(".AGENTS.md.1.3.tmp"), b"pending").unwrap();
    inject_storage_failure("inspect pending project instruction path");
    assert!(ensure_no_instruction_pending(&target).is_err());
    fs::remove_file(root.join(".AGENTS.md.1.3.tmp")).unwrap();
    fs::create_dir(root.join(".AGENTS.md.1.1.tmp")).unwrap();
    assert!(ensure_no_instruction_pending(&target).is_err());
    fs::remove_dir(root.join(".AGENTS.md.1.1.tmp")).unwrap();
    fs::write(root.join(".AGENTS.md.1.2.txn"), b"wrong shape").unwrap();
    assert!(ensure_no_instruction_pending(&target).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn non_utf8_transaction_artifact_names_are_not_claimed() {
    use std::os::unix::ffi::OsStrExt;

    let name = OsStr::from_bytes(b".AGENTS.md.1.\xff.txn");
    assert!(!is_instruction_transaction_name(name, "AGENTS.md"));
}

#[test]
fn legacy_pending_recovery_covers_append_and_conflicting_target_states() {
    let root = temporary();
    Workspace::initialize(&root, "Legacy pending coverage").unwrap();
    let target = root.join("AGENTS.md");
    let original = b"existing";
    fs::write(&target, original).unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();
    let prospective = super::super::agent_integration_content::compose_planned_instruction(
        original,
        &plan.managed_block,
        AgentIntegrationOperation::Append,
    )
    .unwrap();
    let pending = root.join(".AGENTS.md.2.1.tmp");
    fs::write(&pending, &prospective).unwrap();
    recover_instruction_pending(&target, &plan).unwrap();
    assert!(!pending.exists());

    fs::write(&pending, prospective).unwrap();
    fs::write(&target, b"conflict").unwrap();
    assert!(matches!(
        recover_instruction_pending(&target, &plan),
        Err(Error::Conflict(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn artifact_name_and_optional_target_parsers_cover_rejected_shapes() {
    assert!(!is_instruction_artifact_name(
        OsStr::new("other"),
        "AGENTS.md",
        ".tmp"
    ));
    for name in [
        ".AGENTS.md.no-dot.tmp",
        ".AGENTS.md..1.tmp",
        ".AGENTS.md.1..tmp",
        ".AGENTS.md.1.1.1.tmp",
        ".AGENTS.md.x.1.tmp",
        ".AGENTS.md.1.x.tmp",
    ] {
        assert!(!is_instruction_pending_name(OsStr::new(name), "AGENTS.md"));
    }
    let root = temporary();
    let target = root.join("AGENTS.md");
    assert!(read_optional_target(&target).unwrap().is_none());
    fs::create_dir(&target).unwrap();
    assert!(read_optional_target(&target).is_err());
    fs::remove_dir_all(root).unwrap();
}

fn remove_artifacts(root: &Path) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap().to_string_lossy().starts_with('.') {
            if path.is_dir() {
                fs::remove_dir_all(path).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
        }
    }
}
