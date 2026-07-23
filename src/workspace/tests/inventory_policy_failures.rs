use super::*;
use crate::domain::{
    InventoryLimits, InventoryPolicy, InventorySnapshot, MaterialClass, MaterialEntry,
};

fn constrained(maximum_file: u64, maximum_total: u64, maximum_entries: u64) -> InventoryPolicy {
    InventoryPolicy {
        schema_version: 1,
        kind: "inventory-policy".to_owned(),
        limits: InventoryLimits {
            max_entries: maximum_entries,
            max_file_bytes: maximum_file,
            max_total_bytes: maximum_total,
        },
        boundaries: Vec::new(),
    }
}

fn plan_with_policy(root: &std::path::Path, policy: InventoryPolicy) -> crate::Result<()> {
    Workspace::plan_retrofit_with_policy(
        root,
        "Budget fixture",
        "inventory-one",
        "2026-07-20T04:00:00Z",
        explicit_unanchored_retrofit(),
        Some(policy),
    )
    .map(|_| ())
}

#[test]
fn configured_file_aggregate_and_entry_exhaustion_fail_closed() {
    let file_root = temporary();
    fs::write(file_root.join("large.bin"), b"123").expect("large file");
    assert!(plan_with_policy(&file_root, constrained(2, 10, 10)).is_err());

    let aggregate_root = temporary();
    fs::write(aggregate_root.join("one.bin"), b"12").expect("first file");
    fs::write(aggregate_root.join("two.bin"), b"34").expect("second file");
    assert!(plan_with_policy(&aggregate_root, constrained(2, 3, 10)).is_err());

    let count_root = temporary();
    for name in ["one", "two", "three"] {
        fs::write(count_root.join(name), b"1").expect("counted file");
    }
    assert!(plan_with_policy(&count_root, constrained(1, 3, 2)).is_err());
    for root in [file_root, aggregate_root, count_root] {
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn malformed_and_missing_policy_inputs_fail_before_scanning() {
    let root = temporary();
    let malformed = root.join("malformed.json");
    fs::write(&malformed, b"{").expect("malformed policy");
    assert!(Workspace::read_inventory_policy(&malformed).is_err());
    assert!(Workspace::read_inventory_policy(&root.join("missing.json")).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn repeated_plans_are_byte_deterministic() {
    let root = temporary();
    Workspace::initialize(&root, "Deterministic project").expect("workspace");
    fs::write(root.join("note.md"), b"note").expect("note");
    let first = Workspace::plan_retrofit(
        &root,
        "Deterministic project",
        "inventory-one",
        "2026-07-20T04:00:00Z",
        None,
    )
    .expect("first plan");
    let second = Workspace::plan_retrofit(
        &root,
        "Deterministic project",
        "inventory-one",
        "2026-07-20T04:00:00Z",
        None,
    )
    .expect("second plan");
    assert_eq!(
        super::super::publication::canonical_json_bytes(&first),
        super::super::publication::canonical_json_bytes(&second)
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_publication_failure_leaves_no_partial_canonical_record() {
    let root = temporary();
    Workspace::initialize(&root, "Atomic inventory").expect("workspace");
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = Workspace::plan_retrofit(
        &root,
        "Atomic inventory",
        "inventory-one",
        "2026-07-20T04:00:00Z",
        None,
    )
    .expect("plan");
    inject_storage_failure("publish canonical record");
    assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
    let inventories = root.join(".research-run/inventories");
    assert!(
        !inventories.exists()
            || fs::read_dir(&inventories)
                .expect("inventory directory")
                .next()
                .is_none()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn twenty_thousand_entry_inventory_uses_the_bounded_atomic_record_path() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Large metadata").expect("workspace");
    let manifest = workspace.read_manifest().expect("manifest");
    let entries = (0..20_000)
        .map(|index| {
            MaterialEntry {
                path: format!("data/file-{index:05}.bin"),
                class: MaterialClass::Artifact,
                bytes: 0,
                sha256: "a".repeat(64),
            }
            .into()
        })
        .collect();
    let inventory = InventorySnapshot {
        schema_version: 1,
        kind: "inventory".to_owned(),
        id: "large-inventory".to_owned(),
        project_name: manifest.name,
        project_id: manifest.project_id,
        observed_at: "2026-07-20T04:00:00Z".to_owned(),
        previous_snapshot_id: None,
        policy: Some(InventoryPolicy::default()),
        entries,
        changes: Vec::new(),
    };
    inventory.validate().expect("large inventory validates");
    assert!(
        super::super::publication::canonical_json_bytes(&inventory).len() as u64 > MAX_RECORD_BYTES
    );
    workspace
        .publish_record("inventories", &inventory)
        .expect("large atomic publication");
    assert_eq!(
        workspace.latest_inventory().unwrap().unwrap().id,
        inventory.id
    );
    fs::remove_dir_all(root).expect("remove fixture");
}
