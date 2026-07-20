use super::*;
use crate::workspace::inventory_authority::{
    inventory_plan_review_authority, verified_inventory_plan_snapshot, verify_inventory_target,
};

#[test]
fn inventory_planning_propagates_each_authority_stage() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let workspace = Workspace::initialize(&root, "Project").expect("initialize");

    inject_storage_failure("load snapshot#2");
    assert!(
        Workspace::plan_retrofit(
            &root,
            "Project",
            "inventory-one",
            "2026-07-18T20:00:00Z",
            None,
        )
        .is_err()
    );

    inject_storage_failure("inventory plan authority");
    assert!(
        Workspace::plan_retrofit(
            &root,
            "Project",
            "inventory-one",
            "2026-07-18T20:00:00Z",
            None,
        )
        .is_err()
    );

    fs::remove_file(workspace.state.join("manifest.json")).expect("remove manifest");
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn existing_workspace_reconciliation_does_not_require_randomness() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let retrofit = Workspace::plan_retrofit(
        &root,
        "Project",
        "inventory-one",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("retrofit plan");
    Workspace::apply_inventory_plan(&root, retrofit).expect("apply retrofit");

    crate::domain::inject_random_failure();
    Workspace::plan_reconciliation(&root, "Project", "inventory-two", "2026-07-18T20:01:00Z")
        .expect("existing workspace reconciliation");
    assert!(
        ProjectManifest::new("Consume injected failure").is_err(),
        "reconciliation unexpectedly consumed secure randomness"
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_authority_helpers_reject_invalid_and_unreachable_shapes() {
    let root = temporary();
    let authority =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("authority");
    let workspace =
        Workspace::initialize_with_review_authority(&root, "Project", &authority).expect("init");
    let mut duplicate = workspace.load_snapshot().expect("snapshot");
    duplicate.review_authorities.push(authority.clone());
    assert!(inventory_plan_review_authority(Some(&duplicate), None).is_err());
    assert!(inventory_plan_review_authority(None, None).is_err());
    let plan = Workspace::plan_retrofit(
        &root,
        "Project",
        "inventory-one",
        "2026-07-18T20:00:00Z",
        None,
    )
    .expect("plan");
    assert!(verify_inventory_target(&workspace, &duplicate, &plan).is_err());

    fs::remove_file(workspace.state.join("review-authorities/test-human.json"))
        .expect("remove authority");
    assert!(verified_inventory_plan_snapshot(Some(&workspace)).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn legacy_unanchored_plan_is_verified_only_against_unanchored_state() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Project").expect("initialize");
    let snapshot = workspace.load_snapshot().expect("snapshot");
    let mut plan = Workspace::plan_retrofit(
        &root,
        "Project",
        "inventory-one",
        "2026-07-18T20:00:00Z",
        None,
    )
    .expect("plan");
    plan.without_review_authority = false;
    assert!(verify_inventory_target(&workspace, &snapshot, &plan).is_ok());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_initialization_rejects_invalid_key_and_workspace_identity() {
    let invalid = ReviewAuthority {
        schema_version: 2,
        kind: "review-authority".to_owned(),
        id: "test-human".to_owned(),
        public_key: test_review_public_key(),
        fingerprint: "SHA256:invalid".to_owned(),
    };
    let root = temporary();
    assert!(
        Workspace::initialize_for_inventory(&root, "Project", Some(&invalid), &"a".repeat(64))
            .is_err()
    );

    let mut malformed_key =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("authority");
    malformed_key.public_key = "not-an-openssh-key".to_owned();
    assert!(
        Workspace::initialize_for_inventory(
            &root,
            "Project",
            Some(&malformed_key),
            &"a".repeat(64),
        )
        .is_err()
    );
    assert!(Workspace::initialize_for_inventory(&root, "Project", None, "invalid").is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_apply_rejects_a_malformed_plan_authority_before_effects() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let mut plan = Workspace::plan_retrofit(
        &root,
        "Project",
        "inventory-one",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    let mut authority =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("authority");
    authority.public_key = "ssh-ed25519 malformed".to_owned();
    plan.review_authority = Some(authority);
    plan.without_review_authority = false;

    assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
    assert!(!root.join(".research-run").exists());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn identical_inventory_read_failure_propagates_from_apply() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = Workspace::plan_retrofit(
        &root,
        "Project",
        "inventory-one",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    Workspace::apply_inventory_plan(&root, plan.clone()).expect("seed inventory");
    inject_storage_failure("lock identity#2");
    let error =
        Workspace::apply_inventory_plan(&root, plan.clone()).expect_err("publication lock fault");
    assert!(
        error
            .to_string()
            .contains("workspace write lock identity changed during acquisition")
    );
    assert!(
        !error
            .to_string()
            .contains("inventory bootstrap for inventory-one")
    );
    inject_storage_failure("record identity#4");
    let error = Workspace::apply_inventory_plan(&root, plan).expect_err("inventory read fault");
    assert!(
        !error
            .to_string()
            .contains("inventory bootstrap for inventory-one")
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn empty_bootstrap_scaffold_is_retryable_after_pre_marker_failures() {
    for fault in ["lock identity", "create pending record"] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = Workspace::plan_retrofit(
            &root,
            "Project",
            "inventory-one",
            "2026-07-18T20:00:00Z",
            explicit_unanchored_retrofit(),
        )
        .expect("plan");

        inject_storage_failure(fault);
        assert!(Workspace::apply_inventory_plan(&root, plan.clone()).is_err());
        assert!(!root.join(".research-run/inventory-bootstrap.json").exists());
        Workspace::apply_inventory_plan(&root, plan).expect("exact retry");
        assert!(
            root.join(".research-run/inventories/inventory-one.json")
                .is_file()
        );
        assert!(!root.join(".research-run/inventory-bootstrap.json").exists());
        fs::remove_dir_all(root).expect("remove fixture");
    }
}
