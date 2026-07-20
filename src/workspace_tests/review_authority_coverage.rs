use super::*;
use crate::domain::ReviewAuthorization;

#[test]
fn review_authority_construction_and_preparation_propagate_owned_validation() {
    assert!(
        ReviewAuthority::from_openssh("Invalid".to_owned(), &test_review_public_key()).is_err()
    );

    let invalid_root = temporary();
    let invalid = review_workspace(&invalid_root, "Invalid preparation");
    fs::write(invalid.state.join("manifest.json"), b"{").expect("corrupt manifest");
    assert!(
        invalid
            .prepare_review_request(
                "review-one".to_owned(),
                "claim-one".to_owned(),
                Assessment::Limited,
                "Rationale".to_owned(),
                "Reviewer".to_owned(),
            )
            .is_err()
    );
    fs::remove_dir_all(invalid_root).expect("remove fixture");

    let root = temporary();
    let workspace = review_workspace(&root, "Request validation");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    assert!(
        workspace
            .prepare_review_request(
                "Invalid".to_owned(),
                "claim-one".to_owned(),
                Assessment::Limited,
                "Rationale".to_owned(),
                "Reviewer".to_owned(),
            )
            .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn review_preparation_rejects_tampered_key_and_manifest_anchor() {
    for tamper_key in [true, false] {
        let root = temporary();
        let workspace = review_workspace(&root, "Tampered preparation");
        workspace.add_claim(&claim("claim-one")).expect("claim");
        if tamper_key {
            let path = workspace.state.join("review-authorities/test-human.json");
            let mut authority: ReviewAuthority =
                serde_json::from_slice(&fs::read(&path).expect("authority")).expect("parse");
            authority.public_key = "ssh-ed25519 malformed".to_owned();
            fs::write(path, serde_json::to_vec(&authority).expect("serialize")).expect("tamper");
        } else {
            let path = workspace.state.join("manifest.json");
            let mut manifest: ProjectManifest =
                serde_json::from_slice(&fs::read(&path).expect("manifest")).expect("parse");
            manifest.review_authority_id = Some("other-human".to_owned());
            fs::write(path, serde_json::to_vec(&manifest).expect("serialize")).expect("tamper");
        }
        assert!(
            workspace
                .prepare_review_request(
                    "review-one".to_owned(),
                    "claim-one".to_owned(),
                    Assessment::Limited,
                    "Rationale".to_owned(),
                    "Reviewer".to_owned(),
                )
                .is_err()
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn review_authorization_propagates_request_snapshot_key_and_nested_validation() {
    let root = temporary();
    let workspace = review_workspace(&root, "Authorization propagation");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    let request = workspace
        .prepare_review_request(
            "review-one".to_owned(),
            "claim-one".to_owned(),
            Assessment::Limited,
            "Rationale".to_owned(),
            "Reviewer".to_owned(),
        )
        .expect("request");

    let mut invalid_request = request.clone();
    invalid_request.schema_version = 2;
    assert!(
        workspace
            .authorize_review_request(invalid_request, "signature".to_owned())
            .is_err()
    );

    let mut invalid_key_snapshot = workspace.load_snapshot().expect("snapshot");
    invalid_key_snapshot.review_authorities[0].public_key = "invalid".to_owned();
    let authority = &invalid_key_snapshot.review_authorities[0];
    let mut review = request.clone().into_review(ReviewAuthorization {
        authority_id: authority.id.clone(),
        signer_fingerprint: authority.fingerprint.clone(),
        signature: "signature".to_owned(),
    });
    assert!(
        workspace
            .verify_review_authorization(&invalid_key_snapshot, &review)
            .is_err()
    );

    review.rationale.clear();
    assert!(
        workspace
            .verify_review_authorization(&workspace.load_snapshot().expect("snapshot"), &review,)
            .is_err()
    );

    fs::write(workspace.state.join("manifest.json"), b"{").expect("corrupt manifest");
    assert!(
        workspace
            .authorize_review_request(request, "signature".to_owned())
            .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn review_publication_propagates_record_inspection_and_signature_failures() {
    let root = temporary();
    let workspace = review_workspace(&root, "Review publication propagation");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    let mut review = review("review-one", "claim-one");
    review.subject_sha256 = Some(workspace.review_subject_binding("claim-one").unwrap().1);
    authorize_review(&workspace, &mut review);

    assert!(workspace.add_review(&review).expect("initial review"));
    inject_storage_failure("inspect record#5");
    assert!(workspace.add_review(&review).is_err());

    review
        .authorization
        .as_mut()
        .expect("authorization")
        .signature = "invalid".to_owned();
    assert!(workspace.add_review(&review).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}
