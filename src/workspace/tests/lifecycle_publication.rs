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
    inject_storage_failure("publish canonical record#2");
    assert!(Workspace::initialize(&fresh, "Protocol publication").is_err());

    let retry = temporary();
    Workspace::initialize(&retry, "Protocol retry").expect("initialize retry fixture");
    inject_storage_failure("inspect record#2");
    assert!(Workspace::initialize(&retry, "Protocol retry").is_err());
    fs::remove_dir_all(fresh).expect("remove fresh fixture");
    fs::remove_dir_all(retry).expect("remove retry fixture");
}

#[test]
fn conflicting_retry_cannot_expose_a_staged_authority() {
    let root = temporary();
    let authority =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("authority");
    let other = ReviewAuthority::from_openssh("other-human".to_owned(), &test_review_public_key())
        .expect("other authority");

    inject_storage_failure("publish canonical record#2");
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
