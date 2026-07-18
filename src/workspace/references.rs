use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::Result;

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
        let mut reviewed_claims = BTreeSet::new();
        for review in &snapshot.reviews {
            if !claims.contains(review.claim_id.as_str()) {
                errors.push(format!("reviews/{}: unknown claim reference", review.id));
            }
            if !reviewed_claims.insert(review.claim_id.as_str()) {
                errors.push(format!(
                    "reviews/{}: multiple v0.1 reviews for one claim are ambiguous",
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
