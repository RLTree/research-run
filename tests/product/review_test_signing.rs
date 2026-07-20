use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use base64ct::{Base64, Encoding};
use ed25519_dalek::{Signer, SigningKey};
use research_run::domain::REVIEW_SIGNATURE_NAMESPACE;
use research_run::domain::ReviewRequest;
use research_run::workspace::Workspace;
use sha2::{Digest, Sha256, Sha512};

use super::researcher_journey::{TempDir, succeeds};

static REVIEW_KEY: OnceLock<SigningKey> = OnceLock::new();

#[cfg(coverage)]
#[path = "review_test_signing/coverage_records.rs"]
mod coverage_records;
#[cfg(coverage)]
pub(super) use coverage_records::ReviewSigner;

pub(super) struct SignedReview {
    pub(super) _files: TempDir,
    pub(super) request: std::path::PathBuf,
    pub(super) signature: std::path::PathBuf,
}

pub(super) fn add_signed_review(
    project: &Path,
    id: &str,
    claim: &str,
    decision: &str,
    rationale: &str,
    reviewer: &str,
) {
    let signed = prepare_signed_review(project, id, claim, decision, rationale, reviewer);
    succeeds(
        project,
        &[
            "review",
            "add",
            "--request",
            &signed.request.to_string_lossy(),
            "--signature",
            &signed.signature.to_string_lossy(),
        ],
    );
}

pub(super) fn prepare_signed_review(
    project: &Path,
    id: &str,
    claim: &str,
    decision: &str,
    rationale: &str,
    reviewer: &str,
) -> SignedReview {
    enroll_test_authority(project);
    let files = TempDir::new("signed-review");
    let request = files.0.join("request.json");
    let signature = files.0.join("request.json.sig");
    let prepared = succeeds(
        project,
        &[
            "review",
            "prepare",
            "--id",
            id,
            "--claim",
            claim,
            "--decision",
            decision,
            "--rationale",
            rationale,
            "--reviewer",
            reviewer,
        ],
    );
    fs::write(&request, &prepared.stdout).expect("write review request");
    let signed = sign_sshsig(&prepared.stdout);
    fs::write(&signature, signed).expect("write review signature");
    SignedReview {
        _files: files,
        request,
        signature,
    }
}

pub(super) fn signed_review_record(
    project: &Path,
    id: &str,
    claim: &str,
    decision: &str,
    rationale: &str,
    reviewer: &str,
) -> serde_json::Value {
    let signed = prepare_signed_review(project, id, claim, decision, rationale, reviewer);
    let request: ReviewRequest =
        serde_json::from_slice(&fs::read(&signed.request).expect("request")).expect("request JSON");
    let signature = fs::read_to_string(&signed.signature).expect("signature");
    let review = Workspace::discover(project)
        .expect("workspace")
        .authorize_review_request(request, signature)
        .expect("authorized review");
    serde_json::to_value(review).expect("review JSON")
}

pub(super) fn enroll_test_authority(project: &Path) {
    assert!(
        project
            .join(".research-run/review-authorities/test-human.json")
            .is_file(),
        "test review authority must be anchored during workspace initialization"
    );
}

pub(super) fn initialize_with_test_authority(cwd: &Path, project: &Path, name: &str) {
    let files = TempDir::new("review-authority");
    let public_key = write_test_public_key(&files.0);
    succeeds(
        cwd,
        &[
            "init",
            &project.to_string_lossy(),
            "--name",
            name,
            "--review-authority-id",
            "test-human",
            "--review-authority-public-key",
            &public_key.to_string_lossy(),
        ],
    );
}

pub(super) fn write_test_public_key(directory: &Path) -> std::path::PathBuf {
    let public_key = directory.join("research-run-test-authority.pub");
    fs::write(&public_key, test_public_key()).expect("write test public key");
    public_key
}

fn test_key() -> &'static SigningKey {
    REVIEW_KEY
        .get_or_init(|| SigningKey::from_bytes(&Sha256::digest(b"research-run-test-key").into()))
}

fn test_public_key_blob() -> Vec<u8> {
    let mut blob = Vec::new();
    put_ssh_string(&mut blob, b"ssh-ed25519");
    put_ssh_string(&mut blob, test_key().verifying_key().as_bytes());
    blob
}

fn test_public_key() -> String {
    format!(
        "ssh-ed25519 {}",
        Base64::encode_string(&test_public_key_blob())
    )
}

fn sign_sshsig(message: &[u8]) -> String {
    let hash_algorithm = b"sha512";
    let digest = Sha512::digest(message);
    let mut signed = b"SSHSIG".to_vec();
    put_ssh_string(&mut signed, REVIEW_SIGNATURE_NAMESPACE.as_bytes());
    put_ssh_string(&mut signed, b"");
    put_ssh_string(&mut signed, hash_algorithm);
    put_ssh_string(&mut signed, &digest);
    let signature = test_key().sign(&signed);

    let mut signature_blob = Vec::new();
    put_ssh_string(&mut signature_blob, b"ssh-ed25519");
    put_ssh_string(&mut signature_blob, &signature.to_bytes());
    let mut binary = b"SSHSIG".to_vec();
    binary.extend_from_slice(&1_u32.to_be_bytes());
    put_ssh_string(&mut binary, &test_public_key_blob());
    put_ssh_string(&mut binary, REVIEW_SIGNATURE_NAMESPACE.as_bytes());
    put_ssh_string(&mut binary, b"");
    put_ssh_string(&mut binary, hash_algorithm);
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
