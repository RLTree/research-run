use super::*;

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
