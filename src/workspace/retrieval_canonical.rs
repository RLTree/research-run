use crate::domain::Assessment;

use super::references::current_review_bindings;
use super::retrieval_items::{ItemSpec, enum_name, item};
use super::retrieval_types::{ProjectionItem, RelationshipProjection};
use super::{Snapshot, Workspace};

pub(super) fn add_canonical(
    workspace: &Workspace,
    snapshot: &Snapshot,
    items: &mut Vec<ProjectionItem>,
) -> crate::Result<()> {
    let reviews = current_review_bindings(workspace, snapshot)?;
    add_sources(snapshot, items);
    add_claims(snapshot, &reviews, items);
    add_evidence(snapshot, items);
    add_experiments(snapshot, items);
    add_reviews(snapshot, &reviews, items);
    add_relationships(snapshot, items);
    Ok(())
}

pub(super) fn relationship_items(snapshot: &Snapshot) -> Vec<RelationshipProjection> {
    let mut items = snapshot
        .relationships
        .iter()
        .map(|record| RelationshipProjection {
            id: record.id.clone(),
            relationship: enum_name(&record.relationship),
            from_kind: enum_name(&record.from.kind),
            from_id: record.from.id.clone(),
            to_kind: enum_name(&record.to.kind),
            to_id: record.to.id.clone(),
            rationale: record.rationale.clone(),
            occurred_at: record.occurred_at.clone(),
            authority_path: format!(".research-run/relationships/{}.json", record.id),
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    items
}

fn add_sources(snapshot: &Snapshot, items: &mut Vec<ProjectionItem>) {
    for record in &snapshot.sources {
        items.push(item(ItemSpec {
            kind: "source",
            id: &record.id,
            subtype: &enum_name(&record.provenance),
            title: &record.citation,
            summary: &format!("{} {}", record.locator, record.notes),
            occurred_at: None,
            state: None,
            authority_path: &format!(".research-run/sources/{}.json", record.id),
            stale: false,
            invalidated: false,
        }));
    }
}

fn add_claims<'a>(
    snapshot: &'a Snapshot,
    reviews: &std::collections::BTreeMap<&'a str, &'a crate::domain::ReviewDecision>,
    items: &mut Vec<ProjectionItem>,
) {
    for record in &snapshot.claims {
        let assessment = reviews
            .get(record.id.as_str())
            .map(|review| review.decision)
            .unwrap_or(Assessment::Unreviewed);
        items.push(item(ItemSpec {
            kind: "claim",
            id: &record.id,
            subtype: "claim",
            title: &record.text,
            summary: &format!("{} Owner: {}", record.scope, record.owner),
            occurred_at: None,
            state: Some(&enum_name(&assessment)),
            authority_path: &format!(".research-run/claims/{}.json", record.id),
            stale: false,
            invalidated: false,
        }));
    }
}

fn add_evidence(snapshot: &Snapshot, items: &mut Vec<ProjectionItem>) {
    for record in &snapshot.evidence {
        let reference = record
            .source_id
            .as_deref()
            .map(|id| format!("source/{id}"))
            .or_else(|| {
                record
                    .experiment_id
                    .as_deref()
                    .map(|id| format!("experiment/{id}"))
            })
            .or_else(|| {
                record
                    .artifact
                    .as_deref()
                    .map(|path| format!("artifact/{path}"))
            })
            .expect("validated evidence has exactly one reference");
        items.push(item(ItemSpec {
            kind: "evidence",
            id: &record.id,
            subtype: &enum_name(&record.stance),
            title: &format!("Evidence for claim {}", record.claim_id),
            summary: &format!("{} {}", reference, record.specific_evidence),
            occurred_at: None,
            state: None,
            authority_path: &format!(".research-run/evidence/{}.json", record.id),
            stale: false,
            invalidated: false,
        }));
    }
}

fn add_experiments(snapshot: &Snapshot, items: &mut Vec<ProjectionItem>) {
    for record in &snapshot.experiments {
        items.push(item(ItemSpec {
            kind: "experiment",
            id: &record.id,
            subtype: &enum_name(&record.outcome),
            title: &record.question,
            summary: &format!(
                "{} {}",
                record.observations.join(" "),
                record.interpretation
            ),
            occurred_at: None,
            state: Some(&enum_name(&record.outcome)),
            authority_path: &format!(".research-run/experiments/{}.json", record.id),
            stale: false,
            invalidated: false,
        }));
    }
}

fn add_reviews<'a>(
    snapshot: &'a Snapshot,
    reviews: &std::collections::BTreeMap<&'a str, &'a crate::domain::ReviewDecision>,
    items: &mut Vec<ProjectionItem>,
) {
    let current_ids = reviews
        .values()
        .map(|review| review.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for record in &snapshot.reviews {
        items.push(item(ItemSpec {
            kind: "review",
            id: &record.id,
            subtype: "human-review-decision",
            title: &format!("Review of claim {}", record.claim_id),
            summary: &format!("{} Reviewer: {}", record.rationale, record.reviewer),
            occurred_at: None,
            state: Some(&enum_name(&record.decision)),
            authority_path: &format!(".research-run/reviews/{}.json", record.id),
            stale: !current_ids.contains(record.id.as_str()),
            invalidated: false,
        }));
    }
}

fn add_relationships(snapshot: &Snapshot, items: &mut Vec<ProjectionItem>) {
    for record in &snapshot.relationships {
        items.push(item(ItemSpec {
            kind: "relationship",
            id: &record.id,
            subtype: &enum_name(&record.relationship),
            title: &format!(
                "{}/{} to {}/{}",
                enum_name(&record.from.kind),
                record.from.id,
                enum_name(&record.to.kind),
                record.to.id
            ),
            summary: &record.rationale,
            occurred_at: Some(&record.occurred_at),
            state: None,
            authority_path: &format!(".research-run/relationships/{}.json", record.id),
            stale: false,
            invalidated: false,
        }));
    }
}
