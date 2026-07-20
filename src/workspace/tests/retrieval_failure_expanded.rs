use super::*;

#[test]
fn every_retrieval_route_propagates_limits_and_snapshot_failures() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Retrieval faults").expect("initialize");
    assert!(workspace.list(None, 0).is_err());
    assert!(workspace.search("query", 0).is_err());
    assert!(workspace.recent(0).is_err());
    assert!(workspace.timeline(0).is_err());
    assert!(workspace.related("knowledge", "id", 0).is_err());
    assert!(workspace.unresolved(0).is_err());
    assert!(workspace.blocker_items(0).is_err());
    assert!(workspace.next_action_items(0).is_err());
    assert!(workspace.context(None, 0).is_err());
    fs::write(workspace.state.join("manifest.json"), b"{").expect("corrupt manifest");
    assert!(workspace.list(None, 10).is_err());
    assert!(workspace.show("knowledge", "id").is_err());
    assert!(workspace.search("query", 10).is_err());
    assert!(workspace.recent(10).is_err());
    assert!(workspace.timeline(10).is_err());
    assert!(workspace.related("knowledge", "id", 10).is_err());
    assert!(workspace.unresolved(10).is_err());
    assert!(workspace.blocker_items(10).is_err());
    assert!(workspace.next_action_items(10).is_err());
    assert!(workspace.context(None, 10).is_err());
    assert!(
        workspace
            .handoff("handoff", "2026-07-18T20:00:00Z", None, 10)
            .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn handoff_propagates_bundle_validation_failure() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Invalid handoff").expect("initialize");
    assert!(
        workspace
            .handoff("INVALID", "2026-07-18T20:00:00Z", None, 10)
            .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}
