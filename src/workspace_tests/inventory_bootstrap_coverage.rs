use super::*;
use crate::domain::InventoryPlan;

#[test]
fn inventory_application_requires_explicit_bootstrap_and_propagates_anchored_init_failure() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let mut unbound = Workspace::plan_retrofit(
        &root,
        "Project",
        "inventory-one",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    unbound.without_review_authority = false;
    assert!(Workspace::apply_inventory_plan(&root, unbound).is_err());
    fs::remove_dir_all(&root).expect("remove fixture");

    let anchored_root = temporary();
    fs::write(anchored_root.join("note.md"), b"note").expect("note");
    let authority =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("authority");
    let plan = Workspace::plan_retrofit(
        &anchored_root,
        "Project",
        "inventory-one",
        "2026-07-18T20:00:00Z",
        Some(ReviewBootstrap::Authority(authority)),
    )
    .expect("anchored plan");
    inject_storage_failure("create project directory");
    assert!(Workspace::apply_inventory_plan(&anchored_root, plan).is_err());
    fs::remove_dir_all(anchored_root).expect("remove fixture");
}

#[test]
fn inventory_apply_rejects_a_workspace_that_appeared_after_planning() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = unanchored_plan(&root, "Appeared");
    Workspace::initialize(&root, "Appeared").expect("intervening workspace");
    assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
    assert_no_inventory(&root);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_apply_rejects_authority_mode_and_key_changes_after_planning() {
    let unanchored_root = temporary();
    fs::write(unanchored_root.join("note.md"), b"note").expect("note");
    let (plan, authority) = anchored_plan(&unanchored_root, "Mode mismatch");
    Workspace::initialize_for_inventory(
        &unanchored_root,
        "Mode mismatch",
        None,
        &plan.workspace_id,
    )
    .expect("unanchored target");
    assert!(Workspace::apply_inventory_plan(&unanchored_root, plan).is_err());
    assert_no_inventory(&unanchored_root);
    fs::remove_dir_all(unanchored_root).expect("remove fixture");

    let key_root = temporary();
    fs::write(key_root.join("note.md"), b"note").expect("note");
    let (plan, _) = anchored_plan(&key_root, "Key mismatch");
    let other = ReviewAuthority::from_openssh(
        authority.id,
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGqNGVOiQVyevMNkyQjVnsutXzLd1cnernrPi1g9rmyQ",
    )
    .expect("other authority");
    Workspace::initialize_for_inventory(
        &key_root,
        "Key mismatch",
        Some(&other),
        &plan.workspace_id,
    )
    .expect("different anchored target");
    assert!(Workspace::apply_inventory_plan(&key_root, plan).is_err());
    assert_no_inventory(&key_root);
    fs::remove_dir_all(key_root).expect("remove fixture");
}

#[test]
fn inventory_apply_rejects_missing_or_corrupt_anchored_authority() {
    for corrupt in [false, true] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let (plan, authority) = anchored_plan(&root, "Damaged authority");
        let workspace = Workspace::initialize_for_inventory(
            &root,
            "Damaged authority",
            Some(&authority),
            &plan.workspace_id,
        )
        .expect("anchored target");
        let path = workspace.state.join("review-authorities/test-human.json");
        if corrupt {
            fs::write(path, b"{").expect("corrupt authority");
        } else {
            fs::remove_file(path).expect("remove authority");
        }
        assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
        assert_no_inventory(&root);
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn reconciliation_rejects_an_authority_replaced_after_planning() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let (initial, authority) = anchored_plan(&root, "Authority race");
    Workspace::apply_inventory_plan(&root, initial).expect("initial inventory");
    fs::write(root.join("later.md"), b"later").expect("later material");
    let plan = Workspace::plan_reconciliation(
        &root,
        "Authority race",
        "inventory-two",
        "2026-07-18T20:01:00Z",
    )
    .expect("reconciliation plan");
    assert_eq!(plan.review_authority.as_ref(), Some(&authority));

    let replacement = ReviewAuthority::from_openssh(
        authority.id,
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGqNGVOiQVyevMNkyQjVnsutXzLd1cnernrPi1g9rmyQ",
    )
    .expect("replacement authority");
    let state = root.join(".research-run");
    let mut manifest = Workspace::at_exact_root(&root)
        .expect("inspect workspace")
        .expect("workspace")
        .read_manifest()
        .expect("manifest");
    manifest.anchor_review_authority(&replacement.id, &replacement.fingerprint);
    fs::write(
        state.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
    )
    .expect("replace manifest anchor");
    fs::write(
        state.join("review-authorities/test-human.json"),
        serde_json::to_vec_pretty(&replacement).expect("serialize authority"),
    )
    .expect("replace authority");

    assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
    assert_eq!(
        fs::read_dir(state.join("inventories"))
            .expect("inventory directory")
            .count(),
        1
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn appeared_partial_workspace_is_rejected_before_initialization_effects() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let (plan, authority) = anchored_plan(&root, "Partial appeared");
    let state = root.join(".research-run");
    fs::create_dir(&state).expect("partial state");
    fs::write(state.join("write.lock"), b"").expect("existing lock");
    let mut other_manifest = ProjectManifest::new("Partial appeared").expect("manifest");
    other_manifest.anchor_review_authority(&authority.id, &authority.fingerprint);
    assert_ne!(other_manifest.workspace_id, plan.workspace_id);
    fs::write(
        state.join("manifest.json"),
        serde_json::to_vec_pretty(&other_manifest).expect("serialize manifest"),
    )
    .expect("partial manifest");
    let mut before = fs::read_dir(&state)
        .expect("state entries")
        .map(|entry| entry.expect("state entry").file_name())
        .collect::<Vec<_>>();
    before.sort();

    assert!(
        Workspace::initialize_for_inventory(
            &root,
            "Partial appeared",
            Some(&authority),
            &plan.workspace_id,
        )
        .is_err()
    );
    let mut after = fs::read_dir(&state)
        .expect("state entries")
        .map(|entry| entry.expect("state entry").file_name())
        .collect::<Vec<_>>();
    after.sort();
    assert_eq!(after, before);
    assert!(!state.join("review-authorities").exists());
    assert!(!state.join("inventories").exists());
    fs::remove_dir_all(root).expect("remove fixture");
}

fn unanchored_plan(root: &std::path::Path, name: &str) -> InventoryPlan {
    Workspace::plan_retrofit(
        root,
        name,
        "inventory-one",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("unanchored plan")
}

fn anchored_plan(root: &std::path::Path, name: &str) -> (InventoryPlan, ReviewAuthority) {
    let authority =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("authority");
    let plan = Workspace::plan_retrofit(
        root,
        name,
        "inventory-one",
        "2026-07-18T20:00:00Z",
        Some(ReviewBootstrap::Authority(authority.clone())),
    )
    .expect("anchored plan");
    (plan, authority)
}

fn assert_no_inventory(root: &std::path::Path) {
    assert!(
        fs::read_dir(root.join(".research-run/inventories"))
            .expect("inventory directory")
            .next()
            .is_none()
    );
}
