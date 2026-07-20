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
