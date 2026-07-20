use base64ct::{Base64, Encoding};

use crate::domain::{
    Assessment, CanonicalRecord, FORMAT_VERSION, REVIEW_SIGNATURE_NAMESPACE, ReviewAuthority,
    ReviewAuthorization, ReviewDecision, ReviewRequest,
};
use crate::{Error, Result};

use super::publication::canonical_json_bytes;
use super::sshsig::{fingerprint, parse_authority_key, parse_openssh_public_key, verify};
use super::{Snapshot, Workspace};

impl ReviewAuthority {
    pub fn from_openssh(id: String, encoded: &str) -> Result<Self> {
        let parsed = parse_openssh_public_key(encoded)?;
        let authority = Self {
            schema_version: FORMAT_VERSION,
            kind: Self::KIND.to_owned(),
            id,
            public_key: format!("ssh-ed25519 {}", Base64::encode_string(&parsed.blob)),
            fingerprint: fingerprint(&parsed),
        };
        authority.validate()?;
        Ok(authority)
    }
}

impl Workspace {
    pub fn prepare_review_request(
        &self,
        id: String,
        claim_id: String,
        decision: Assessment,
        rationale: String,
        reviewer: String,
    ) -> Result<ReviewRequest> {
        let snapshot = self.load_snapshot()?;
        let authority = sole_authority(&snapshot)?;
        parse_authority_key(authority)?;
        authority_matches_manifest(&snapshot, authority)?;
        let (evidence_ids, subject_sha256) = self.review_subject_binding(&claim_id)?;
        let request = ReviewRequest {
            schema_version: FORMAT_VERSION,
            kind: "review-request".to_owned(),
            project_id: snapshot.manifest.project_id,
            workspace_id: snapshot.manifest.workspace_id,
            id,
            claim_id,
            evidence_ids,
            subject_sha256,
            decision,
            rationale,
            reviewer,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn authorize_review_request(
        &self,
        request: ReviewRequest,
        signature: String,
    ) -> Result<ReviewDecision> {
        request.validate()?;
        let snapshot = self.load_snapshot()?;
        if request.project_id != snapshot.manifest.project_id
            || request.workspace_id != snapshot.manifest.workspace_id
        {
            return Err(Error::Conflict(
                "review request project or workspace identity does not match target workspace"
                    .to_owned(),
            ));
        }
        let authority = sole_authority(&snapshot)?;
        let authorization = ReviewAuthorization {
            authority_id: authority.id.clone(),
            signer_fingerprint: authority.fingerprint.clone(),
            signature,
        };
        let review = request.into_review(authorization);
        self.verify_review_authorization(&snapshot, &review)?;
        Ok(review)
    }

    pub(super) fn review_authorization_errors(&self, snapshot: &Snapshot) -> Vec<String> {
        let mut errors = Vec::new();
        let manifest_is_anchored = snapshot.manifest.review_authority_id.is_some();
        if manifest_is_anchored && snapshot.review_authorities.len() != 1 {
            errors.push(
                "review-authorities: an anchored manifest requires exactly one enrolled authority"
                    .to_owned(),
            );
        } else if !manifest_is_anchored && !snapshot.review_authorities.is_empty() {
            errors.push(
                "review-authorities: an unanchored manifest cannot enroll an authority".to_owned(),
            );
        }
        for authority in &snapshot.review_authorities {
            if let Err(error) = parse_authority_key(authority) {
                errors.push(format!("review-authorities/{}: {error}", authority.id));
            } else if let Err(error) = authority_matches_manifest(snapshot, authority) {
                errors.push(format!("review-authorities/{}: {error}", authority.id));
            }
        }
        for review in &snapshot.reviews {
            if review.authorization.is_none() && !manifest_is_anchored {
                continue;
            }
            if let Err(error) = self.verify_review_authorization(snapshot, review) {
                errors.push(format!("reviews/{}: {error}", review.id));
            }
        }
        errors
    }

    pub(super) fn verify_review_authorization(
        &self,
        snapshot: &Snapshot,
        review: &ReviewDecision,
    ) -> Result<()> {
        let authorization = review.authorization.as_ref().ok_or_else(|| {
            Error::invalid(
                "review authorization",
                "a detached human-authority signature is required",
            )
        })?;
        let authority = snapshot
            .review_authorities
            .iter()
            .find(|candidate| candidate.id == authorization.authority_id)
            .ok_or_else(|| {
                Error::invalid(
                    "review authorization",
                    "the referenced review authority is not enrolled",
                )
            })?;
        if authorization.signer_fingerprint != authority.fingerprint {
            return Err(Error::invalid(
                "review authorization",
                "signer fingerprint does not match the enrolled authority",
            ));
        }
        authority_matches_manifest(snapshot, authority)?;
        let public_key = parse_authority_key(authority)?;
        let request = request_from_review(
            &snapshot.manifest.project_id,
            &snapshot.manifest.workspace_id,
            review,
        )?;
        verify(
            &public_key,
            REVIEW_SIGNATURE_NAMESPACE,
            &canonical_json_bytes(&request),
            &authorization.signature,
        )
    }
}

fn sole_authority(snapshot: &Snapshot) -> Result<&ReviewAuthority> {
    if snapshot.review_authorities.len() == 1 {
        Ok(&snapshot.review_authorities[0])
    } else {
        Err(Error::invalid(
            "review authority",
            "exactly one review authority must be enrolled",
        ))
    }
}

fn authority_matches_manifest(snapshot: &Snapshot, authority: &ReviewAuthority) -> Result<()> {
    if snapshot.manifest.review_authority_id.as_deref() != Some(&authority.id)
        || snapshot.manifest.review_authority_fingerprint.as_deref() != Some(&authority.fingerprint)
    {
        return Err(Error::invalid(
            "review authority",
            "authority does not match the immutable manifest trust anchor",
        ));
    }
    Ok(())
}

fn request_from_review(
    project_id: &str,
    workspace_id: &str,
    review: &ReviewDecision,
) -> Result<ReviewRequest> {
    let request = ReviewRequest {
        schema_version: review.schema_version,
        kind: "review-request".to_owned(),
        project_id: project_id.to_owned(),
        workspace_id: workspace_id.to_owned(),
        id: review.id.clone(),
        claim_id: review.claim_id.clone(),
        evidence_ids: review.evidence_ids.clone(),
        subject_sha256: review.subject_sha256.clone().ok_or_else(|| {
            Error::invalid(
                "review authorization",
                "subject_sha256 is required for signed reviews",
            )
        })?,
        decision: review.decision,
        rationale: review.rationale.clone(),
        reviewer: review.reviewer.clone(),
    };
    request.validate()?;
    Ok(request)
}
