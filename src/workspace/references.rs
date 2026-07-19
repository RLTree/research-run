use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::Result;
use crate::domain::ReviewDecision;

use super::knowledge::relationship_reference_errors;
use super::path_safety::reject_symlink_chain;
use super::{Snapshot, Workspace};

impl Workspace {
    pub(super) fn reference_errors(&self, snapshot: &Snapshot) -> Vec<String> {
        let sources = ids(&snapshot.sources, |record| record.id.as_str());
        let claims = ids(&snapshot.claims, |record| record.id.as_str());
        let experiments = ids(&snapshot.experiments, |record| record.id.as_str());
        let evidence_claims = snapshot
            .evidence
            .iter()
            .map(|record| (record.id.as_str(), record.claim_id.as_str()))
            .collect::<BTreeMap<_, _>>();
        let mut errors = evidence_reference_errors(self, snapshot, &sources, &claims, &experiments);
        errors.extend(review_reference_errors(snapshot, &claims, &evidence_claims));
        for experiment in &snapshot.experiments {
            if let Err(error) = self.validate_artifact_paths(&experiment.artifacts) {
                errors.push(format!("experiments/{}: {error}", experiment.id));
            }
        }
        errors.extend(relationship_reference_errors(snapshot));
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
        let candidate = self.root.join(locator);
        reject_symlink_chain(&candidate)?;
        Ok(candidate)
    }
}

fn ids<'a, T>(records: &'a [T], id: impl Fn(&'a T) -> &'a str) -> BTreeSet<&'a str> {
    records.iter().map(id).collect()
}

fn evidence_reference_errors(
    workspace: &Workspace,
    snapshot: &Snapshot,
    sources: &BTreeSet<&str>,
    claims: &BTreeSet<&str>,
    experiments: &BTreeSet<&str>,
) -> Vec<String> {
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
            && let Err(error) = workspace.validate_workspace_path(locator)
        {
            errors.push(format!("evidence/{}: {error}", link.id));
        }
    }
    errors
}

fn review_reference_errors(
    snapshot: &Snapshot,
    claims: &BTreeSet<&str>,
    evidence_claims: &BTreeMap<&str, &str>,
) -> Vec<String> {
    let mut errors = Vec::new();
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
                Some(claim_id) if *claim_id != review.claim_id.as_str() => errors.push(format!(
                    "reviews/{}: evidence {evidence_id} belongs to claim {claim_id}",
                    review.id
                )),
                Some(_) => {}
            }
        }
        if !reviewed_graphs.insert((
            review.claim_id.as_str(),
            review.evidence_ids.as_slice(),
            review.subject_sha256.as_deref(),
        )) {
            errors.push(format!(
                "reviews/{}: duplicate review authority binding is ambiguous",
                review.id
            ));
        }
    }
    errors
}

pub(super) fn current_review_bindings<'a>(
    workspace: &Workspace,
    snapshot: &'a Snapshot,
) -> Result<BTreeMap<&'a str, &'a ReviewDecision>> {
    let mut bindings = BTreeMap::new();
    for review in &snapshot.reviews {
        if review.evidence_ids == claim_evidence_ids(snapshot, &review.claim_id)
            && let Some(expected) = &review.subject_sha256
            && workspace.review_subject_sha256(snapshot, &review.claim_id)? == *expected
        {
            bindings.insert(review.claim_id.as_str(), review);
        }
    }
    Ok(bindings)
}

pub(super) fn claim_evidence_ids(snapshot: &Snapshot, claim_id: &str) -> Vec<String> {
    let mut ids = snapshot
        .evidence
        .iter()
        .filter(|evidence| evidence.claim_id == claim_id)
        .map(|evidence| evidence.id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids
}
