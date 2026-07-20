use super::*;
use crate::workspace::status_authority::review_authority_status_from_manifest;

#[test]
fn mixed_review_authority_identity_fails_closed_in_status() {
    let root = temporary();
    let mut manifest = Workspace::initialize(&root, "Mixed authority status")
        .expect("workspace")
        .read_manifest()
        .expect("manifest");
    manifest.review_authority_id = Some("test-human".to_owned());
    let missing_fingerprint = review_authority_status_from_manifest(&manifest);
    assert_eq!(missing_fingerprint.mode, "identity-conflict");
    assert!(!missing_fingerprint.promotion_capable);

    manifest.review_authority_id = None;
    manifest.review_authority_fingerprint = Some("SHA256:test".to_owned());
    let missing_id = review_authority_status_from_manifest(&manifest);
    assert_eq!(missing_id.mode, "identity-conflict");
    assert!(!missing_id.promotion_capable);
    fs::remove_dir_all(root).expect("remove fixture");
}
