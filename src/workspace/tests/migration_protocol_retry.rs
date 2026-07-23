use super::*;
use crate::domain::MigrationRecord;

#[test]
fn identical_migration_retry_recovers_its_pending_protocol() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration protocol retry").expect("initialize");
    let migration =
        Workspace::plan_migration(&root, "migration", "2026-07-18T20:00:00Z").expect("plan");
    Workspace::apply_migration(&root, migration).expect("seed migration");
    let protocol = workspace
        .state
        .join("contribution-protocols/agent-contribution.json");
    fs::remove_file(&protocol).expect("remove activation policy");
    let pending = protocol
        .parent()
        .expect("protocol directory")
        .join(".agent-contribution.json.999.0.tmp");
    fs::write(
        pending,
        crate::workspace::publication::canonical_json_bytes(
            &crate::domain::ContributionProtocol::agent_v1(),
        ),
    )
    .expect("stage interrupted activation retry");

    workspace
        .recover()
        .expect("recover typed migration activation retry");
    assert!(protocol.is_file());
    workspace
        .load_snapshot()
        .expect("validate recovered activation");

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn migration_detects_protocol_deletion_after_planning() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration deletion").expect("initialize");
    let migration =
        Workspace::plan_migration(&root, "migration", "2026-07-18T20:00:00Z").expect("plan");
    let protocol = workspace
        .state
        .join("contribution-protocols/agent-contribution.json");
    fs::remove_file(&protocol).expect("remove activation policy");

    assert!(Workspace::apply_migration(&root, migration).is_err());
    assert!(!workspace.state.join("migrations/migration.json").exists());
    assert!(
        !protocol.exists(),
        "stale migration repaired changed authority"
    );

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn identical_migration_repair_validates_references_before_activation() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Migration validation").expect("initialize");
    let protocol = workspace
        .state
        .join("contribution-protocols/agent-contribution.json");
    fs::remove_file(&protocol).expect("remove activation policy");
    fs::write(
        workspace.state.join("evidence/evidence-one.json"),
        r#"{
  "schema_version": 1,
  "kind": "evidence",
  "id": "evidence-one",
  "claim_id": "missing-claim",
  "source_id": null,
  "experiment_id": null,
  "artifact": "artifact.txt",
  "stance": "supports",
  "specific_evidence": "Dangling evidence",
  "authorship": "human"
}
"#,
    )
    .expect("write dangling evidence");
    let migration =
        Workspace::plan_migration(&root, "migration", "2026-07-18T20:00:00Z").expect("plan");
    fs::write(
        workspace.state.join("migrations/migration.json"),
        crate::workspace::publication::canonical_json_bytes(&MigrationRecord::from(
            migration.clone(),
        )),
    )
    .expect("seed canonical migration");

    assert!(Workspace::apply_migration(&root, migration).is_err());
    assert!(!protocol.exists(), "invalid authority was activated");

    fs::remove_dir_all(root).expect("remove fixture");
}
