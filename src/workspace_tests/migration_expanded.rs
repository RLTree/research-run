use super::*;
use crate::domain::{
    EntityKind, EntityRef, KnowledgeKind, KnowledgeRecord, KnowledgeState, MigrationPlan,
    RelationshipKind, RelationshipRecord,
};
use crate::workspace::recovery_optional::preflight_optional;
use crate::workspace::storage::ReadBudget;

fn migration_plan(project_id: &str) -> MigrationPlan {
    MigrationPlan {
        schema_version: 1,
        kind: "migration-plan".to_owned(),
        id: "migration-one".to_owned(),
        project_id: project_id.to_owned(),
        from_format: "research-run-v0.1".to_owned(),
        to_format: "research-run-v0.1-extended".to_owned(),
        migrated_at: "2026-07-18T20:00:00Z".to_owned(),
        authority_files: 1,
        authority_bytes: 1,
        authority_sha256: "a".repeat(64),
    }
}

#[test]
fn migration_planning_and_application_fail_closed_at_authority_boundaries() {
    let missing = temporary();
    assert!(Workspace::plan_migration(&missing, "migration-one", "2026-07-18T20:00:00Z").is_err());
    assert!(Workspace::apply_migration(&missing, migration_plan("missing")).is_err());
    fs::remove_dir_all(missing).expect("remove missing");

    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration").expect("initialize");
    let plan =
        Workspace::plan_migration(&root, "migration-one", "2026-07-18T20:00:00Z").expect("plan");
    let path = root.join("migration-plan.json");
    fs::write(&path, serde_json::to_vec_pretty(&plan).unwrap()).expect("plan file");
    assert_eq!(
        Workspace::read_migration_plan(&path).expect("read plan"),
        plan
    );
    fs::remove_file(path).expect("remove plan");
    assert_migration_identity_failures(&root, &plan);
    assert!(Workspace::apply_migration(&root, plan.clone()).expect("apply"));
    assert!(!Workspace::apply_migration(&root, plan.clone()).expect("retry"));
    let mut other = plan;
    other.id = "migration-other".to_owned();
    other.migrated_at = "2026-07-18T20:01:00Z".to_owned();
    assert!(Workspace::apply_migration(&root, other).is_err());
    let malformed = root.join("plan.json");
    fs::write(&malformed, b"{").expect("malformed plan");
    assert!(Workspace::read_migration_plan(&malformed).is_err());
    fs::remove_file(malformed).expect("remove malformed");
    fs::write(workspace.state.join("unexpected.txt"), b"unexpected").expect("unexpected");
    assert!(Workspace::plan_migration(&root, "migration-two", "2026-07-18T20:01:00Z").is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

fn assert_migration_identity_failures(root: &std::path::Path, plan: &MigrationPlan) {
    let mut wrong = plan.clone();
    wrong.project_id = "other".to_owned();
    assert!(Workspace::apply_migration(root, wrong).is_err());
    let mut stale = plan.clone();
    stale.authority_sha256 = "b".repeat(64);
    assert!(Workspace::apply_migration(root, stale).is_err());
}

#[test]
fn migration_authority_propagates_storage_and_path_failures() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration faults").expect("initialize");
    for point in [
        "read migration authority",
        "read migration authority entry",
        "inspect migration authority",
        "inspect record",
        "open record",
        "inspect opened record",
        "read record",
        "reinspect record",
        "migration path UTF-8",
    ] {
        inject_storage_failure(point);
        assert!(
            Workspace::plan_migration(&root, "migration-one", "2026-07-18T20:00:00Z").is_err(),
            "{point}"
        );
    }
    let plan =
        Workspace::plan_migration(&root, "migration-one", "2026-07-18T20:00:00Z").expect("plan");
    inject_storage_failure("open workspace write lock");
    assert!(Workspace::apply_migration(&root, plan).is_err());
    assert_authority_symlink(&workspace, &root);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
fn assert_authority_symlink(workspace: &Workspace, root: &std::path::Path) {
    use std::os::unix::fs::symlink;
    let linked = workspace.state.join("linked.json");
    symlink(workspace.state.join("manifest.json"), &linked).expect("authority symlink");
    assert!(Workspace::plan_migration(root, "migration-linked", "2026-07-18T20:00:00Z").is_err());
    fs::remove_file(linked).expect("remove symlink");
}

#[cfg(not(unix))]
fn assert_authority_symlink(_workspace: &Workspace, _root: &std::path::Path) {}

#[test]
fn optional_recovery_accepts_legacy_absence_and_current_empty_directories() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Optional recovery").expect("initialize");
    for directory in ["inventories", "knowledge", "relationships", "migrations"] {
        fs::remove_dir(workspace.state.join(directory)).expect("remove optional directory");
    }
    assert!(workspace.load_snapshot().is_ok());
    let mut budget = ReadBudget::default();
    let legacy = preflight_optional(&workspace, &mut budget).expect("legacy recovery");
    assert!(legacy.inventories.is_empty());
    assert!(legacy.knowledge.is_empty());
    assert!(legacy.relationships.is_empty());
    assert!(legacy.migrations.is_empty());
    for directory in ["inventories", "knowledge", "relationships", "migrations"] {
        fs::create_dir(workspace.state.join(directory)).expect("current directory");
    }
    add_optional_records(&workspace);
    assert!(workspace.load_snapshot().is_ok());
    let mut budget = ReadBudget::default();
    let current = preflight_optional(&workspace, &mut budget).expect("current");
    assert_eq!(current.knowledge.len(), 2);
    assert_eq!(current.relationships.len(), 1);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn optional_recovery_propagates_each_collection_and_record_failure() {
    for occurrence in 1..=4 {
        let root = temporary();
        let workspace = Workspace::initialize(&root, "Optional recovery").expect("initialize");
        let point = occurrence_point("read recovery directory", occurrence);
        inject_storage_failure(point);
        assert!(preflight_optional(&workspace, &mut ReadBudget::default()).is_err());
        fs::remove_dir_all(root).expect("remove fixture");
    }
    for occurrence in 1..=4 {
        let root = temporary();
        let workspace = Workspace::initialize(&root, "Optional records").expect("initialize");
        let point = occurrence_point("read record directory", occurrence);
        inject_storage_failure(point);
        assert!(preflight_optional(&workspace, &mut ReadBudget::default()).is_err());
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

fn occurrence_point(base: &str, occurrence: usize) -> &'static str {
    match (base, occurrence) {
        ("read recovery directory", 1) => "read recovery directory",
        ("read recovery directory", 2) => "read recovery directory#2",
        ("read recovery directory", 3) => "read recovery directory#3",
        ("read recovery directory", _) => "read recovery directory#4",
        ("read record directory", 1) => "read record directory",
        ("read record directory", 2) => "read record directory#2",
        ("read record directory", 3) => "read record directory#3",
        _ => "read record directory#4",
    }
}

fn add_optional_records(workspace: &Workspace) {
    for id in ["knowledge-one", "knowledge-two"] {
        workspace
            .add_knowledge(&KnowledgeRecord {
                schema_version: 1,
                kind: "knowledge".to_owned(),
                id: id.to_owned(),
                record_type: KnowledgeKind::Observation,
                title: id.to_owned(),
                body: "Optional recovery".to_owned(),
                occurred_at: "2026-07-18T20:00:00Z".to_owned(),
                state: KnowledgeState::Open,
                authorship: Authorship::Human,
            })
            .expect("optional knowledge");
    }
    workspace
        .add_relationship(&RelationshipRecord {
            schema_version: 1,
            kind: "relationship".to_owned(),
            id: "relationship-one".to_owned(),
            relationship: RelationshipKind::RelatedTo,
            from: EntityRef {
                kind: EntityKind::Knowledge,
                id: "knowledge-one".to_owned(),
            },
            to: EntityRef {
                kind: EntityKind::Knowledge,
                id: "knowledge-two".to_owned(),
            },
            rationale: "Optional recovery".to_owned(),
            occurred_at: "2026-07-18T20:00:00Z".to_owned(),
            authorship: Authorship::Human,
        })
        .expect("optional relationship");
}
