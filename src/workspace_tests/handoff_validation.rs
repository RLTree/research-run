use super::*;
use crate::workspace::{CLAIM_CEILING, HandoffBundle, ProjectionItem, RelationshipProjection};

fn item(kind: &str) -> ProjectionItem {
    ProjectionItem {
        kind: kind.to_owned(),
        id: "item-one".to_owned(),
        subtype: "type".to_owned(),
        title: "Title".to_owned(),
        summary: "Summary".to_owned(),
        occurred_at: Some("2026-07-18T20:10:00Z".to_owned()),
        state: Some("open".to_owned()),
        authority_path: ".research-run/knowledge/item-one.json".to_owned(),
        matched_by: vec!["id".to_owned()],
        stale: false,
        invalidated: false,
    }
}

fn relationship() -> RelationshipProjection {
    RelationshipProjection {
        id: "relationship-one".to_owned(),
        relationship: "related-to".to_owned(),
        from_kind: "source".to_owned(),
        from_id: "source-one".to_owned(),
        to_kind: "claim".to_owned(),
        to_id: "claim-one".to_owned(),
        rationale: "Current relationship".to_owned(),
        occurred_at: "2026-07-18T20:10:00Z".to_owned(),
        authority_path: ".research-run/relationships/relationship-one.json".to_owned(),
    }
}

fn bundle() -> HandoffBundle {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Handoff branches").expect("initialize");
    let bundle = workspace
        .handoff("handoff-one", "2026-07-18T20:10:00Z", None, 10)
        .expect("handoff");
    fs::remove_dir_all(root).expect("remove fixture");
    bundle
}

#[test]
fn projection_validation_covers_nested_kinds_and_budgets() {
    for kind in ["inventory", "material"] {
        let mut value = bundle();
        let mut projection = item(kind);
        if kind == "material" {
            projection.id = "notes/result.md".to_owned();
        }
        value.context.matches.push(projection);
        assert!(value.validate().is_ok(), "{kind}");
    }
    for mutate in [
        |value: &mut ProjectionItem| value.kind = "unknown".to_owned(),
        |value: &mut ProjectionItem| value.id = "INVALID".to_owned(),
        |value: &mut ProjectionItem| value.summary = "x".repeat(513),
        |value: &mut ProjectionItem| value.occurred_at = Some("bad".to_owned()),
        |value: &mut ProjectionItem| value.state = Some(String::new()),
        |value: &mut ProjectionItem| value.matched_by = vec!["id".to_owned(); 6],
        |value: &mut ProjectionItem| value.matched_by = vec!["unknown".to_owned()],
        |value: &mut ProjectionItem| value.authority_path = "notes/result.md".to_owned(),
        |value: &mut ProjectionItem| value.title = "x".repeat(crate::domain::MAX_TEXT_BYTES + 1),
        |value: &mut ProjectionItem| value.subtype = "x".repeat(crate::domain::MAX_TEXT_BYTES + 1),
    ] {
        let mut value = bundle();
        let mut projection = item("knowledge");
        mutate(&mut projection);
        value.context.matches.push(projection);
        assert!(value.validate().is_err());
    }
    let mut invalid_material = bundle();
    let mut material = item("material");
    material.id = "../escape".to_owned();
    invalid_material.context.matches.push(material);
    assert!(invalid_material.validate().is_err());
}

#[test]
fn relationship_validation_covers_nested_identity_and_budgets() {
    let mut valid = bundle();
    valid.context.relationships.push(relationship());
    assert!(valid.validate().is_ok());
    for mutate in [
        |value: &mut RelationshipProjection| value.relationship = "unknown".to_owned(),
        |value: &mut RelationshipProjection| value.id = "INVALID".to_owned(),
        |value: &mut RelationshipProjection| value.from_kind = "unknown".to_owned(),
        |value: &mut RelationshipProjection| value.to_id = "INVALID".to_owned(),
        |value: &mut RelationshipProjection| {
            value.to_kind = value.from_kind.clone();
            value.to_id = value.from_id.clone();
        },
        |value: &mut RelationshipProjection| value.rationale = String::new(),
        |value: &mut RelationshipProjection| value.occurred_at = "bad".to_owned(),
        |value: &mut RelationshipProjection| value.authority_path = "escape.json".to_owned(),
    ] {
        let mut handoff = bundle();
        let mut relation = relationship();
        mutate(&mut relation);
        handoff.context.relationships.push(relation);
        assert!(handoff.validate().is_err());
    }
    let mut excessive = bundle();
    excessive.context.relationships = vec![relationship(); crate::domain::MAX_LIST_ITEMS + 1];
    assert!(excessive.validate().is_err());
    let mut invalid_project_name = bundle();
    invalid_project_name.context.project_name = String::new();
    assert!(invalid_project_name.validate().is_err());
    let mut excessive_project_name = bundle();
    excessive_project_name.context.project_name = "x".repeat(crate::domain::MAX_TEXT_BYTES + 1);
    assert!(excessive_project_name.validate().is_err());
    let mut invalid_scope = bundle();
    invalid_scope.context.scope = String::new();
    assert!(invalid_scope.validate().is_err());
    assert_eq!(bundle().context.claim_ceiling, CLAIM_CEILING);
}
