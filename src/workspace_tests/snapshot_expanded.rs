use super::*;
use crate::domain::KnowledgeRecord;
use crate::workspace::storage::ReadBudget;

#[test]
fn expanded_optional_snapshot_loads_propagate_owned_failures() {
    for directory in ["knowledge", "migrations"] {
        let root = temporary();
        let workspace = Workspace::initialize(&root, "Snapshot failure").expect("initialize");
        fs::write(workspace.state.join(directory).join("bad.json"), b"{}").expect("bad record");
        assert!(workspace.load_snapshot().is_err(), "{directory}");
        fs::remove_dir_all(root).expect("remove fixture");
    }
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Optional path failure").expect("initialize");
    inject_storage_failure("inspect workspace path");
    assert!(
        workspace
            .load_optional_records::<KnowledgeRecord>(
                "knowledge",
                false,
                &mut ReadBudget::default()
            )
            .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}
