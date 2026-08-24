use std::fs;

use crate::Error;
use crate::domain::{InventoryLimits, InventoryPolicy, InventorySnapshot};
use crate::workspace::Workspace;
use crate::workspace::inject_storage_failure;
use crate::workspace::tests::temporary;

use super::*;

#[cfg(unix)]
#[test]
fn non_utf8_transaction_artifact_names_are_not_claimed() {
    use std::os::unix::ffi::OsStrExt;

    let name = std::ffi::OsStr::from_bytes(b".AGENTS.md.1.\xff.txn");
    assert!(instruction_artifact_kind(name, ".AGENTS.md.").is_none());
}

#[test]
fn pending_instruction_scan_has_a_deterministic_entry_budget() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    fs::write(&target, b"instructions").unwrap();
    inject_storage_failure("project instruction root entry budget");

    let error = ensure_no_instruction_pending(&target, default_instruction_scan_budget().unwrap())
        .expect_err("ordinary root entries must consume the pending scan budget");

    assert!(matches!(error, Error::Budget(_)), "{error:?}");
    assert_eq!(fs::read(&target).unwrap(), b"instructions");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pending_scan_budget_propagates_truthfully_through_plan_apply_and_status() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Pending scan propagation").unwrap();
    let plan = Workspace::plan_agent_integration(&root).unwrap();

    inject_storage_failure("project instruction root entry budget");
    assert!(matches!(
        Workspace::plan_agent_integration(&root),
        Err(Error::Budget(_))
    ));
    inject_storage_failure("project instruction root entry budget");
    assert!(matches!(
        Workspace::apply_agent_integration(&root, plan),
        Err(Error::AmbiguousEffect(_))
    ));
    assert!(!root.join("AGENTS.md").exists());
    inject_storage_failure("project instruction root entry budget");
    assert!(matches!(
        workspace.agent_integration_status(),
        Err(Error::Budget(_))
    ));
    inject_storage_failure("project instruction root entry budget");
    let onboarding = workspace.agent_integration_onboarding_status();
    assert!(!onboarding.agent_integration_ready);
    assert_eq!(onboarding.ready_scope, "unavailable");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn root_entry_budget_uses_the_default_floor_allowance_and_checked_growth() {
    assert_eq!(scan_budget_from_max_entries(0).unwrap(), 20_256);
    assert_eq!(
        scan_budget_from_max_entries(InventoryLimits::legacy().max_entries).unwrap(),
        20_256
    );
    assert_eq!(scan_budget_from_max_entries(100_000).unwrap(), 100_256);
    assert!(matches!(
        scan_budget_from_max_entries(u64::MAX),
        Err(Error::Budget(_))
    ));
}

#[test]
fn root_entry_budget_follows_the_latest_accepted_inventory_policy() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Pending scan policy").unwrap();
    let mut snapshot = workspace.load_snapshot().unwrap();
    snapshot
        .inventories
        .push(inventory("inventory-old", "2026-08-22T00:00:00Z", 100_000));
    snapshot
        .inventories
        .push(inventory("inventory-new", "2026-08-23T00:00:00Z", 50_000));

    assert_eq!(instruction_scan_budget(&snapshot).unwrap(), 50_256);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn pending_scan_accepts_the_boundary_and_rejects_the_next_root_entry() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    fs::write(&target, b"instructions").unwrap();
    fs::write(root.join("unrelated"), b"one").unwrap();

    assert!(instruction_pending(&target, 2).unwrap().is_empty());
    fs::write(root.join("another"), b"two").unwrap();
    assert!(matches!(
        instruction_pending(&target, 2),
        Err(Error::Budget(_))
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn third_recognized_artifact_fails_closed_without_removing_evidence() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    fs::write(&target, b"instructions").unwrap();
    let artifacts = [
        root.join(".AGENTS.md.1.1.tmp"),
        root.join(".AGENTS.md.1.2.done"),
        root.join(".AGENTS.md.1.3.txn"),
    ];
    fs::write(&artifacts[0], b"pending").unwrap();
    fs::write(&artifacts[1], b"completion").unwrap();
    fs::create_dir(&artifacts[2]).unwrap();

    assert!(matches!(
        instruction_pending(&target, 10),
        Err(Error::AmbiguousEffect(_))
    ));
    assert!(artifacts.iter().all(|path| path.exists()));
    fs::remove_dir_all(root).unwrap();
}

fn inventory(id: &str, observed_at: &str, max_entries: u64) -> InventorySnapshot {
    let mut policy = InventoryPolicy::default();
    policy.limits.max_entries = max_entries;
    InventorySnapshot {
        schema_version: 1,
        kind: "inventory".to_owned(),
        id: id.to_owned(),
        project_name: "Pending scan policy".to_owned(),
        project_id: "project-pending-scan".to_owned(),
        observed_at: observed_at.to_owned(),
        previous_snapshot_id: None,
        policy: Some(policy),
        entries: Vec::new(),
        changes: Vec::new(),
    }
}
