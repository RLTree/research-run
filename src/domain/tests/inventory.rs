use super::*;

fn entry(path: &str) -> MaterialEntry {
    MaterialEntry {
        path: path.to_owned(),
        class: MaterialClass::Note,
        bytes: 1,
        sha256: "a".repeat(64),
    }
}

fn inventory_plan() -> InventoryPlan {
    InventoryPlan {
        schema_version: 1,
        kind: "inventory-plan".to_owned(),
        id: "inventory-one".to_owned(),
        project_name: "Project".to_owned(),
        project_id: "project".to_owned(),
        workspace_id: "b".repeat(64),
        observed_at: "2026-07-18T20:00:00Z".to_owned(),
        previous_snapshot_id: None,
        review_authority: None,
        without_review_authority: true,
        policy: None,
        entries: vec![entry("notes/result.md").into()],
        changes: Vec::new(),
    }
}

#[test]
fn inventory_validation_covers_every_bounded_shape() {
    let plan = inventory_plan();
    assert!(plan.validate().is_ok());
    assert!(InventorySnapshot::from(plan.clone()).validate().is_ok());
    assert_invalid_plan_fields(&plan);
    assert_invalid_changes(&plan);

    let mut snapshot = InventorySnapshot::from(inventory_plan());
    snapshot.schema_version = 2;
    assert!(snapshot.validate().is_err());
    snapshot = InventorySnapshot::from(inventory_plan());
    snapshot.kind = "wrong".to_owned();
    assert!(snapshot.validate().is_err());
}

fn assert_invalid_plan_fields(plan: &InventoryPlan) {
    let mut invalid = plan.clone();
    invalid.schema_version = 2;
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    invalid.kind = "wrong".to_owned();
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    invalid.id = "INVALID".to_owned();
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    invalid.project_name.clear();
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    invalid.project_id = "INVALID".to_owned();
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    invalid.workspace_id = "INVALID".to_owned();
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    invalid.observed_at = "bad".to_owned();
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    indexed_mut(&mut invalid.entries[0]).path = "../escape".to_owned();
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    indexed_mut(&mut invalid.entries[0]).sha256 = "A".repeat(64);
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    invalid.previous_snapshot_id = Some(invalid.id.clone());
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    invalid.previous_snapshot_id = Some("INVALID".to_owned());
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    invalid.entries.push(invalid.entries[0].clone());
    assert!(invalid.validate().is_err());
    invalid = plan.clone();
    invalid.entries = vec![InventoryEntry::from(entry("note")); 2_049];
    assert!(invalid.validate().is_err());
    assert_invalid_authority_fields(plan);
}

fn assert_invalid_authority_fields(plan: &InventoryPlan) {
    let mut legacy = plan.clone();
    legacy.without_review_authority = false;
    legacy.workspace_id.clear();
    assert!(legacy.validate().is_ok());
    legacy.workspace_id = "INVALID".to_owned();
    assert!(legacy.validate().is_err());

    let mut explicit_unanchored = plan.clone();
    explicit_unanchored.workspace_id.clear();
    assert!(explicit_unanchored.validate().is_err());

    let mut invalid = plan.clone();
    invalid.review_authority = Some(ReviewAuthority {
        schema_version: 2,
        kind: "review-authority".to_owned(),
        id: "test-human".to_owned(),
        public_key: "invalid".to_owned(),
        fingerprint: "SHA256:invalid".to_owned(),
    });
    assert!(invalid.validate().is_err());
    invalid.without_review_authority = false;
    assert!(invalid.validate().is_err());
}

fn assert_invalid_changes(plan: &InventoryPlan) {
    for (before, after, detail) in [
        (Some("../before"), None, "detail"),
        (None, Some("../after"), "detail"),
        (None, None, "   "),
    ] {
        let mut changed = plan.clone();
        changed.changes.push(ReconciliationChange {
            kind: ReconciliationKind::Added,
            before: before.map(str::to_owned),
            after: after.map(str::to_owned),
            detail: detail.to_owned(),
        });
        assert!(changed.validate().is_err());
    }
    let mut invalid = plan.clone();
    invalid.changes = vec![
        ReconciliationChange {
            kind: ReconciliationKind::Added,
            before: None,
            after: Some("note".to_owned()),
            detail: "detail".to_owned(),
        };
        4_097
    ];
    assert!(invalid.validate().is_err());
}

fn indexed_mut(entry: &mut InventoryEntry) -> &mut MaterialEntry {
    match entry {
        InventoryEntry::Indexed(entry) => entry,
        InventoryEntry::Boundary(_) => panic!("fixture must be indexed"),
    }
}

#[test]
fn inventory_policy_is_versioned_typed_and_fail_closed() {
    let default = InventoryPolicy::default();
    assert!(default.validate().is_ok());
    let mut invalid = default.clone();
    invalid.schema_version = 2;
    assert!(invalid.validate().is_err());
    invalid = default.clone();
    invalid.limits.max_entries = 0;
    assert!(invalid.validate().is_err());
    invalid = default.clone();
    invalid.limits.max_file_bytes = invalid.limits.max_total_bytes + 1;
    assert!(invalid.validate().is_err());

    assert!(
        serde_json::from_str::<InventoryPolicy>(
            r#"{"schema_version":1,"kind":"inventory-policy","limits":{"max_entries":1,"max_file_bytes":1,"max_total_bytes":1},"boundaries":[],"unknown":true}"#
        )
        .is_err()
    );
    let escaping = InventoryPolicy {
        boundaries: vec![InventoryBoundary::ChildWorkspace {
            path: "../child".to_owned(),
            rationale: "Child".to_owned(),
        }],
        ..default
    };
    assert!(escaping.canonicalized().is_err());
}
