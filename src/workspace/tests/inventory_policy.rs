use super::*;
use crate::domain::{
    BoundaryEntry, DeclaredReference, DeclaredReferenceKind, InspectionStatus, InventoryBoundary,
    InventoryEntry, InventoryLimits, InventoryPolicy, InventorySnapshot, MaterialClass,
    RootNodeKind,
};

fn policy(boundaries: Vec<InventoryBoundary>) -> InventoryPolicy {
    InventoryPolicy {
        schema_version: 1,
        kind: "inventory-policy".to_owned(),
        limits: InventoryLimits::default(),
        boundaries,
    }
    .canonicalized()
    .expect("canonical policy")
}

fn reference_only(path: &str) -> InventoryBoundary {
    InventoryBoundary::ReferenceOnly {
        path: path.to_owned(),
        class: MaterialClass::Artifact,
        rationale: "Instrument-managed raw data".to_owned(),
        reference: Some(DeclaredReference {
            reference_kind: DeclaredReferenceKind::Manifest,
            locator_type: ArtifactLocatorType::External,
            locator: "instrument://run-001".to_owned(),
            description: "Operator-supplied manifest locator".to_owned(),
            sha256: None,
        }),
    }
}

#[test]
fn reference_only_root_is_visible_without_descendant_inspection() {
    let root = temporary();
    let raw = root.join("raw-data");
    fs::create_dir(&raw).expect("raw directory");
    let oversized = File::create(raw.join("instrument.bin")).expect("large fixture");
    oversized
        .set_len(3 * 1024 * 1024 * 1024)
        .expect("sparse large fixture");
    let selected = policy(vec![reference_only("raw-data")]);
    let plan = Workspace::plan_retrofit_with_policy(
        &root,
        "Reference boundary",
        "inventory-one",
        "2026-07-20T03:00:00Z",
        explicit_unanchored_retrofit(),
        Some(selected),
    )
    .expect("reference-only plan");
    let boundary = plan
        .entries
        .iter()
        .find_map(|entry| match entry {
            InventoryEntry::Boundary(BoundaryEntry::ReferenceOnly {
                root_observation,
                content_status,
                ..
            }) => Some((root_observation, content_status)),
            _ => None,
        })
        .expect("canonical reference boundary");
    assert_eq!(boundary.0.node_kind, RootNodeKind::Directory);
    assert_eq!(boundary.0.bytes, None);
    assert_eq!(*boundary.1, InspectionStatus::NotInspected);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn declarations_fail_closed_when_missing_overlapping_or_symlinked() {
    let root = temporary();
    fs::create_dir(root.join("data")).expect("data directory");
    fs::create_dir(root.join("data/raw")).expect("raw directory");
    let overlap = InventoryPolicy {
        boundaries: vec![reference_only("data"), reference_only("data/raw")],
        ..InventoryPolicy::default()
    };
    assert!(overlap.canonicalized().is_err());
    assert!(
        Workspace::plan_retrofit_with_policy(
            &root,
            "Missing boundary",
            "inventory-one",
            "2026-07-20T03:00:00Z",
            explicit_unanchored_retrofit(),
            Some(policy(vec![reference_only("missing")]))
        )
        .is_err()
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("data"), root.join("linked"))
            .expect("symlink boundary");
        assert!(
            Workspace::plan_retrofit_with_policy(
                &root,
                "Symlink boundary",
                "inventory-two",
                "2026-07-20T03:01:00Z",
                explicit_unanchored_retrofit(),
                Some(policy(vec![reference_only("linked")]))
            )
            .is_err()
        );
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn child_workspace_records_only_canonical_identity() {
    let root = temporary();
    let child = root.join("child");
    fs::create_dir(&child).expect("child root");
    fs::write(child.join("note.md"), b"child note").expect("child note");
    let child_workspace = Workspace::initialize(&child, "Child project").expect("child workspace");
    let child_plan = Workspace::plan_retrofit(
        &child,
        "Child project",
        "child-inventory",
        "2026-07-20T03:00:00Z",
        None,
    )
    .expect("child plan");
    Workspace::apply_inventory_plan(&child, child_plan).expect("child inventory");
    let child_manifest = child_workspace.read_manifest().expect("child manifest");

    let selected = policy(vec![InventoryBoundary::ChildWorkspace {
        path: "child".to_owned(),
        rationale: "Independent project authority".to_owned(),
    }]);
    let plan = Workspace::plan_retrofit_with_policy(
        &root,
        "Parent project",
        "parent-inventory",
        "2026-07-20T03:01:00Z",
        explicit_unanchored_retrofit(),
        Some(selected),
    )
    .expect("parent plan");
    let observed = plan
        .entries
        .iter()
        .find_map(|entry| match entry {
            InventoryEntry::Boundary(BoundaryEntry::ChildWorkspace {
                workspace_observation,
                child_material_status,
                ..
            }) => Some((workspace_observation, child_material_status)),
            _ => None,
        })
        .expect("child boundary entry");
    assert_eq!(observed.0.workspace_id, child_manifest.workspace_id);
    assert_eq!(
        observed
            .0
            .latest_inventory
            .as_ref()
            .map(|value| value.id.as_str()),
        Some("child-inventory")
    );
    assert_eq!(*observed.1, InspectionStatus::NotInspected);
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn undeclared_nested_workspace_and_malformed_identity_fail_closed() {
    let root = temporary();
    let child = root.join("child");
    Workspace::initialize(&child, "Child").expect("child workspace");
    assert!(
        Workspace::plan_retrofit(
            &root,
            "Parent",
            "inventory-one",
            "2026-07-20T03:00:00Z",
            explicit_unanchored_retrofit()
        )
        .is_err()
    );

    let malformed_root = temporary();
    let malformed_child = malformed_root.join("child/.research-run");
    fs::create_dir_all(&malformed_child).expect("malformed state");
    fs::write(malformed_child.join("manifest.json"), b"{").expect("malformed manifest");
    assert!(
        Workspace::plan_retrofit_with_policy(
            &malformed_root,
            "Parent",
            "inventory-one",
            "2026-07-20T03:00:00Z",
            explicit_unanchored_retrofit(),
            Some(policy(vec![InventoryBoundary::ChildWorkspace {
                path: "child".to_owned(),
                rationale: "Independent project authority".to_owned(),
            }]))
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
    fs::remove_dir_all(malformed_root).expect("remove malformed fixture");
}

#[test]
fn legacy_policy_is_inherited_until_explicit_migration() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Legacy project").expect("workspace");
    let manifest = workspace.read_manifest().expect("manifest");
    workspace
        .publish_record(
            "inventories",
            &InventorySnapshot {
                schema_version: 1,
                kind: "inventory".to_owned(),
                id: "legacy-inventory".to_owned(),
                project_name: manifest.name.clone(),
                project_id: manifest.project_id.clone(),
                observed_at: "2026-07-20T03:00:00Z".to_owned(),
                previous_snapshot_id: None,
                policy: None,
                entries: Vec::new(),
                changes: Vec::new(),
            },
        )
        .expect("legacy inventory");
    fs::write(root.join("note.md"), b"note").expect("note");
    let legacy = Workspace::plan_reconciliation(
        &root,
        &manifest.name,
        "legacy-next",
        "2026-07-20T03:01:00Z",
    )
    .expect("legacy reconciliation");
    assert!(legacy.policy.is_none());
    let migrated = Workspace::plan_reconciliation_with_policy(
        &root,
        &manifest.name,
        "policy-migration",
        "2026-07-20T03:02:00Z",
        Some(InventoryPolicy::default()),
    )
    .expect("policy migration");
    assert_eq!(migrated.policy, Some(InventoryPolicy::default()));
    fs::remove_dir_all(root).expect("remove fixture");
}
