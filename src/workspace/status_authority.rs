use crate::domain::ProjectManifest;

use super::{ClaimStatus, ReviewAuthorityStatus, Snapshot};

pub(super) fn review_authority_status(snapshot: &Snapshot) -> ReviewAuthorityStatus {
    review_authority_status_from_manifest(&snapshot.manifest)
}

pub(super) fn review_authority_status_from_manifest(
    manifest: &ProjectManifest,
) -> ReviewAuthorityStatus {
    match (
        &manifest.review_authority_id,
        &manifest.review_authority_fingerprint,
    ) {
        (Some(id), Some(fingerprint)) => ReviewAuthorityStatus {
            mode: "anchored",
            id: Some(id.clone()),
            fingerprint: Some(fingerprint.clone()),
            promotion_capable: true,
            repairable: false,
            blocker: None,
            next_action: "Prepare a review request and obtain the configured key's detached signature.",
        },
        _ => ReviewAuthorityStatus {
            mode: "unanchored",
            id: None,
            fingerprint: None,
            promotion_capable: false,
            repairable: false,
            blocker: Some(
                "Workspace has no review authority; claims can never be promoted in this workspace.",
            ),
            next_action: "Create a new workspace with --review-authority-id and --review-authority-public-key.",
        },
    }
}

pub(super) fn aggregate_actions(
    claims: &[ClaimStatus],
    review_authority: &ReviewAuthorityStatus,
) -> (Vec<String>, Vec<String>) {
    let mut blockers = claims
        .iter()
        .flat_map(|claim| claim.blockers.iter().cloned())
        .collect::<Vec<_>>();
    let mut next_actions = claims
        .iter()
        .map(|claim| claim.next_action.clone())
        .collect::<Vec<_>>();
    if let Some(blocker) = review_authority.blocker {
        blockers.insert(0, blocker.to_owned());
        next_actions.insert(0, review_authority.next_action.to_owned());
    }
    (blockers, next_actions)
}
