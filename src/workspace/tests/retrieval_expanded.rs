use super::*;
use crate::domain::{
    EntityKind, EntityRef, InventorySnapshot, KnowledgeKind, KnowledgeRecord, KnowledgeState,
    MaterialClass, MaterialEntry, RelationshipKind, RelationshipRecord,
};

fn knowledge(id: &str, kind: KnowledgeKind, state: KnowledgeState) -> KnowledgeRecord {
    KnowledgeRecord {
        schema_version: 1,
        kind: "knowledge".to_owned(),
        id: id.to_owned(),
        record_type: kind,
        title: id.to_owned(),
        body: "Retained observation".to_owned(),
        occurred_at: "2026-07-18T20:00:00Z".to_owned(),
        state,
        authorship: Authorship::Human,
    }
}

fn setup_retrieval(root: &std::path::Path) -> Workspace {
    fs::write(root.join("artifact.txt"), b"artifact").expect("artifact");
    let workspace = review_workspace(root, "Retrieval unit");
    workspace.add_source(&source("source-one")).expect("source");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    workspace
        .add_experiment(&experiment("experiment-one"))
        .expect("experiment");
    add_retrieval_evidence(&workspace);
    add_retrieval_knowledge(&workspace);
    add_invalidation(&workspace);
    add_inventories(&workspace);
    workspace
}

fn add_retrieval_evidence(workspace: &Workspace) {
    workspace
        .add_evidence(&evidence(
            "evidence-source",
            "claim-one",
            Some("source-one"),
        ))
        .expect("source evidence");
    let experiment_evidence = EvidenceLink {
        id: "evidence-experiment".to_owned(),
        source_id: None,
        experiment_id: Some("experiment-one".to_owned()),
        artifact: None,
        ..evidence("evidence-template", "claim-one", Some("source-one"))
    };
    workspace
        .add_evidence(&experiment_evidence)
        .expect("experiment evidence");
    workspace
        .add_evidence(&evidence("evidence-artifact", "claim-one", None))
        .expect("artifact evidence");
    let mut decision = review("review-one", "claim-one");
    decision.evidence_ids = vec![
        "evidence-artifact".to_owned(),
        "evidence-experiment".to_owned(),
        "evidence-source".to_owned(),
    ];
    decision.subject_sha256 = Some(
        workspace
            .review_subject_binding("claim-one")
            .expect("binding")
            .1,
    );
    authorize_review(workspace, &mut decision);
    workspace.add_review(&decision).expect("review");
}

fn add_retrieval_knowledge(workspace: &Workspace) {
    for (id, kind, state) in [
        (
            "question",
            KnowledgeKind::ResearchQuestion,
            KnowledgeState::Open,
        ),
        (
            "hypothesis",
            KnowledgeKind::Hypothesis,
            KnowledgeState::Active,
        ),
        ("risk", KnowledgeKind::Risk, KnowledgeState::Open),
        ("blocker", KnowledgeKind::Blocker, KnowledgeState::Open),
        (
            "uncertainty",
            KnowledgeKind::Uncertainty,
            KnowledgeState::Open,
        ),
        (
            "contradiction",
            KnowledgeKind::Contradiction,
            KnowledgeState::Open,
        ),
        (
            "next-action",
            KnowledgeKind::NextAction,
            KnowledgeState::Active,
        ),
        (
            "completed-risk",
            KnowledgeKind::Risk,
            KnowledgeState::Completed,
        ),
    ] {
        let mut record = knowledge(id, kind, state);
        if id == "question" {
            record.body = "q".repeat(600);
        }
        workspace
            .add_knowledge(&record)
            .expect("knowledge projection");
    }
}

fn add_invalidation(workspace: &Workspace) {
    let record = RelationshipRecord {
        schema_version: 1,
        kind: "relationship".to_owned(),
        id: "relationship-invalidation".to_owned(),
        relationship: RelationshipKind::Invalidates,
        from: EntityRef {
            kind: EntityKind::Knowledge,
            id: "contradiction".to_owned(),
        },
        to: EntityRef {
            kind: EntityKind::Knowledge,
            id: "question".to_owned(),
        },
        rationale: "Contradiction invalidates question framing".to_owned(),
        occurred_at: "2026-07-18T20:01:00Z".to_owned(),
        authorship: Authorship::Human,
    };
    workspace.add_relationship(&record).expect("invalidation");
}

fn add_inventories(workspace: &Workspace) {
    for (id, at, previous) in [
        ("inventory-old", "2026-07-18T20:00:00Z", None),
        (
            "inventory-new",
            "2026-07-18T20:01:00Z",
            Some("inventory-old".to_owned()),
        ),
    ] {
        let inventory = InventorySnapshot {
            schema_version: 1,
            kind: "inventory".to_owned(),
            id: id.to_owned(),
            project_name: "Retrieval unit".to_owned(),
            project_id: "retrieval-unit".to_owned(),
            observed_at: at.to_owned(),
            previous_snapshot_id: previous,
            entries: vec![MaterialEntry {
                path: "note.md".to_owned(),
                class: MaterialClass::Note,
                bytes: 1,
                sha256: "a".repeat(64),
            }],
            changes: Vec::new(),
        };
        workspace
            .publish_record("inventories", &inventory)
            .expect("inventory");
    }
}

#[test]
fn retrieval_projects_every_canonical_reference_and_history_state() {
    let root = temporary();
    let workspace = setup_retrieval(&root);
    assert_basic_retrieval(&workspace);
    assert_work_retrieval(&workspace);
    fs::remove_dir_all(root).expect("remove fixture");
}

fn assert_basic_retrieval(workspace: &Workspace) {
    assert!(workspace.list(None, 256).expect("list").total_matches > 10);
    assert_eq!(
        workspace
            .list(Some("risk"), 10)
            .expect("risk list")
            .items
            .len(),
        2
    );
    assert_eq!(
        workspace
            .show("evidence", "evidence-artifact")
            .expect("show")
            .kind,
        "evidence"
    );
    assert_eq!(
        workspace
            .show("review", "review-one")
            .expect("review")
            .state
            .as_deref(),
        Some("limited")
    );
    assert!(workspace.show("material", "note.md").is_err());
    let question = workspace.show("knowledge", "question").expect("question");
    assert_eq!(question.summary.len(), 512);
    assert!(question.invalidated);
    for query in ["question", "observation", "retained", ".research-run"] {
        assert!(
            !workspace
                .search(query, 10)
                .expect("search")
                .items
                .is_empty(),
            "{query}"
        );
    }
    assert!(workspace.search(&"x".repeat(257), 10).is_err());
    assert!(!workspace.recent(10).expect("recent").items.is_empty());
    assert!(!workspace.timeline(10).expect("timeline").items.is_empty());
}

fn assert_work_retrieval(workspace: &Workspace) {
    assert_eq!(workspace.unresolved(20).expect("unresolved").items.len(), 6);
    assert_eq!(
        workspace.blocker_items(10).expect("blockers").items.len(),
        1
    );
    assert_eq!(
        workspace.next_action_items(10).expect("next").items.len(),
        2
    );
    let claim_next = workspace
        .next_action_items(10)
        .expect("claim next")
        .items
        .into_iter()
        .find(|item| item.kind == "claim")
        .expect("reviewed claim next action");
    assert_eq!(claim_next.id, "claim-one");
    assert_eq!(claim_next.state.as_deref(), Some("limited"));
    assert_eq!(
        workspace
            .related("knowledge", "question", 10)
            .expect("related")
            .len(),
        1
    );
    assert!(
        workspace
            .related("knowledge", "missing", 10)
            .expect("none")
            .is_empty()
    );
    assert_eq!(
        workspace.context(None, 10).expect("context").scope,
        "recent workspace state"
    );
}
