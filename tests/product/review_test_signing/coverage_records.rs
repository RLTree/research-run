use std::fs;
use std::path::Path;

use research_run::domain::{
    Assessment, Authorship, ClaimRecord, ReviewAuthorization, ReviewRequest,
};
use sha2::{Digest, Sha256};

use super::sign_sshsig;

pub(crate) struct ReviewSigner {
    project_id: String,
    workspace_id: String,
    authority_id: String,
    authority_fingerprint: String,
}

impl ReviewSigner {
    pub(crate) fn for_project(project: &Path) -> Self {
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(project.join(".research-run/manifest.json")).expect("manifest"),
        )
        .expect("manifest JSON");
        let authority: serde_json::Value = serde_json::from_slice(
            &fs::read(project.join(".research-run/review-authorities/test-human.json"))
                .expect("authority"),
        )
        .expect("authority JSON");
        Self {
            project_id: string_field(&manifest, "project_id"),
            workspace_id: string_field(&manifest, "workspace_id"),
            authority_id: string_field(&authority, "id"),
            authority_fingerprint: string_field(&authority, "fingerprint"),
        }
    }

    pub(crate) fn record(&self, id: &str, claim_id: &str, decision: &str) -> serde_json::Value {
        let request = ReviewRequest {
            schema_version: 1,
            kind: "review-request".to_owned(),
            project_id: self.project_id.clone(),
            workspace_id: self.workspace_id.clone(),
            id: id.to_owned(),
            claim_id: claim_id.to_owned(),
            evidence_ids: Vec::new(),
            subject_sha256: claim_subject_digest(claim_id),
            decision: parse_decision(decision),
            rationale: "Rationale".to_owned(),
            reviewer: "Reviewer".to_owned(),
        };
        let signature = sign_sshsig(&canonical_json(&request));
        serde_json::to_value(request.into_review(ReviewAuthorization {
            authority_id: self.authority_id.clone(),
            signer_fingerprint: self.authority_fingerprint.clone(),
            signature,
        }))
        .expect("review JSON")
    }
}

fn claim_subject_digest(claim_id: &str) -> String {
    let claim = ClaimRecord {
        schema_version: 1,
        kind: "claim".to_owned(),
        id: claim_id.to_owned(),
        text: "Claim".to_owned(),
        scope: "Scope".to_owned(),
        owner: "Owner".to_owned(),
        authorship: Authorship::Human,
    };
    let mut hasher = Sha256::new();
    hasher.update(5_u64.to_be_bytes());
    hasher.update(b"claim");
    hasher.update((claim_id.len() as u64).to_be_bytes());
    hasher.update(claim_id.as_bytes());
    let bytes = canonical_json(&claim);
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn canonical_json(value: &impl serde::Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).expect("canonical JSON");
    bytes.push(b'\n');
    bytes
}

fn parse_decision(value: &str) -> Assessment {
    match value {
        "unsupported" => Assessment::Unsupported,
        "limited" => Assessment::Limited,
        "supported" => Assessment::Supported,
        "contradicted" => Assessment::Contradicted,
        _ => panic!("invalid test review decision"),
    }
}

fn string_field(value: &serde_json::Value, field: &str) -> String {
    value[field].as_str().expect("string field").to_owned()
}
