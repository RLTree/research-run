use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{
    Assessment, Authorship, ClaimRecord, EvidenceLink, ExperimentReceipt, ReviewDecision,
    SourceProvenance, SourceRecord,
};
use crate::{Error, Result};

use super::references::current_review_bindings;
use super::{
    AiDraftStatus, ClaimStatus, EvidenceStatus, ExperimentStatus, ProjectStatus, Snapshot, Status,
    Workspace,
};

const CLAIM_CEILING: &str = "Assessments describe reviewed support within this workspace; they do not establish scientific truth or real-world validity.";

struct StatusIndex<'a> {
    reviews_by_claim: BTreeMap<&'a str, &'a ReviewDecision>,
    reviewed_claims: BTreeSet<&'a str>,
    claims_by_source: BTreeMap<&'a str, Vec<&'a str>>,
    evidence_by_claim: BTreeMap<&'a str, Vec<&'a EvidenceLink>>,
    ai_evidence_claims: BTreeSet<&'a str>,
}

impl<'a> StatusIndex<'a> {
    fn build(snapshot: &'a Snapshot) -> Self {
        let mut index = Self {
            reviews_by_claim: BTreeMap::new(),
            reviewed_claims: BTreeSet::new(),
            claims_by_source: BTreeMap::new(),
            evidence_by_claim: BTreeMap::new(),
            ai_evidence_claims: BTreeSet::new(),
        };
        for link in &snapshot.evidence {
            index
                .evidence_by_claim
                .entry(link.claim_id.as_str())
                .or_default()
                .push(link);
            if let Some(source_id) = link.source_id.as_deref() {
                index
                    .claims_by_source
                    .entry(source_id)
                    .or_default()
                    .push(link.claim_id.as_str());
            }
            if link.authorship == Authorship::Ai {
                index.ai_evidence_claims.insert(link.claim_id.as_str());
            }
        }
        index.reviews_by_claim = current_review_bindings(snapshot);
        index.reviewed_claims = index.reviews_by_claim.keys().copied().collect();
        index
    }
}

impl Workspace {
    pub fn status(&self) -> Result<Status> {
        let snapshot = self.load_snapshot()?;
        if !self.reference_errors(&snapshot).is_empty() {
            return Err(Error::invalid(
                "workspace",
                "reference validation failed; run 'research-run validate'",
            ));
        }
        let counts = snapshot.counts();
        let mut index = StatusIndex::build(&snapshot);
        let unreviewed_ai_drafts = unreviewed_ai_drafts(&snapshot, &index);
        let claims = claim_statuses(&snapshot.claims, &mut index);
        let experiments = experiment_statuses(snapshot.experiments);
        let (blockers, next_actions) = aggregate_actions(&claims);
        Ok(Status {
            format_version: crate::domain::FORMAT_VERSION,
            project: ProjectStatus {
                id: snapshot.manifest.project_id,
                name: snapshot.manifest.name,
            },
            claim_ceiling: CLAIM_CEILING,
            counts,
            claims,
            experiments,
            unreviewed_ai_drafts,
            blockers,
            next_actions,
        })
    }
}

fn unreviewed_ai_drafts(snapshot: &Snapshot, index: &StatusIndex<'_>) -> Vec<AiDraftStatus> {
    let mut drafts = Vec::new();
    drafts.extend(
        snapshot
            .sources
            .iter()
            .filter(|source| source_requires_review(source, index))
            .map(|source| AiDraftStatus {
                kind: "source",
                id: source.id.clone(),
                reason: "AI-provenance source is not covered by a claim review.",
            }),
    );
    drafts.extend(
        snapshot
            .claims
            .iter()
            .filter(|claim| {
                claim.authorship == Authorship::Ai
                    && !index.reviewed_claims.contains(claim.id.as_str())
            })
            .map(|claim| AiDraftStatus {
                kind: "claim",
                id: claim.id.clone(),
                reason: "AI-authored claim lacks an explicit human review decision.",
            }),
    );
    drafts.extend(
        snapshot
            .evidence
            .iter()
            .filter(|link| {
                link.authorship == Authorship::Ai
                    && !index.reviewed_claims.contains(link.claim_id.as_str())
            })
            .map(|link| AiDraftStatus {
                kind: "evidence",
                id: link.id.clone(),
                reason: "AI-authored evidence link is not covered by a claim review.",
            }),
    );
    drafts.sort();
    drafts
}

fn source_requires_review(source: &SourceRecord, index: &StatusIndex<'_>) -> bool {
    if source.provenance != SourceProvenance::Ai {
        return false;
    }
    let Some(claims) = index.claims_by_source.get(source.id.as_str()) else {
        return true;
    };
    claims.is_empty()
        || claims
            .iter()
            .any(|claim_id| !index.reviewed_claims.contains(claim_id))
}

fn claim_statuses(claims: &[ClaimRecord], index: &mut StatusIndex<'_>) -> Vec<ClaimStatus> {
    claims
        .iter()
        .map(|claim| {
            let links = index
                .evidence_by_claim
                .remove(claim.id.as_str())
                .unwrap_or_default();
            let reviewed = index.reviews_by_claim.get(claim.id.as_str()).copied();
            let ai_draft = claim.authorship == Authorship::Ai
                || index.ai_evidence_claims.contains(claim.id.as_str());
            let (assessment, blockers, next_action) =
                claim_assessment(claim, &links, reviewed, ai_draft);
            ClaimStatus {
                id: claim.id.clone(),
                text: claim.text.clone(),
                scope: claim.scope.clone(),
                assessment,
                evidence: links
                    .into_iter()
                    .map(|link| EvidenceStatus {
                        id: link.id.clone(),
                        stance: link.stance,
                        authorship: link.authorship,
                    })
                    .collect(),
                blockers,
                next_action,
            }
        })
        .collect()
}

fn claim_assessment(
    claim: &ClaimRecord,
    links: &[&EvidenceLink],
    review: Option<&ReviewDecision>,
    ai_draft: bool,
) -> (Assessment, Vec<String>, String) {
    if let Some(review) = review {
        return (
            review.decision,
            Vec::new(),
            "Re-review when material evidence changes.".to_owned(),
        );
    }
    if ai_draft {
        return (
            Assessment::Unreviewed,
            vec!["AI-authored material requires an explicit human review decision.".to_owned()],
            format!("Add a human review for {}.", claim.id),
        );
    }
    let mut blockers = vec!["Claim lacks an explicit human review decision.".to_owned()];
    if links.is_empty() {
        blockers.push("Claim has no evidence links.".to_owned());
    }
    let next_action = if links.is_empty() {
        format!("Add evidence for {}.", claim.id)
    } else {
        format!("Add a human review for {}.", claim.id)
    };
    (Assessment::Unreviewed, blockers, next_action)
}

fn experiment_statuses(experiments: Vec<ExperimentReceipt>) -> Vec<ExperimentStatus> {
    experiments
        .into_iter()
        .map(|experiment| ExperimentStatus {
            id: experiment.id,
            outcome: experiment.outcome,
            observations: experiment.observations,
            interpretation: experiment.interpretation,
            limitations: experiment.limitations,
            next_move: experiment.next_move,
        })
        .collect()
}

fn aggregate_actions(claims: &[ClaimStatus]) -> (Vec<String>, Vec<String>) {
    let blockers = claims
        .iter()
        .flat_map(|claim| claim.blockers.iter().cloned())
        .collect();
    let next_actions = claims
        .iter()
        .map(|claim| claim.next_action.clone())
        .collect();
    (blockers, next_actions)
}
