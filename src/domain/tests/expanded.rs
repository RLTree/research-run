use super::*;

fn digest() -> String {
    "a".repeat(64)
}

fn knowledge() -> KnowledgeRecord {
    KnowledgeRecord {
        schema_version: 1,
        kind: "knowledge".to_owned(),
        id: "knowledge-one".to_owned(),
        record_type: KnowledgeKind::Observation,
        title: "Observed result".to_owned(),
        body: "Negative and retained".to_owned(),
        occurred_at: "2026-07-18T20:00:00Z".to_owned(),
        state: KnowledgeState::Open,
        authorship: Authorship::Human,
    }
}

fn relationship(kind: RelationshipKind) -> RelationshipRecord {
    RelationshipRecord {
        schema_version: 1,
        kind: "relationship".to_owned(),
        id: "relationship-one".to_owned(),
        relationship: kind,
        from: EntityRef {
            kind: EntityKind::Knowledge,
            id: "knowledge-one".to_owned(),
        },
        to: EntityRef {
            kind: EntityKind::Knowledge,
            id: "knowledge-two".to_owned(),
        },
        rationale: "Typed connection".to_owned(),
        occurred_at: "2026-07-18T20:00:00Z".to_owned(),
        authorship: Authorship::Human,
    }
}

#[test]
fn knowledge_and_relationship_validation_cover_failure_classes() {
    let mut record = knowledge();
    assert!(record.validate().is_ok());
    record.schema_version = 2;
    assert!(record.validate().is_err());
    record = knowledge();
    record.kind = "wrong".to_owned();
    assert!(record.validate().is_err());
    record = knowledge();
    record.id = "INVALID".to_owned();
    assert!(record.validate().is_err());
    record = knowledge();
    record.title.clear();
    assert!(record.validate().is_err());
    record = knowledge();
    record.body = "x".repeat(MAX_TEXT_BYTES + 1);
    assert!(record.validate().is_err());
    record = knowledge();
    record.occurred_at = "not-a-time".to_owned();
    assert!(record.validate().is_err());

    for kind in [
        RelationshipKind::RelatedTo,
        RelationshipKind::DependsOn,
        RelationshipKind::DerivedFrom,
        RelationshipKind::Uses,
        RelationshipKind::Supersedes,
        RelationshipKind::Revises,
        RelationshipKind::Invalidates,
        RelationshipKind::Resolves,
        RelationshipKind::Blocks,
        RelationshipKind::Contradicts,
    ] {
        assert!(relationship(kind).validate().is_ok());
    }
    let mut invalid = relationship(RelationshipKind::RelatedTo);
    invalid.schema_version = 2;
    assert!(invalid.validate().is_err());
    invalid = relationship(RelationshipKind::RelatedTo);
    invalid.kind = "wrong".to_owned();
    assert!(invalid.validate().is_err());
    invalid = relationship(RelationshipKind::RelatedTo);
    invalid.id = "INVALID".to_owned();
    assert!(invalid.validate().is_err());
    invalid = relationship(RelationshipKind::RelatedTo);
    invalid.from.id = "INVALID".to_owned();
    assert!(invalid.validate().is_err());
    invalid = relationship(RelationshipKind::RelatedTo);
    invalid.to.id = "INVALID".to_owned();
    assert!(invalid.validate().is_err());
    invalid = relationship(RelationshipKind::RelatedTo);
    invalid.to = invalid.from.clone();
    assert!(invalid.validate().is_err());
    invalid = relationship(RelationshipKind::Supersedes);
    invalid.from.kind = EntityKind::Source;
    assert!(invalid.validate().is_err());
    invalid = relationship(RelationshipKind::RelatedTo);
    invalid.rationale.clear();
    assert!(invalid.validate().is_err());
}

fn migration() -> MigrationPlan {
    MigrationPlan {
        schema_version: 1,
        kind: "migration-plan".to_owned(),
        id: "migration-one".to_owned(),
        project_id: "project".to_owned(),
        from_format: "research-run-v0.1".to_owned(),
        to_format: "research-run-v0.1-extended".to_owned(),
        migrated_at: "2026-07-18T20:00:00Z".to_owned(),
        authority_files: 1,
        authority_bytes: 1,
        authority_sha256: digest(),
    }
}

#[test]
fn migration_and_timestamp_validation_cover_every_boundary() {
    let plan = migration();
    assert!(plan.validate().is_ok());
    assert!(MigrationRecord::from(plan.clone()).validate().is_ok());
    let mut invalid_record = MigrationRecord::from(plan.clone());
    invalid_record.schema_version = 2;
    assert!(invalid_record.validate().is_err());
    for mutate in [
        |value: &mut MigrationPlan| value.schema_version = 2,
        |value: &mut MigrationPlan| value.kind = "wrong".to_owned(),
        |value: &mut MigrationPlan| value.id = "INVALID".to_owned(),
        |value: &mut MigrationPlan| value.project_id = "INVALID".to_owned(),
        |value: &mut MigrationPlan| value.from_format = "unknown".to_owned(),
        |value: &mut MigrationPlan| value.to_format = "unknown".to_owned(),
        |value: &mut MigrationPlan| value.migrated_at = "bad".to_owned(),
        |value: &mut MigrationPlan| value.authority_files = 0,
        |value: &mut MigrationPlan| value.authority_bytes = 0,
        |value: &mut MigrationPlan| value.authority_sha256 = "A".repeat(64),
        |value: &mut MigrationPlan| value.authority_sha256 = "a".repeat(63),
    ] {
        let mut invalid = plan.clone();
        mutate(&mut invalid);
        assert!(invalid.validate().is_err());
    }
    for timestamp in [
        "2026-00-18T20:00:00Z",
        "2026-13-18T20:00:00Z",
        "2026-07-00T20:00:00Z",
        "2026-07-32T20:00:00Z",
        "2026-07-18T24:00:00Z",
        "2026-07-18T20:60:00Z",
        "2026-07-18T20:00:60Z",
    ] {
        assert!(validate_timestamp(timestamp).is_err(), "{timestamp}");
    }
}
