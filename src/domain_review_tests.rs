use super::{
    Assessment, CanonicalRecord, ProjectManifest, ReviewAuthority, ReviewAuthorization,
    ReviewRequest,
};

fn authority() -> ReviewAuthority {
    ReviewAuthority {
        schema_version: 1,
        kind: "review-authority".to_owned(),
        id: "authority".to_owned(),
        public_key: "ssh-ed25519 encoded".to_owned(),
        fingerprint: "SHA256:fingerprint".to_owned(),
    }
}

fn authorization() -> ReviewAuthorization {
    ReviewAuthorization {
        authority_id: "authority".to_owned(),
        signer_fingerprint: "SHA256:fingerprint".to_owned(),
        signature: "signature".to_owned(),
    }
}

fn request() -> ReviewRequest {
    let manifest = ProjectManifest::new("Review request").expect("manifest");
    ReviewRequest {
        schema_version: 1,
        kind: "review-request".to_owned(),
        project_id: manifest.project_id,
        workspace_id: manifest.workspace_id,
        id: "review-one".to_owned(),
        claim_id: "claim-one".to_owned(),
        evidence_ids: Vec::new(),
        subject_sha256: "a".repeat(64),
        decision: Assessment::Limited,
        rationale: "Rationale".to_owned(),
        reviewer: "Reviewer".to_owned(),
    }
}

#[test]
fn review_authority_and_authorization_validate_each_owned_field() {
    let valid = authority();
    assert!(valid.validate().is_ok());
    for invalid in [
        ReviewAuthority {
            schema_version: 2,
            ..valid.clone()
        },
        ReviewAuthority {
            public_key: String::new(),
            ..valid.clone()
        },
        ReviewAuthority {
            fingerprint: String::new(),
            ..valid.clone()
        },
    ] {
        assert!(invalid.validate().is_err());
    }

    let valid = authorization();
    assert!(valid.validate().is_ok());
    for invalid in [
        ReviewAuthorization {
            authority_id: "Invalid".to_owned(),
            ..valid.clone()
        },
        ReviewAuthorization {
            signer_fingerprint: String::new(),
            ..valid.clone()
        },
        ReviewAuthorization {
            signature: String::new(),
            ..valid.clone()
        },
    ] {
        assert!(invalid.validate().is_err());
    }
}

#[test]
fn review_request_and_nested_authorization_fail_at_each_boundary() {
    let valid = request();
    assert!(valid.validate().is_ok());
    for invalid in [
        ReviewRequest {
            schema_version: 2,
            ..valid.clone()
        },
        ReviewRequest {
            project_id: "Invalid".to_owned(),
            ..valid.clone()
        },
        ReviewRequest {
            workspace_id: "not-a-digest".to_owned(),
            ..valid.clone()
        },
    ] {
        assert!(invalid.validate().is_err());
    }
    let mut review = valid.into_review(ReviewAuthorization {
        signature: String::new(),
        ..authorization()
    });
    assert!(review.validate().is_err());
    review.authorization = None;
    assert!(review.validate().is_ok());
}
