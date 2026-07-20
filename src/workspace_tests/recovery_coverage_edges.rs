use super::*;
use crate::domain::{InventorySnapshot, MigrationRecord};
use crate::workspace::recovery_plan::PendingRecord;
use crate::workspace::recovery_semantics::validate_recovered_authority;

#[test]
fn recovery_rejects_batches_with_multiple_new_semantic_authorities() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Recovery batch").expect("initialize");
    let snapshot = workspace.load_snapshot().expect("snapshot");
    let pending = |id: &str| PendingRecord {
        path: workspace.state.join(format!(".{id}.json.1.1.tmp")),
        target: workspace.state.join(format!("{id}.json")),
        bytes: Vec::new(),
    };
    let inventory = [pending("inventory-one"), pending("inventory-two")];
    let empty: &[PendingRecord] = &[];
    assert!(
        validate_recovered_authority(&workspace, &snapshot, &inventory, empty, [empty; 10])
            .is_err()
    );
    let migration = [pending("migration-one"), pending("migration-two")];
    assert!(
        validate_recovered_authority(&workspace, &snapshot, empty, &migration, [empty; 10])
            .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn recovered_inventory_rechecks_prior_authority_and_missing_candidates() {
    let root = temporary();
    fs::write(root.join("material.txt"), b"one").expect("material");
    let first = Workspace::plan_retrofit(
        &root,
        "Inventory history",
        "inventory-one",
        "2026-07-20T00:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("first plan");
    Workspace::apply_inventory_plan(&root, first).expect("first inventory");
    fs::write(root.join("material.txt"), b"two").expect("change material");
    let second = Workspace::plan_reconciliation(
        &root,
        "Inventory history",
        "inventory-two",
        "2026-07-20T00:01:00Z",
    )
    .expect("second plan");
    Workspace::apply_inventory_plan(&root, second).expect("second inventory");
    fs::write(root.join("material.txt"), b"three").expect("change material again");
    let third = Workspace::plan_reconciliation(
        &root,
        "Inventory history",
        "inventory-three",
        "2026-07-20T00:02:00Z",
    )
    .expect("third plan");
    let workspace = Workspace::discover(&root).expect("workspace");
    let mut snapshot = workspace.load_snapshot().expect("snapshot");
    snapshot.inventories.push(InventorySnapshot::from(third));
    let pending = [PendingRecord {
        path: workspace
            .state
            .join("inventories/.inventory-three.json.1.1.tmp"),
        target: workspace.state.join("inventories/inventory-three.json"),
        bytes: Vec::new(),
    }];
    let empty: &[PendingRecord] = &[];
    validate_recovered_authority(&workspace, &snapshot, &pending, empty, [empty; 10])
        .expect("current inventory authority");

    let missing = [PendingRecord {
        path: workspace.state.join("inventories/.missing.json.1.1.tmp"),
        target: workspace.state.join("inventories/missing.json"),
        bytes: Vec::new(),
    }];
    assert!(
        validate_recovered_authority(
            &workspace,
            &workspace.load_snapshot().expect("snapshot"),
            &missing,
            empty,
            [empty; 10],
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn migration_recovery_rejects_other_new_effects() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration isolation").expect("initialize");
    let mut snapshot = workspace.load_snapshot().expect("snapshot");
    snapshot.migrations.push(MigrationRecord {
        schema_version: 1,
        kind: "migration".to_owned(),
        id: "migration-one".to_owned(),
        project_id: snapshot.manifest.project_id.clone(),
        from_format: "research-run-v0.1".to_owned(),
        to_format: "research-run-v0.1-extended".to_owned(),
        migrated_at: "2026-07-20T00:00:00Z".to_owned(),
        authority_files: 1,
        authority_bytes: 1,
        authority_sha256: "a".repeat(64),
    });
    let migration = [PendingRecord {
        path: workspace
            .state
            .join("migrations/.migration-one.json.1.1.tmp"),
        target: workspace.state.join("migrations/migration-one.json"),
        bytes: Vec::new(),
    }];
    let other = [PendingRecord {
        path: workspace.state.join("sources/.source-one.json.1.1.tmp"),
        target: workspace.state.join("sources/source-one.json"),
        bytes: Vec::new(),
    }];
    let empty: &[PendingRecord] = &[];
    assert!(
        validate_recovered_authority(
            &workspace,
            &snapshot,
            empty,
            &migration,
            [
                &other, empty, empty, empty, empty, empty, empty, empty, empty, empty
            ],
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn recovery_target_ids_reject_non_utf8_names() {
    use std::os::unix::ffi::OsStringExt;

    let root = temporary();
    let workspace = Workspace::initialize(&root, "Non UTF-8 recovery").expect("initialize");
    let mut name = std::ffi::OsString::from_vec(vec![0xff]);
    name.push(".json");
    let pending = [PendingRecord {
        path: workspace.state.join("inventories/.invalid.json.1.1.tmp"),
        target: workspace.state.join("inventories").join(name),
        bytes: Vec::new(),
    }];
    let empty: &[PendingRecord] = &[];
    assert!(
        validate_recovered_authority(
            &workspace,
            &workspace.load_snapshot().expect("snapshot"),
            &pending,
            empty,
            [empty; 10],
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}
