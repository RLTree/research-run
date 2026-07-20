use super::*;

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
