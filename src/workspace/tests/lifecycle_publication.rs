use super::*;

#[test]
fn authority_publication_failure_never_exposes_a_manifest() {
    let root = temporary();
    let authority =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("authority");

    inject_storage_failure("publish canonical record");
    assert!(
        Workspace::initialize_with_review_authority(&root, "Authority failure", &authority)
            .is_err()
    );
    assert!(!root.join(".research-run/manifest.json").exists());
    assert!(
        !root
            .join(".research-run/review-authorities/test-human.json")
            .exists()
    );
    assert!(
        Workspace::discover(&root).is_err(),
        "failed authority publication became discoverable"
    );

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn contribution_protocol_publication_failures_propagate_on_init_and_retry() {
    let fresh = temporary();
    inject_storage_failure("create pending record#2");
    assert!(Workspace::initialize(&fresh, "Protocol publication").is_err());
    assert!(
        !fresh.join(".research-run/manifest.json").exists(),
        "failed activation publication exposed a discoverable workspace"
    );

    let retry = temporary();
    Workspace::initialize(&retry, "Protocol retry").expect("initialize retry fixture");
    inject_storage_failure("inspect record#2");
    assert!(Workspace::initialize(&retry, "Protocol retry").is_err());
    fs::remove_dir_all(fresh).expect("remove fresh fixture");
    fs::remove_dir_all(retry).expect("remove retry fixture");
}

#[test]
fn fresh_activation_interruption_leaves_one_recoverable_batch() {
    let root = temporary();

    inject_storage_failure("publish recovered record#2");
    assert!(Workspace::initialize(&root, "Recoverable activation").is_err());
    assert!(!root.join(".research-run/manifest.json").exists());
    assert!(
        root.join(".research-run/contribution-protocols/agent-contribution.json")
            .is_file(),
        "activation protocol must precede the discoverable manifest"
    );

    let workspace = Workspace::for_recovery(&root).expect("partial workspace");
    workspace.recover().expect("recover activation batch");
    Workspace::discover(&root)
        .expect("discover recovered workspace")
        .load_snapshot()
        .expect("validate recovered activation");

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn existing_legacy_workspace_requires_typed_migration_for_activation() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Legacy init").expect("initialize");
    let protocol = workspace
        .state
        .join("contribution-protocols/agent-contribution.json");
    fs::remove_file(&protocol).expect("remove activation protocol");

    assert!(Workspace::initialize(&root, "Legacy init").is_err());
    assert!(!protocol.exists(), "init bypassed typed migration");
    assert!(
        !workspace
            .state
            .join("migrations")
            .read_dir()
            .unwrap()
            .any(|_| true),
        "init created an undocumented migration"
    );

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn malformed_protocol_blocks_authority_recovery_before_publication() {
    let root = temporary();
    let authority =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("authority");
    let workspace =
        Workspace::initialize_with_review_authority(&root, "Malformed activation", &authority)
            .expect("initialize");
    let authority_path = workspace.state.join("review-authorities/test-human.json");
    fs::remove_file(&authority_path).expect("remove authority");
    fs::write(
        workspace
            .state
            .join("contribution-protocols/agent-contribution.json"),
        b"{",
    )
    .expect("corrupt activation protocol");

    assert!(
        Workspace::initialize_with_review_authority(&root, "Malformed activation", &authority)
            .is_err()
    );
    assert!(
        !authority_path.exists(),
        "authority was published before protocol validation"
    );

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn existing_invalid_workspace_is_not_activated_before_validation() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Legacy activation").expect("initialize");
    let protocol = workspace
        .state
        .join("contribution-protocols/agent-contribution.json");
    fs::remove_file(&protocol).expect("remove protocol");
    fs::write(workspace.state.join("inventories/bad.json"), b"{}").expect("bad inventory");

    assert!(Workspace::initialize(&root, "Legacy activation").is_err());
    assert!(
        !protocol.exists(),
        "invalid canonical state was mutated before validation"
    );

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn conflicting_retry_cannot_expose_a_staged_authority() {
    let root = temporary();
    let authority =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("authority");
    let other = ReviewAuthority::from_openssh("other-human".to_owned(), &test_review_public_key())
        .expect("other authority");

    inject_storage_failure("publish recovered record");
    assert!(Workspace::initialize_with_review_authority(&root, "Retry", &authority).is_err());
    assert!(
        root.join(".research-run/review-authorities/test-human.json")
            .is_file()
    );
    assert!(!root.join(".research-run/manifest.json").exists());

    assert!(Workspace::initialize(&root, "Unanchored retry").is_err());
    assert!(!root.join(".research-run/manifest.json").exists());
    assert!(Workspace::discover(&root).is_err());

    assert!(Workspace::initialize_with_review_authority(&root, "Other", &other).is_err());
    assert!(
        !root
            .join(".research-run/review-authorities/other-human.json")
            .exists()
    );
    assert!(!root.join(".research-run/manifest.json").exists());
    assert!(Workspace::discover(&root).is_err());

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn authority_state_read_failures_preserve_the_discovery_boundary() {
    let staged_root = temporary();
    inject_storage_failure("read record directory#2");
    assert!(Workspace::initialize(&staged_root, "Staged read failure").is_err());
    assert!(!staged_root.join(".research-run/manifest.json").exists());
    Workspace::for_recovery(&staged_root)
        .expect("partial staged workspace")
        .recover()
        .expect("recover staged read failure");
    Workspace::initialize(&staged_root, "Staged read failure").expect("staged read retry");

    let new_root = temporary();
    inject_storage_failure("load snapshot");
    assert!(Workspace::initialize(&new_root, "Final read failure").is_err());
    assert!(new_root.join(".research-run/manifest.json").is_file());
    Workspace::initialize(&new_root, "Final read failure").expect("final read retry");

    inject_storage_failure("load snapshot");
    assert!(Workspace::initialize(&new_root, "Final read failure").is_err());
    assert!(new_root.join(".research-run/manifest.json").is_file());

    fs::remove_dir_all(staged_root).expect("remove staged fixture");
    fs::remove_dir_all(new_root).expect("remove final fixture");
}
