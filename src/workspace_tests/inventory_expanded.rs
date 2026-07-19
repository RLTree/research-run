use super::*;
use crate::domain::{MaterialClass, MaterialEntry, ReconciliationKind};
use crate::workspace::inventory_reconcile::reconcile;
use crate::workspace::inventory_scan::{
    add_inventory_bytes, classify_material, enforce_file_count, scan_materials,
};

fn digest(byte: char) -> String {
    byte.to_string().repeat(64)
}

fn material(path: &str, byte: char) -> MaterialEntry {
    MaterialEntry {
        path: path.to_owned(),
        class: MaterialClass::Note,
        bytes: 1,
        sha256: digest(byte),
    }
}

#[test]
fn reconciliation_classifies_every_change_without_inference() {
    let previous = vec![
        material("same", 'a'),
        material("changed", 'b'),
        material("old-move", 'c'),
        material("ambiguous-one", 'd'),
        material("ambiguous-two", 'd'),
        material("missing", 'e'),
    ];
    let current = vec![
        material("same", 'a'),
        material("changed", 'f'),
        material("new-move", 'c'),
        material("ambiguous-new", 'd'),
        material("added", 'g'),
        material("duplicate-one", 'h'),
        material("duplicate-two", 'h'),
    ];
    let changes = reconcile(&previous, &current);
    for kind in [
        ReconciliationKind::Added,
        ReconciliationKind::Changed,
        ReconciliationKind::Moved,
        ReconciliationKind::Missing,
        ReconciliationKind::Duplicate,
        ReconciliationKind::Conflict,
    ] {
        assert!(changes.iter().any(|change| change.kind == kind), "{kind:?}");
    }
}

#[test]
fn reconciliation_distinguishes_each_fingerprint_and_consumes_only_exact_moves() {
    let unchanged = material("same", 'a');
    assert!(reconcile(std::slice::from_ref(&unchanged), &[unchanged.clone()]).is_empty());

    let mut hash_changed = material("same", 'a');
    hash_changed.sha256 = digest('b');
    assert_eq!(
        reconcile(&[material("same", 'a')], &[hash_changed])[0].kind,
        ReconciliationKind::Changed
    );
    let mut size_changed = material("same", 'a');
    size_changed.bytes = 2;
    assert_eq!(
        reconcile(&[material("same", 'a')], &[size_changed])[0].kind,
        ReconciliationKind::Changed
    );

    let moved = reconcile(&[material("old", 'c')], &[material("new", 'c')]);
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].kind, ReconciliationKind::Moved);
    let ambiguous = reconcile(
        &[material("old-one", 'd'), material("old-two", 'd')],
        &[material("new", 'd')],
    );
    assert_eq!(
        ambiguous
            .iter()
            .filter(|change| change.kind == ReconciliationKind::Conflict)
            .count(),
        2
    );
    assert_eq!(ambiguous.len(), 5);
}

#[test]
fn inventory_count_and_byte_budgets_accept_the_limit_and_reject_the_next_unit() {
    assert!(enforce_file_count(crate::domain::MAX_INVENTORY_ENTRIES).is_ok());
    assert!(enforce_file_count(crate::domain::MAX_INVENTORY_ENTRIES + 1).is_err());
    let limit = 512 * 1_048_576;
    assert_eq!(add_inventory_bytes(0, limit).expect("exact limit"), limit);
    assert!(add_inventory_bytes(limit, 1).is_err());
    assert!(add_inventory_bytes(u64::MAX, 1).is_err());
}

#[test]
fn material_classification_covers_each_declared_class() {
    for (path, expected) in [
        ("protocols/assay.md", MaterialClass::Protocol),
        ("methods/assay.txt", MaterialClass::Protocol),
        ("sources/paper.pdf", MaterialClass::Source),
        ("references.bib", MaterialClass::Source),
        ("literature.csv", MaterialClass::Source),
        ("experiment-one.csv", MaterialClass::Experiment),
        ("run-001.dat", MaterialClass::Experiment),
        ("observations.txt", MaterialClass::Observation),
        ("results.csv", MaterialClass::Observation),
        ("analysis.csv", MaterialClass::Analysis),
        ("decision.md", MaterialClass::Decision),
        ("plan.md", MaterialClass::Plan),
        ("talk.pptx", MaterialClass::Presentation),
        ("presentation.pdf", MaterialClass::Presentation),
        ("notes.md", MaterialClass::Note),
        ("image.png", MaterialClass::Artifact),
        ("README", MaterialClass::Unknown),
    ] {
        assert_eq!(classify_material(path), expected, "{path}");
    }
}

#[test]
fn material_scan_propagates_storage_failures_and_budgets() {
    let root = temporary();
    fs::create_dir(root.join(".git")).expect("git fixture");
    fs::create_dir(root.join(".research-run")).expect("state fixture");
    fs::write(root.join("note.md"), b"note").expect("note fixture");
    fs::write(root.join(".git/ignored"), b"ignored").expect("git bytes");
    fs::write(root.join(".research-run/ignored"), b"ignored").expect("state bytes");
    assert_eq!(scan_materials(&root).expect("scan").1.len(), 1);
    for point in [
        "read retrofit directory",
        "read retrofit entry",
        "inspect retrofit entry",
        "inspect material",
        "open material",
        "hash material",
        "reinspect material",
        "material grew",
        "material identity",
        "inventory byte budget",
        "material path UTF-8",
        "material pre-hash path",
        "material post-hash path",
    ] {
        inject_storage_failure(point);
        assert!(scan_materials(&root).is_err(), "{point}");
    }
    assert!(scan_materials(&root.join("note.md")).is_err());
    File::create(root.join("large.bin"))
        .expect("large fixture")
        .set_len(64 * 1_048_576 + 1)
        .expect("large length");
    assert!(scan_materials(&root).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn material_scan_enforces_file_count_budget() {
    let root = temporary();
    for index in 0..=crate::domain::MAX_INVENTORY_ENTRIES {
        fs::write(root.join(format!("file-{index:04}")), b"").expect("file fixture");
    }
    assert!(scan_materials(&root).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn material_scan_rejects_unsupported_entries() {
    let root = temporary();
    let status = std::process::Command::new("mkfifo")
        .arg(root.join("pipe"))
        .status()
        .expect("mkfifo");
    assert!(status.success());
    assert!(scan_materials(&root).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_lifecycle_rejects_identity_conflicts_and_stale_authority() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    assert!(
        Workspace::plan_reconciliation(&root, "Project", "reconcile", "2026-07-18T20:00:00Z")
            .is_err()
    );
    let plan = Workspace::plan_retrofit(&root, "Project", "inventory-one", "2026-07-18T20:00:00Z")
        .expect("plan");
    let plan_path = root.join("plan.json");
    fs::write(&plan_path, serde_json::to_vec_pretty(&plan).unwrap()).expect("plan file");
    assert_eq!(
        Workspace::read_inventory_plan(&plan_path).expect("read"),
        plan
    );
    fs::remove_file(plan_path).expect("remove plan");
    assert!(Workspace::apply_inventory_plan(&root, plan).expect("apply"));
    assert!(Workspace::plan_retrofit(&root, "Project", "again", "2026-07-18T20:01:00Z").is_err());
    assert!(
        Workspace::plan_reconciliation(&root, "Wrong", "wrong", "2026-07-18T20:01:00Z").is_err()
    );
    assert_inventory_conflict(&root);
    assert_stale_inventory(&root);
    fs::remove_dir_all(root).expect("remove fixture");
    let blocked = temporary();
    fs::write(blocked.join(".research-run"), b"file").expect("blocked state");
    assert!(Workspace::at_exact_root(&blocked).is_err());
    fs::remove_dir_all(blocked).expect("remove blocked");
}

fn assert_inventory_conflict(root: &std::path::Path) {
    let mut plan = Workspace::plan_reconciliation(
        root,
        "Project",
        "inventory-conflict",
        "2026-07-18T20:01:00Z",
    )
    .expect("base");
    plan.changes.push(crate::domain::ReconciliationChange {
        kind: ReconciliationKind::Conflict,
        before: Some("note.md".to_owned()),
        after: Some("note.md".to_owned()),
        detail: "ambiguous".to_owned(),
    });
    assert!(Workspace::apply_inventory_plan(root, plan).is_err());
    let mut wrong =
        Workspace::plan_reconciliation(root, "Project", "inventory-wrong", "2026-07-18T20:01:00Z")
            .expect("identity");
    wrong.project_id = "other".to_owned();
    assert!(Workspace::apply_inventory_plan(root, wrong).is_err());
}

fn assert_stale_inventory(root: &std::path::Path) {
    let stale =
        Workspace::plan_reconciliation(root, "Project", "inventory-stale", "2026-07-18T20:02:00Z")
            .expect("stale");
    let intervening = Workspace::plan_reconciliation(
        root,
        "Project",
        "inventory-intervening",
        "2026-07-18T20:01:00Z",
    )
    .expect("intervening");
    Workspace::apply_inventory_plan(root, intervening).expect("intervening apply");
    assert!(Workspace::apply_inventory_plan(root, stale).is_err());
    assert_eq!(
        Workspace::discover(root)
            .expect("discover")
            .latest_inventory()
            .expect("latest")
            .unwrap()
            .id,
        "inventory-intervening"
    );
}
