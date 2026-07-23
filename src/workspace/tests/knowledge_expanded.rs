use super::*;
use crate::domain::{
    EntityKind, EntityRef, KnowledgeKind, KnowledgeRecord, KnowledgeState, RelationshipKind,
    RelationshipRecord,
};
use crate::workspace::MAX_RECORDS_PER_KIND;
use crate::workspace::knowledge::{ensure_capacity, relationship_reference_errors};

fn knowledge(id: &str) -> KnowledgeRecord {
    KnowledgeRecord {
        schema_version: 1,
        kind: "knowledge".to_owned(),
        id: id.to_owned(),
        record_type: KnowledgeKind::Observation,
        title: id.to_owned(),
        body: "Retained history".to_owned(),
        occurred_at: "2026-07-18T20:00:00Z".to_owned(),
        state: KnowledgeState::Open,
        authorship: Authorship::Human,
    }
}

fn reference(kind: EntityKind, id: &str) -> EntityRef {
    EntityRef {
        kind,
        id: id.to_owned(),
    }
}

fn relationship(id: &str, from: EntityRef, to: EntityRef) -> RelationshipRecord {
    RelationshipRecord {
        schema_version: 1,
        kind: "relationship".to_owned(),
        id: id.to_owned(),
        relationship: RelationshipKind::RelatedTo,
        from,
        to,
        rationale: "Typed relation".to_owned(),
        occurred_at: "2026-07-18T20:00:00Z".to_owned(),
        authorship: Authorship::Human,
    }
}

#[test]
fn knowledge_relations_resolve_every_entity_kind_and_remain_idempotent() {
    let root = temporary();
    fs::write(root.join("artifact.txt"), b"artifact").expect("artifact");
    let workspace = review_workspace(&root, "Relations");
    workspace.add_source(&source("source-one")).expect("source");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    workspace
        .add_experiment(&experiment("experiment-one"))
        .expect("experiment");
    workspace
        .add_evidence(&evidence("evidence-one", "claim-one", Some("source-one")))
        .expect("evidence");
    let mut decision = review("review-one", "claim-one");
    decision.evidence_ids = vec!["evidence-one".to_owned()];
    decision.subject_sha256 = Some(workspace.review_subject_binding("claim-one").unwrap().1);
    authorize_review(&workspace, &mut decision);
    workspace.add_review(&decision).expect("review");
    workspace
        .add_knowledge(&knowledge("knowledge-one"))
        .expect("knowledge");
    workspace
        .add_knowledge(&knowledge("knowledge-two"))
        .expect("second knowledge");
    assert!(
        !workspace
            .add_knowledge(&knowledge("knowledge-one"))
            .expect("retry")
    );
    super::knowledge_inventory::publish_inventory(&workspace);
    let entities = [
        (EntityKind::Source, "source-one"),
        (EntityKind::Claim, "claim-one"),
        (EntityKind::Evidence, "evidence-one"),
        (EntityKind::Experiment, "experiment-one"),
        (EntityKind::Review, "review-one"),
        (EntityKind::Knowledge, "knowledge-one"),
        (EntityKind::Inventory, "inventory-one"),
    ];
    for (index, (kind, id)) in entities.into_iter().enumerate() {
        let record = relationship(
            &format!("relationship-{index}"),
            reference(kind, id),
            reference(EntityKind::Knowledge, "knowledge-two"),
        );
        workspace.add_relationship(&record).expect("relationship");
        assert!(!workspace.add_relationship(&record).expect("retry"));
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn knowledge_mutations_propagate_validation_reference_lock_and_identity_failures() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Knowledge faults").expect("initialize");
    inject_storage_failure("open workspace write lock");
    assert!(
        workspace
            .add_knowledge(&knowledge("knowledge-lock"))
            .is_err()
    );
    let mut invalid = knowledge("knowledge-invalid");
    invalid.title.clear();
    assert!(workspace.add_knowledge(&invalid).is_err());
    workspace
        .add_knowledge(&knowledge("knowledge-one"))
        .expect("one");
    workspace
        .add_knowledge(&knowledge("knowledge-two"))
        .expect("two");
    let mut collision = knowledge("knowledge-one");
    collision.body = "different".to_owned();
    assert!(workspace.add_knowledge(&collision).is_err());
    assert_reference_failures(&workspace);
    fs::remove_dir_all(root).expect("remove fixture");
}

fn assert_reference_failures(workspace: &Workspace) {
    let unknown_from = relationship(
        "relationship-from",
        reference(EntityKind::Knowledge, "missing"),
        reference(EntityKind::Knowledge, "knowledge-two"),
    );
    assert!(workspace.add_relationship(&unknown_from).is_err());
    let unknown_to = relationship(
        "relationship-to",
        reference(EntityKind::Knowledge, "knowledge-one"),
        reference(EntityKind::Knowledge, "missing"),
    );
    assert!(workspace.add_relationship(&unknown_to).is_err());
    let valid = relationship(
        "relationship-valid",
        reference(EntityKind::Knowledge, "knowledge-one"),
        reference(EntityKind::Knowledge, "knowledge-two"),
    );
    workspace.add_relationship(&valid).expect("valid relation");
    let mut collision = valid;
    collision.rationale = "different".to_owned();
    assert!(workspace.add_relationship(&collision).is_err());

    let mut snapshot = workspace.load_snapshot().expect("snapshot");
    snapshot.relationships.push(unknown_from);
    snapshot.relationships.push(unknown_to);
    let errors = relationship_reference_errors(&snapshot);
    assert!(errors.iter().any(|error| error.contains("unknown from")));
    assert!(errors.iter().any(|error| error.contains("unknown to")));
    assert!(ensure_capacity(MAX_RECORDS_PER_KIND, "knowledge").is_err());
}

#[test]
fn knowledge_writers_propagate_snapshot_identity_capacity_and_publication_failures() {
    for point in [
        "inspect record",
        "corrupt snapshot",
        "knowledge capacity",
        "publish canonical record",
    ] {
        let root = temporary();
        let workspace = Workspace::initialize(&root, "Knowledge propagation").expect("init");
        if point == "inspect record" {
            workspace
                .add_knowledge(&knowledge("knowledge-one"))
                .expect("seed");
        }
        if point == "corrupt snapshot" {
            fs::write(workspace.state.join("manifest.json"), b"{").expect("corrupt");
        } else {
            inject_storage_failure(point);
        }
        let id = if point == "inspect record" {
            "knowledge-one"
        } else {
            "knowledge-new"
        };
        assert!(workspace.add_knowledge(&knowledge(id)).is_err(), "{point}");
        fs::remove_dir_all(root).expect("remove fixture");
    }
    for point in [
        "open workspace write lock",
        "corrupt snapshot",
        "inspect record",
        "relationship capacity",
        "publish canonical record",
    ] {
        let root = temporary();
        let workspace = Workspace::initialize(&root, "Relationship propagation").expect("init");
        workspace
            .add_knowledge(&knowledge("knowledge-one"))
            .expect("one");
        workspace
            .add_knowledge(&knowledge("knowledge-two"))
            .expect("two");
        let record = relationship(
            "relationship-one",
            reference(EntityKind::Knowledge, "knowledge-one"),
            reference(EntityKind::Knowledge, "knowledge-two"),
        );
        if point == "inspect record" {
            workspace.add_relationship(&record).expect("seed relation");
        }
        if point == "corrupt snapshot" {
            fs::write(workspace.state.join("manifest.json"), b"{").expect("corrupt");
        } else {
            inject_storage_failure(point);
        }
        assert!(workspace.add_relationship(&record).is_err(), "{point}");
        fs::remove_dir_all(root).expect("remove fixture");
    }
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Invalid relationship").expect("init");
    let invalid = relationship(
        "relationship-invalid",
        reference(EntityKind::Knowledge, "same"),
        reference(EntityKind::Knowledge, "same"),
    );
    assert!(workspace.add_relationship(&invalid).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn knowledge_and_relationship_identical_record_reads_propagate() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Knowledge identical").expect("initialize");
    let first = knowledge("knowledge-one");
    workspace.add_knowledge(&first).expect("first");
    inject_storage_failure("inspect record#4");
    assert!(workspace.add_knowledge(&first).is_err());
    let second = knowledge("knowledge-two");
    workspace.add_knowledge(&second).expect("second");
    let relation = relationship(
        "relationship-one",
        reference(EntityKind::Knowledge, "knowledge-one"),
        reference(EntityKind::Knowledge, "knowledge-two"),
    );
    workspace.add_relationship(&relation).expect("relation");
    inject_storage_failure("inspect record#6");
    assert!(workspace.add_relationship(&relation).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}
