use super::*;
use crate::domain::MigrationPlan;
use crate::workspace::migration::collect_authority;

fn plan(root: &std::path::Path, id: &str) -> MigrationPlan {
    Workspace::plan_migration(root, id, "2026-07-18T20:00:00Z").expect("migration plan")
}

#[test]
fn migration_planning_propagates_root_manifest_authority_and_validation_failures() {
    inject_storage_failure("read current directory");
    assert!(
        Workspace::plan_migration(
            std::path::Path::new("relative"),
            "migration",
            "2026-07-18T20:00:00Z"
        )
        .is_err()
    );
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration plan faults").expect("initialize");
    assert!(Workspace::plan_migration(&root, "INVALID", "2026-07-18T20:00:00Z").is_err());
    inject_storage_failure("read migration authority");
    assert!(Workspace::plan_migration(&root, "migration", "2026-07-18T20:00:00Z").is_err());
    fs::write(workspace.state.join("manifest.json"), b"{").expect("corrupt manifest");
    assert!(Workspace::plan_migration(&root, "migration", "2026-07-18T20:00:00Z").is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn migration_application_propagates_validation_root_and_directory_failures() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration apply faults").expect("initialize");
    let mut invalid = plan(&root, "migration-invalid");
    invalid.schema_version = 2;
    assert!(Workspace::apply_migration(&root, invalid).is_err());
    let valid = plan(&root, "migration-one");
    for directory in ["inventories", "knowledge", "relationships", "migrations"] {
        fs::remove_dir(workspace.state.join(directory)).expect("remove optional directory");
    }
    inject_storage_failure("create project directory");
    assert!(Workspace::apply_migration(&root, valid).is_err());
    fs::remove_dir_all(root).expect("remove fixture");

    inject_storage_failure("read current directory");
    assert!(Workspace::apply_migration(std::path::Path::new("relative"), placeholder()).is_err());
}

#[test]
fn migration_application_propagates_record_snapshot_and_publication_failures() {
    for point in ["inspect record", "publish recovered record"] {
        let root = temporary();
        Workspace::initialize(&root, "Migration record faults").expect("initialize");
        let migration = plan(&root, "migration-one");
        if point == "inspect record" {
            Workspace::apply_migration(&root, migration.clone()).expect("seed migration");
        }
        inject_storage_failure(point);
        assert!(
            Workspace::apply_migration(&root, migration).is_err(),
            "{point}"
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration snapshot fault").expect("initialize");
    fs::write(workspace.state.join("inventories/bad.json"), b"{}").expect("bad inventory");
    let migration = plan(&root, "migration-one");
    assert!(Workspace::apply_migration(&root, migration).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

fn placeholder() -> MigrationPlan {
    MigrationPlan {
        schema_version: 1,
        kind: "migration-plan".to_owned(),
        id: "migration".to_owned(),
        project_id: "project".to_owned(),
        from_format: "research-run-v0.1".to_owned(),
        to_format: "research-run-v0.1-extended".to_owned(),
        migrated_at: "2026-07-18T20:00:00Z".to_owned(),
        authority_files: 1,
        authority_bytes: 1,
        authority_sha256: "a".repeat(64),
    }
}

#[test]
fn migration_manifest_fingerprint_and_identical_record_failures_propagate() {
    for point in ["inspect record#2", "inspect record#3"] {
        let root = temporary();
        Workspace::initialize(&root, "Migration plan boundary").expect("initialize");
        inject_storage_failure(point);
        assert!(Workspace::plan_migration(&root, "migration", "2026-07-18T20:00:00Z").is_err());
        fs::remove_dir_all(root).expect("remove fixture");
    }
    for point in ["inspect record#2", "read migration authority"] {
        let root = temporary();
        Workspace::initialize(&root, "Migration apply boundary").expect("initialize");
        let migration = plan(&root, "migration");
        inject_storage_failure(point);
        assert!(
            Workspace::apply_migration(&root, migration).is_err(),
            "{point}"
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
    let root = temporary();
    Workspace::initialize(&root, "Migration identical boundary").expect("initialize");
    let migration = plan(&root, "migration");
    Workspace::apply_migration(&root, migration.clone()).expect("seed");
    inject_storage_failure("inspect record#5");
    assert!(Workspace::apply_migration(&root, migration).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn migration_propagates_contribution_protocol_publication_failure() {
    let root = temporary();
    Workspace::initialize(&root, "Migration activation").expect("initialize");
    let migration = plan(&root, "migration");
    fs::remove_file(root.join(".research-run/contribution-protocols/agent-contribution.json"))
        .expect("remove activation policy");
    inject_storage_failure("create pending record#2");
    assert!(Workspace::apply_migration(&root, migration).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn migration_interruption_never_publishes_a_protocol_only_state() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Atomic migration").expect("initialize");
    let migration = plan(&root, "migration");
    let protocol = workspace
        .state
        .join("contribution-protocols/agent-contribution.json");
    let migration_record = workspace.state.join("migrations/migration.json");
    fs::remove_file(&protocol).expect("remove activation policy");

    inject_storage_failure("publish recovered record");
    assert!(Workspace::apply_migration(&root, migration).is_err());
    assert!(
        !protocol.exists(),
        "failed migration activated the workspace"
    );
    assert!(
        !migration_record.exists(),
        "failed migration published its canonical record"
    );

    workspace.recover().expect("recover migration batch");
    assert!(protocol.is_file());
    assert!(migration_record.is_file());
    workspace
        .load_snapshot()
        .expect("validate recovered migration");

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn migration_recovery_publishes_protocol_before_record() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Ordered migration").expect("initialize");
    let migration = plan(&root, "migration");
    let protocol = workspace
        .state
        .join("contribution-protocols/agent-contribution.json");
    let migration_record = workspace.state.join("migrations/migration.json");
    fs::remove_file(&protocol).expect("remove activation policy");

    inject_storage_failure("publish recovered record#2");
    assert!(Workspace::apply_migration(&root, migration).is_err());
    assert!(protocol.is_file(), "migration published before activation");
    assert!(
        !migration_record.is_file(),
        "interruption left an activated migration record"
    );
    workspace
        .recover()
        .expect("recover after ordered migration interruption");
    assert!(protocol.is_file());
    assert!(migration_record.is_file());
    workspace
        .load_snapshot()
        .expect("validate recovered migration");

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn migration_validates_activation_directory_before_publication() {
    let root = temporary();
    let workspace =
        Workspace::initialize(&root, "Migration activation validation").expect("initialize");
    let migration = plan(&root, "migration");
    let protocol = workspace
        .state
        .join("contribution-protocols/agent-contribution.json");
    fs::remove_file(&protocol).expect("remove activation policy");
    fs::write(
        workspace
            .state
            .join("contribution-protocols/unexpected.json"),
        b"{}",
    )
    .expect("unexpected activation record");

    assert!(Workspace::apply_migration(&root, migration).is_err());
    assert!(
        !protocol.exists(),
        "migration mutated invalid activation state before validation"
    );

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn migration_plan_reader_and_authority_recursion_propagate_failures() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration read boundary").expect("initialize");
    let invalid = root.join("invalid-migration.json");
    fs::write(&invalid, b"{}").expect("invalid plan");
    inject_storage_failure("inspect workspace path");
    assert!(Workspace::read_migration_plan(&invalid).is_err());
    let mut semantic = placeholder();
    semantic.id = "INVALID".to_owned();
    fs::write(&invalid, serde_json::to_vec(&semantic).unwrap()).expect("semantic plan");
    assert!(Workspace::read_migration_plan(&invalid).is_err());
    assert!(Workspace::read_migration_plan(&invalid).is_err());
    fs::create_dir(workspace.state.join("nested-authority")).expect("nested authority");
    fs::write(workspace.state.join("nested-authority/record.json"), b"{}").expect("record");
    for point in ["inspect workspace path", "inspect workspace path#2"] {
        let mut paths = Vec::new();
        inject_storage_failure(point);
        assert!(collect_authority(&workspace.state, &workspace.state, &mut paths).is_err());
    }
    fs::remove_dir_all(root).expect("remove fixture");
}
