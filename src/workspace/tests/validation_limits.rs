use std::fs;

use super::*;

#[test]
fn validation_reports_a_deterministic_error_budget_overflow() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Validation error budget").expect("initialize");
    let mut snapshot = workspace.load_snapshot().expect("snapshot");
    for index in 0..257 {
        snapshot.evidence.push(EvidenceLink {
            schema_version: 1,
            kind: "evidence".to_owned(),
            id: format!("evidence-{index:03}"),
            claim_id: "missing-claim".to_owned(),
            source_id: None,
            experiment_id: None,
            artifact: None,
            stance: Stance::Limits,
            specific_evidence: "Invalid reference for validation budget coverage".to_owned(),
            authorship: Authorship::Human,
        });
    }

    let result = workspace.validation_from_snapshot(&snapshot);
    assert!(!result.valid);
    assert_eq!(result.errors.len(), 256);
    assert_eq!(
        result.errors[254],
        "evidence/evidence-254: unknown claim reference"
    );
    assert_eq!(
        result.errors[255],
        "validation error budget exceeded: 2 additional diagnostics omitted"
    );
    fs::remove_dir_all(root).expect("remove fixture");
}
