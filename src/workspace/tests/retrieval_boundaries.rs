use std::fs;

use crate::workspace::ProjectionItem;

fn projection(id: &str) -> ProjectionItem {
    ProjectionItem {
        kind: "knowledge".to_owned(),
        id: id.to_owned(),
        subtype: "blocker".to_owned(),
        title: "Title".to_owned(),
        summary: "Summary".to_owned(),
        occurred_at: None,
        state: Some("open".to_owned()),
        authority_path: format!(".research-run/knowledge/{id}.json"),
        matched_by: Vec::new(),
        stale: false,
        invalidated: false,
    }
}

#[test]
fn projection_sort_deduplicates_and_truncates() {
    let mut items = vec![
        projection("second"),
        projection("first"),
        projection("first"),
    ];
    super::super::retrieval_context::sort_and_truncate(&mut items, 1);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "first");
}

#[test]
fn derived_work_views_propagate_invalid_reference_status() {
    let root = super::temporary();
    let workspace = super::Workspace::initialize(&root, "Invalid retrieval status").unwrap();
    let damaged = super::evidence("evidence-one", "missing-claim", None);
    fs::write(
        workspace.state.join("evidence/evidence-one.json"),
        serde_json::to_vec_pretty(&damaged).unwrap(),
    )
    .unwrap();
    assert!(workspace.blocker_items(10).is_err());
    assert!(workspace.next_action_items(10).is_err());
    assert!(workspace.context(None, 10).is_err());
    fs::remove_dir_all(root).unwrap();
}
