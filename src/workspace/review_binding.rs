use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::domain::{ArtifactLocatorType, EvidenceLink, InventoryLimits};
use crate::{Error, Result};

use super::inventory_authority::latest_inventory_from_snapshot;
use super::inventory_scan::hash_file;
use super::publication::canonical_json_bytes;
use super::references::claim_evidence_ids;
use super::{Snapshot, Workspace};

impl Workspace {
    pub fn review_subject_binding(&self, claim_id: &str) -> Result<(Vec<String>, String)> {
        let snapshot = self.load_snapshot()?;
        let ids = claim_evidence_ids(&snapshot, claim_id);
        let digest = self.review_subject_sha256(&snapshot, claim_id)?;
        Ok((ids, digest))
    }

    pub(super) fn review_binding_errors(&self, snapshot: &Snapshot) -> Vec<String> {
        let mut errors = Vec::new();
        for review in &snapshot.reviews {
            // Unsigned reviews cannot be canonical bindings, so staleness does not apply to them.
            if review.authorization.is_none() {
                continue;
            }
            if review.evidence_ids != claim_evidence_ids(snapshot, &review.claim_id) {
                continue;
            }
            match &review.subject_sha256 {
                None => errors.push(format!(
                    "reviews/{}: legacy review lacks a subject content binding",
                    review.id
                )),
                Some(expected) => match self.review_subject_sha256(snapshot, &review.claim_id) {
                    Ok(actual) if &actual == expected => {}
                    Ok(_) => errors.push(format!(
                        "reviews/{}: reviewed subject authority changed in place",
                        review.id
                    )),
                    Err(error) => errors.push(format!("reviews/{}: {error}", review.id)),
                },
            }
        }
        errors
    }

    pub(super) fn review_subject_sha256(
        &self,
        snapshot: &Snapshot,
        claim_id: &str,
    ) -> Result<String> {
        let claim = snapshot
            .claims
            .iter()
            .find(|claim| claim.id == claim_id)
            .ok_or_else(|| Error::invalid("claim reference", "claim does not exist"))?;
        let mut hasher = Sha256::new();
        let file_limit = active_inventory_file_limit(snapshot);
        hash_serialized(&mut hasher, "claim", claim_id, claim);
        let mut evidence = snapshot
            .evidence
            .iter()
            .filter(|record| record.claim_id == claim_id)
            .collect::<Vec<_>>();
        evidence.sort_by(|left, right| left.id.cmp(&right.id));
        for record in evidence {
            hash_serialized(&mut hasher, "evidence", &record.id, record);
            self.hash_evidence_authority(&mut hasher, snapshot, record, file_limit)?;
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn hash_evidence_authority(
        &self,
        hasher: &mut Sha256,
        snapshot: &Snapshot,
        evidence: &EvidenceLink,
        file_limit: u64,
    ) -> Result<()> {
        if let Some(source_id) = &evidence.source_id {
            let source = snapshot
                .sources
                .iter()
                .find(|record| record.id == *source_id)
                .ok_or_else(|| Error::invalid("source reference", "source does not exist"))?;
            hash_serialized(hasher, "source", source_id, source);
        }
        if let Some(experiment_id) = &evidence.experiment_id {
            let experiment = snapshot
                .experiments
                .iter()
                .find(|record| record.id == *experiment_id)
                .ok_or_else(|| {
                    Error::invalid("experiment reference", "experiment does not exist")
                })?;
            hash_serialized(hasher, "experiment", experiment_id, experiment);
            for artifact in experiment
                .artifacts
                .iter()
                .filter(|artifact| artifact.locator_type == ArtifactLocatorType::Workspace)
            {
                self.hash_workspace_artifact(
                    hasher,
                    "experiment-artifact",
                    &artifact.locator,
                    file_limit,
                )?;
            }
        }
        if let Some(locator) = &evidence.artifact {
            self.hash_workspace_artifact(hasher, "artifact", locator, file_limit)?;
        }
        Ok(())
    }

    fn hash_workspace_artifact(
        &self,
        hasher: &mut Sha256,
        kind: &str,
        locator: &str,
        file_limit: u64,
    ) -> Result<()> {
        let (bytes, digest) = hash_file(&self.validate_workspace_path(locator)?, file_limit)?;
        hash_frame(
            hasher,
            kind,
            Path::new(locator).as_os_str().as_encoded_bytes(),
        );
        hasher.update(bytes.to_be_bytes());
        hasher.update(digest.as_bytes());
        Ok(())
    }
}

fn active_inventory_file_limit(snapshot: &Snapshot) -> u64 {
    latest_inventory_from_snapshot(snapshot)
        .and_then(|inventory| inventory.policy)
        .map_or(InventoryLimits::legacy().max_file_bytes, |policy| {
            policy.limits.max_file_bytes
        })
}

fn hash_serialized(hasher: &mut Sha256, kind: &str, id: &str, value: &impl Serialize) {
    hash_frame(hasher, kind, id.as_bytes());
    let bytes = canonical_json_bytes(value);
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn hash_frame(hasher: &mut Sha256, kind: &str, value: &[u8]) {
    hasher.update((kind.len() as u64).to_be_bytes());
    hasher.update(kind.as_bytes());
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}
