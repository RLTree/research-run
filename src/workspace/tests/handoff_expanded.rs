use super::*;

#[test]
fn handoff_validation_rejects_every_identity_boundary() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Handoff validation").expect("initialize");
    let bundle = workspace
        .handoff("handoff-one", "2026-07-18T20:10:00Z", None, 10)
        .expect("handoff");
    for mutate in [
        |value: &mut crate::workspace::HandoffBundle| value.schema_version = 3,
        |value: &mut crate::workspace::HandoffBundle| value.kind = "unknown".to_owned(),
        |value: &mut crate::workspace::HandoffBundle| value.context.kind = "unknown".to_owned(),
        |value: &mut crate::workspace::HandoffBundle| value.id = "INVALID".to_owned(),
        |value: &mut crate::workspace::HandoffBundle| value.generated_at = "bad".to_owned(),
        |value: &mut crate::workspace::HandoffBundle| {
            value.context.project_id = "INVALID".to_owned()
        },
    ] {
        let mut invalid = bundle.clone();
        mutate(&mut invalid);
        assert!(invalid.validate().is_err());
    }
    let mut missing_protocol = bundle.clone();
    missing_protocol.context.contribution_protocol = None;
    assert!(missing_protocol.validate().is_err());
    let mut modified_protocol = bundle.clone();
    modified_protocol
        .context
        .contribution_protocol
        .as_mut()
        .expect("current handoff protocol")
        .claim_promotion = "agent-may-promote".to_owned();
    assert!(modified_protocol.validate().is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn handoff_validation_rejects_nested_forgery_and_budget_bypass() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Nested handoff").expect("initialize");
    workspace.add_claim(&claim("claim-one")).expect("claim");
    let bundle = workspace
        .handoff("handoff-nested", "2026-07-18T20:10:00Z", None, 10)
        .expect("handoff");
    let mut forged = bundle.clone();
    forged.context.claim_ceiling = "forged authority".to_owned();
    assert!(forged.validate().is_err());
    let mut invalid_item = bundle.clone();
    invalid_item.context.matches[0].authority_path = "../escape".to_owned();
    assert!(invalid_item.validate().is_err());
    let mut invalid_match = bundle.clone();
    invalid_match.context.matches[0].matched_by = vec!["unknown".to_owned()];
    assert!(invalid_match.validate().is_err());
    let mut excessive = bundle.clone();
    excessive.context.matches = vec![bundle.context.matches[0].clone(); 257];
    assert!(excessive.validate().is_err());
    let mut relationship = bundle;
    relationship
        .context
        .relationships
        .push(crate::workspace::RelationshipProjection {
            id: "relationship-one".to_owned(),
            relationship: "unknown".to_owned(),
            from_kind: "claim".to_owned(),
            from_id: "claim-one".to_owned(),
            to_kind: "claim".to_owned(),
            to_id: "claim-two".to_owned(),
            rationale: "Forged relation".to_owned(),
            occurred_at: "2026-07-18T20:10:00Z".to_owned(),
            authority_path: ".research-run/relationships/relationship-one.json".to_owned(),
        });
    assert!(relationship.validate().is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}
