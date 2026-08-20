use std::fs;

use super::{Workspace, temporary};

#[test]
fn handoff_validation_receipt_binds_result_context_and_project() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Receipt binding").unwrap();
    let bundle = workspace
        .handoff("handoff-receipt", "2026-08-20T20:00:00Z", None, 10)
        .unwrap();
    assert!(bundle.validate().is_ok());

    let mut changed_context = bundle.clone();
    changed_context.context.scope = "changed scope".to_owned();
    assert!(changed_context.validate().is_err());

    let mut changed_result = bundle.clone();
    changed_result
        .validation
        .as_mut()
        .unwrap()
        .result
        .counts
        .insert("knowledge".to_owned(), 1);
    assert!(changed_result.validate().is_err());

    let other_root = temporary();
    let other = Workspace::initialize(&other_root, "Other receipt workspace").unwrap();
    let mut cross_workspace = bundle;
    let other_bundle = other
        .handoff("handoff-other", "2026-08-20T20:01:00Z", None, 10)
        .unwrap();
    cross_workspace.context.project_id = other_bundle.context.project_id.clone();
    cross_workspace.validation = other_bundle.validation;
    assert!(cross_workspace.validate().is_err());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(other_root).unwrap();
}
