use super::*;
use crate::domain::{InventorySnapshot, MigrationRecord};
use crate::workspace::recovery_commit::discard_identical;
use crate::workspace::recovery_optional::preflight_optional;
use crate::workspace::recovery_plan::PendingRecord;
use crate::workspace::recovery_semantics::validate_recovered_authority;
use crate::workspace::storage::ReadBudget;

#[test]
fn optional_preflight_and_identical_discard_propagate_owned_reads() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Recovery propagation").expect("initialize");
    inject_storage_failure("read recovery directory");
    assert!(preflight_optional(&workspace, &mut ReadBudget::default()).is_err());

    let target = workspace.state.join("sources/source-one.json");
    fs::write(&target, b"{}").expect("target");
    let record = PendingRecord {
        path: workspace.state.join("sources/.source-one.json.1.1.tmp"),
        target,
        bytes: b"{}".to_vec(),
    };
    inject_storage_failure("inspect record");
    assert!(
        discard_identical(
            record,
            &mut crate::workspace::RecoveryResult {
                recovered: Vec::new(),
                discarded_identical: Vec::new(),
            },
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn recovery_batch_propagates_optional_collection_failure() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Optional recovery").expect("initialize");
    inject_storage_failure("read recovery directory#8");
    assert!(workspace.preflight_recovery_batch().is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn recovered_semantics_propagate_scan_and_migration_authority_failures() {
    let root = temporary();
    fs::write(root.join("material.txt"), b"material").expect("material");
    let inventory = Workspace::plan_retrofit(
        &root,
        "Semantic faults",
        "inventory-one",
        "2026-07-20T00:00:00Z",
    )
    .expect("inventory plan");
    let workspace = Workspace::initialize(&root, "Semantic faults").expect("initialize");
    let mut inventory_snapshot = workspace.load_snapshot().expect("snapshot");
    inventory_snapshot
        .inventories
        .push(InventorySnapshot::from(inventory));
    let inventory_pending = [PendingRecord {
        path: workspace
            .state
            .join("inventories/.inventory-one.json.1.1.tmp"),
        target: workspace.state.join("inventories/inventory-one.json"),
        bytes: Vec::new(),
    }];
    let empty: &[PendingRecord] = &[];
    inject_storage_failure("inspect workspace path");
    assert!(
        validate_recovered_authority(
            &workspace,
            &inventory_snapshot,
            &inventory_pending,
            empty,
            [empty; 10],
        )
        .is_err()
    );

    let plan =
        Workspace::plan_migration(&root, "migration-one", "2026-07-20T00:01:00Z").expect("plan");
    let migration = MigrationRecord::from(plan);
    let mut migration_snapshot = workspace.load_snapshot().expect("snapshot");
    migration_snapshot.migrations.push(migration);
    let migration_pending = [PendingRecord {
        path: workspace
            .state
            .join("migrations/.migration-one.json.1.1.tmp"),
        target: workspace.state.join("migrations/migration-one.json"),
        bytes: Vec::new(),
    }];
    inject_storage_failure("inspect record");
    assert!(
        validate_recovered_authority(
            &workspace,
            &migration_snapshot,
            empty,
            &migration_pending,
            [empty; 10],
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn migration_target_ids_and_missing_candidates_fail_closed() {
    use std::os::unix::ffi::OsStringExt;

    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration target").expect("initialize");
    let mut snapshot = workspace.load_snapshot().expect("snapshot");
    snapshot.migrations.push(MigrationRecord {
        schema_version: 1,
        kind: "migration".to_owned(),
        id: "other-migration".to_owned(),
        project_id: snapshot.manifest.project_id.clone(),
        from_format: "research-run-v0.1".to_owned(),
        to_format: "research-run-v0.1-extended".to_owned(),
        migrated_at: "2026-07-20T00:00:00Z".to_owned(),
        authority_files: 1,
        authority_bytes: 1,
        authority_sha256: "a".repeat(64),
    });
    let empty: &[PendingRecord] = &[];
    let missing = [PendingRecord {
        path: workspace
            .state
            .join("migrations/.migration-one.json.1.1.tmp"),
        target: workspace.state.join("migrations/migration-one.json"),
        bytes: Vec::new(),
    }];
    assert!(
        validate_recovered_authority(&workspace, &snapshot, empty, &missing, [empty; 10]).is_err()
    );

    let mut name = std::ffi::OsString::from_vec(vec![0xff]);
    name.push(".json");
    let invalid = [PendingRecord {
        path: workspace.state.join("migrations/.invalid.json.1.1.tmp"),
        target: workspace.state.join("migrations").join(name),
        bytes: Vec::new(),
    }];
    assert!(
        validate_recovered_authority(&workspace, &snapshot, empty, &invalid, [empty; 10]).is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}
