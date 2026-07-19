use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::Result;
use crate::domain::ReviewDecision;

use super::path_safety::reject_symlink_chain;
use super::{Snapshot, Workspace};

impl Workspace {
    pub(super) fn reference_errors(&self, snapshot: &Snapshot) -> Vec<String> {
        let sources: BTreeSet<_> = snapshot
            .sources
            .iter()
            .map(|record| record.id.as_str())
            .collect();
        let claims: BTreeSet<_> = snapshot
            .claims
            .iter()
            .map(|record| record.id.as_str())
            .collect();
        let experiments: BTreeSet<_> = snapshot
            .experiments
            .iter()
            .map(|record| record.id.as_str())
            .collect();
        let evidence_claims = snapshot
            .evidence
            .iter()
            .map(|record| (record.id.as_str(), record.claim_id.as_str()))
            .collect::<BTreeMap<_, _>>();
        let mut errors = Vec::new();
        for link in &snapshot.evidence {
            if !claims.contains(link.claim_id.as_str()) {
                errors.push(format!("evidence/{}: unknown claim reference", link.id));
            }
            if let Some(source_id) = &link.source_id
                && !sources.contains(source_id.as_str())
            {
                errors.push(format!("evidence/{}: unknown source reference", link.id));
            }
            if let Some(experiment_id) = &link.experiment_id
                && !experiments.contains(experiment_id.as_str())
            {
                errors.push(format!(
                    "evidence/{}: unknown experiment reference",
                    link.id
                ));
            }
            if let Some(locator) = &link.artifact
                && let Err(error) = self.validate_workspace_path(locator)
            {
                errors.push(format!("evidence/{}: {error}", link.id));
            }
        }
        let mut reviewed_graphs = BTreeSet::new();
        for review in &snapshot.reviews {
            if !claims.contains(review.claim_id.as_str()) {
                errors.push(format!("reviews/{}: unknown claim reference", review.id));
            }
            for evidence_id in &review.evidence_ids {
                match evidence_claims.get(evidence_id.as_str()) {
                    None => errors.push(format!(
                        "reviews/{}: unknown evidence reference {evidence_id}",
                        review.id
                    )),
                    Some(claim_id) if *claim_id != review.claim_id.as_str() => {
                        errors.push(format!(
                            "reviews/{}: evidence {evidence_id} belongs to claim {claim_id}",
                            review.id
                        ))
                    }
                    Some(_) => {}
                }
            }
            if !reviewed_graphs.insert((review.claim_id.as_str(), review.evidence_ids.as_slice())) {
                errors.push(format!(
                    "reviews/{}: multiple v0.1 reviews for one claim evidence graph are ambiguous",
                    review.id
                ));
            }
        }
        for experiment in &snapshot.experiments {
            if let Err(error) = self.validate_artifact_paths(&experiment.artifacts) {
                errors.push(format!("experiments/{}: {error}", experiment.id));
            }
        }
        errors
    }

    pub(super) fn validate_artifact_paths(
        &self,
        artifacts: &[crate::domain::ArtifactPointer],
    ) -> Result<()> {
        for artifact in artifacts {
            if artifact.locator_type == crate::domain::ArtifactLocatorType::Workspace {
                self.validate_workspace_path(&artifact.locator)?;
            }
        }
        Ok(())
    }

    pub(super) fn validate_workspace_path(&self, locator: &str) -> Result<PathBuf> {
        // Every caller holds a record that has already passed its typed
        // workspace-locator validation; this private helper enforces the
        // filesystem half of that boundary.
        let candidate = self.root.join(locator);
        reject_symlink_chain(&candidate)?;
        Ok(candidate)
    }
}

pub(super) fn current_review_bindings(snapshot: &Snapshot) -> BTreeMap<&str, &ReviewDecision> {
    let mut evidence_by_claim = BTreeMap::<&str, Vec<&str>>::new();
    for evidence in &snapshot.evidence {
        evidence_by_claim
            .entry(evidence.claim_id.as_str())
            .or_default()
            .push(evidence.id.as_str());
    }
    for ids in evidence_by_claim.values_mut() {
        ids.sort_unstable();
    }
    snapshot
        .reviews
        .iter()
        .filter(|review| {
            let current = evidence_by_claim
                .get(review.claim_id.as_str())
                .map(Vec::as_slice)
                .unwrap_or_default();
            review
                .evidence_ids
                .iter()
                .map(String::as_str)
                .eq(current.iter().copied())
        })
        .map(|review| (review.claim_id.as_str(), review))
        .collect()
}
