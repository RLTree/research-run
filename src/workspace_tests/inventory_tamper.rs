use super::*;

#[test]
fn inventory_apply_recomputes_semantic_changes_under_lock() {
    let root = temporary();
    fs::write(root.join("note.md"), b"initial").expect("initial material");
    let initial = Workspace::plan_retrofit(
        &root,
        "Tamper proof",
        "inventory-initial",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("initial plan");
    Workspace::apply_inventory_plan(&root, initial).expect("initial apply");
    fs::write(root.join("note.md"), b"changed").expect("changed material");
    let mut omitted = Workspace::plan_reconciliation(
        &root,
        "Tamper proof",
        "inventory-changed",
        "2026-07-18T20:01:00Z",
    )
    .expect("changed plan");
    assert!(!omitted.changes.is_empty());
    omitted.changes.clear();
    assert!(Workspace::apply_inventory_plan(&root, omitted).is_err());
    assert_eq!(
        Workspace::discover(&root)
            .expect("workspace")
            .latest_inventory()
            .expect("latest")
            .expect("inventory")
            .id,
        "inventory-initial"
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_apply_rejects_material_changes_after_planning() {
    let root = temporary();
    fs::write(root.join("note.md"), b"initial").expect("initial material");
    let plan = Workspace::plan_retrofit(
        &root,
        "Stale plan",
        "inventory-stale",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    fs::write(root.join("new.txt"), b"new material").expect("new material");
    let error = Workspace::apply_inventory_plan(&root, plan).expect_err("stale plan");
    assert!(error.to_string().contains("project materials changed"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_apply_propagates_locked_rescan_failure() {
    let root = temporary();
    fs::write(root.join("note.md"), b"initial").expect("initial material");
    let plan = Workspace::plan_retrofit(
        &root,
        "Locked scan",
        "inventory-locked",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    inject_storage_failure("read retrofit directory#2");
    assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_apply_rejects_an_under_lock_change() {
    let root = temporary();
    fs::write(root.join("note.md"), b"initial").expect("initial material");
    let plan = Workspace::plan_retrofit(
        &root,
        "Under lock change",
        "inventory-under-lock",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    inject_storage_failure("materials changed under lock");
    let error = Workspace::apply_inventory_plan(&root, plan).expect_err("under-lock change");
    assert!(error.to_string().contains("acquiring inventory authority"));
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_apply_rejects_genuine_ambiguous_moves() {
    let root = temporary();
    fs::write(root.join("first.txt"), b"same").expect("first");
    fs::write(root.join("second.txt"), b"same").expect("second");
    let initial = Workspace::plan_retrofit(
        &root,
        "Conflict plan",
        "inventory-initial",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("initial plan");
    Workspace::apply_inventory_plan(&root, initial).expect("initial apply");
    fs::remove_file(root.join("first.txt")).expect("remove first");
    fs::remove_file(root.join("second.txt")).expect("remove second");
    fs::write(root.join("third.txt"), b"same").expect("third");
    let conflict = Workspace::plan_reconciliation(
        &root,
        "Conflict plan",
        "inventory-conflict",
        "2026-07-18T20:01:00Z",
    )
    .expect("conflict plan");
    assert!(
        conflict
            .changes
            .iter()
            .any(|change| change.kind == crate::domain::ReconciliationKind::Conflict)
    );
    assert!(Workspace::apply_inventory_plan(&root, conflict).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}
