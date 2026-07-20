use std::path::Path;
use std::sync::OnceLock;

use base64ct::{Base64, Encoding};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256, Sha512};

use crate::domain::{
    REVIEW_SIGNATURE_NAMESPACE, ReviewAuthority, ReviewAuthorization, ReviewDecision, ReviewRequest,
};
use crate::workspace::Workspace;
use crate::workspace::publication::canonical_json_bytes;

static REVIEW_KEY: OnceLock<SigningKey> = OnceLock::new();

pub(super) fn authorize_review(workspace: &Workspace, review: &mut ReviewDecision) {
    let snapshot = workspace.load_snapshot().expect("authority snapshot");
    assert_eq!(
        snapshot.review_authorities.len(),
        1,
        "review authority must be anchored during initialization"
    );
    let authority = &snapshot.review_authorities[0];
    let request = ReviewRequest {
        schema_version: review.schema_version,
        kind: "review-request".to_owned(),
        project_id: snapshot.manifest.project_id,
        workspace_id: snapshot.manifest.workspace_id,
        id: review.id.clone(),
        claim_id: review.claim_id.clone(),
        evidence_ids: review.evidence_ids.clone(),
        subject_sha256: review
            .subject_sha256
            .clone()
            .expect("authorized test review has a subject digest"),
        decision: review.decision,
        rationale: review.rationale.clone(),
        reviewer: review.reviewer.clone(),
    };
    review.authorization = Some(ReviewAuthorization {
        authority_id: authority.id.clone(),
        signer_fingerprint: authority.fingerprint.clone(),
        signature: sign_test_review(&canonical_json_bytes(&request)),
    });
}

pub(super) fn publish_review(
    workspace: &Workspace,
    review: &mut ReviewDecision,
) -> crate::Result<bool> {
    authorize_review(workspace, review);
    workspace.add_review(review)
}

pub(super) fn review_workspace(root: &Path, name: &str) -> Workspace {
    let authority =
        ReviewAuthority::from_openssh("test-human".to_owned(), &test_review_public_key())
            .expect("test authority");
    Workspace::initialize_with_review_authority(root, name, &authority)
        .expect("initialize review workspace")
}

pub(super) fn test_review_public_key() -> String {
    format!(
        "ssh-ed25519 {}",
        Base64::encode_string(&test_review_public_key_blob())
    )
}

fn test_review_key() -> &'static SigningKey {
    REVIEW_KEY.get_or_init(|| {
        let seed: [u8; 32] = Sha256::digest(b"research-run-unit-test-key").into();
        SigningKey::from_bytes(&seed)
    })
}

fn test_review_public_key_blob() -> Vec<u8> {
    let mut blob = Vec::new();
    put_ssh_string(&mut blob, b"ssh-ed25519");
    put_ssh_string(&mut blob, test_review_key().verifying_key().as_bytes());
    blob
}

fn sign_test_review(message: &[u8]) -> String {
    let digest = Sha512::digest(message);
    let mut signed = b"SSHSIG".to_vec();
    put_ssh_string(&mut signed, REVIEW_SIGNATURE_NAMESPACE.as_bytes());
    put_ssh_string(&mut signed, b"");
    put_ssh_string(&mut signed, b"sha512");
    put_ssh_string(&mut signed, &digest);
    let signature = test_review_key().sign(&signed);
    let mut signature_blob = Vec::new();
    put_ssh_string(&mut signature_blob, b"ssh-ed25519");
    put_ssh_string(&mut signature_blob, &signature.to_bytes());
    let mut binary = b"SSHSIG".to_vec();
    binary.extend_from_slice(&1_u32.to_be_bytes());
    put_ssh_string(&mut binary, &test_review_public_key_blob());
    put_ssh_string(&mut binary, REVIEW_SIGNATURE_NAMESPACE.as_bytes());
    put_ssh_string(&mut binary, b"");
    put_ssh_string(&mut binary, b"sha512");
    put_ssh_string(&mut binary, &signature_blob);
    format!(
        "-----BEGIN SSH SIGNATURE-----\n{}\n-----END SSH SIGNATURE-----\n",
        Base64::encode_string(&binary)
    )
}

fn put_ssh_string(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(
        &u32::try_from(value.len())
            .expect("test SSH field length")
            .to_be_bytes(),
    );
    output.extend_from_slice(value);
}

#[test]
fn review_authorization_rejects_missing_and_mismatched_authority_metadata() {
    let root = super::temporary();
    let workspace = review_workspace(&root, "Authorization failures");
    workspace
        .add_claim(&super::claim("claim-one"))
        .expect("claim");
    let request = workspace
        .prepare_review_request(
            "review-one".to_owned(),
            "claim-one".to_owned(),
            crate::domain::Assessment::Limited,
            "Rationale".to_owned(),
            "Reviewer".to_owned(),
        )
        .expect("request");
    let mut wrong_project = request.clone();
    wrong_project.project_id = "other-project".to_owned();
    assert!(
        workspace
            .authorize_review_request(wrong_project, "bad".to_owned())
            .is_err()
    );
    let mut review = request.clone().into_review(ReviewAuthorization {
        authority_id: "test-human".to_owned(),
        signer_fingerprint: "wrong".to_owned(),
        signature: "bad".to_owned(),
    });
    let snapshot = workspace.load_snapshot().expect("snapshot");
    assert!(
        workspace
            .verify_review_authorization(&snapshot, &review)
            .is_err()
    );
    review
        .authorization
        .as_mut()
        .expect("authorization")
        .authority_id = "missing".to_owned();
    assert!(
        workspace
            .verify_review_authorization(&snapshot, &review)
            .is_err()
    );
    review.authorization = None;
    assert!(
        workspace
            .verify_review_authorization(&snapshot, &review)
            .is_err()
    );
    authorize_review(&workspace, &mut review);
    review.subject_sha256 = None;
    assert!(
        workspace
            .verify_review_authorization(&snapshot, &review)
            .is_err()
    );
    review.subject_sha256 = Some(request.subject_sha256.clone());
    let mut mismatched = workspace.load_snapshot().expect("snapshot");
    mismatched.manifest.review_authority_fingerprint = Some("SHA256:wrong".to_owned());
    assert!(
        workspace
            .verify_review_authorization(&mismatched, &review)
            .is_err()
    );
    let mut mismatched = workspace.load_snapshot().expect("snapshot");
    mismatched.manifest.review_authority_id = Some("wrong-authority".to_owned());
    assert!(
        workspace
            .verify_review_authorization(&mismatched, &review)
            .is_err()
    );
    assert_unanchored_review_rejection(request);
    std::fs::remove_dir_all(root).expect("remove fixture");
}

fn assert_unanchored_review_rejection(mut request: ReviewRequest) {
    let unanchored_root = super::temporary();
    let unanchored = Workspace::initialize(&unanchored_root, "Unanchored").expect("initialize");
    assert!(
        unanchored
            .prepare_review_request(
                "review".to_owned(),
                "claim".to_owned(),
                crate::domain::Assessment::Limited,
                "Rationale".to_owned(),
                "Reviewer".to_owned(),
            )
            .is_err()
    );
    request.project_id = unanchored.read_manifest().expect("manifest").project_id;
    request.workspace_id = unanchored.read_manifest().expect("manifest").workspace_id;
    assert!(
        unanchored
            .authorize_review_request(request, "bad".to_owned())
            .is_err()
    );
    std::fs::remove_dir_all(unanchored_root).expect("remove fixture");
}
