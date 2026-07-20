use super::*;
use crate::workspace::inventory_scan::scan_materials;

#[test]
fn material_scan_enforces_file_count_budget() {
    let root = temporary();
    for index in 0..=crate::domain::MAX_INVENTORY_ENTRIES {
        fs::write(root.join(format!("file-{index:04}")), b"").expect("file fixture");
    }
    assert!(scan_materials(&root).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_planning_propagates_validation_and_storage_failures() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    inject_storage_failure("read retrofit directory");
    assert!(
        Workspace::plan_retrofit(
            &root,
            "Project",
            "inventory",
            "2026-07-18T20:00:00Z",
            explicit_unanchored_retrofit(),
        )
        .is_err()
    );
    assert!(
        Workspace::plan_retrofit(
            &root,
            "",
            "inventory",
            "2026-07-18T20:00:00Z",
            explicit_unanchored_retrofit(),
        )
        .is_err()
    );
    assert!(
        Workspace::plan_retrofit(
            &root,
            "Project",
            "INVALID",
            "2026-07-18T20:00:00Z",
            explicit_unanchored_retrofit(),
        )
        .is_err()
    );
    let workspace = Workspace::initialize(&root, "Project").expect("initialize");
    assert!(
        Workspace::plan_reconciliation(&root, "Project", "inventory", "2026-07-18T20:00:00Z",)
            .is_err(),
        "reconciliation without a prior inventory was accepted"
    );
    assert!(
        Workspace::plan_retrofit(
            &root,
            "Project",
            "inventory",
            "2026-07-18T20:00:00Z",
            explicit_unanchored_retrofit(),
        )
        .is_err(),
        "existing workspace accepted retrofit bootstrap"
    );
    fs::write(workspace.state.join("manifest.json"), b"{").expect("corrupt manifest");
    assert!(
        Workspace::plan_retrofit(&root, "Project", "inventory", "2026-07-18T20:00:00Z", None,)
            .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_application_propagates_owned_storage_failures() {
    for point in [
        "read retrofit directory",
        "create project directory",
        "open workspace write lock",
        "publish canonical record",
    ] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = Workspace::plan_retrofit(
            &root,
            "Project",
            "inventory-one",
            "2026-07-18T20:00:00Z",
            explicit_unanchored_retrofit(),
        )
        .expect("plan");
        inject_storage_failure(point);
        assert!(
            Workspace::apply_inventory_plan(&root, plan).is_err(),
            "{point}"
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let mut invalid = Workspace::plan_retrofit(
        &root,
        "Project",
        "inventory-one",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    invalid.id = "INVALID".to_owned();
    assert!(Workspace::apply_inventory_plan(&root, invalid).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn material_scan_rejects_a_symlinked_root() {
    use std::os::unix::fs::symlink;
    let root = temporary();
    let target = temporary();
    let linked = root.join("linked");
    symlink(&target, &linked).expect("root symlink");
    assert!(scan_materials(&linked).is_err());
    fs::remove_dir_all(root).expect("remove root");
    fs::remove_dir_all(target).expect("remove target");
}

#[test]
fn material_scan_propagates_relative_root_discovery_failure() {
    inject_storage_failure("read current directory");
    assert!(scan_materials(std::path::Path::new("relative")).is_err());
}

#[test]
fn material_scan_propagates_each_symlink_recheck_boundary() {
    let root = temporary();
    fs::create_dir(root.join("nested")).expect("nested");
    fs::write(root.join("nested/note.md"), b"note").expect("note");
    for occurrence in 1..=6 {
        let point = match occurrence {
            1 => "inspect workspace path",
            2 => "inspect workspace path#2",
            3 => "inspect workspace path#3",
            4 => "inspect workspace path#4",
            5 => "inspect workspace path#5",
            _ => "inspect workspace path#6",
        };
        inject_storage_failure(point);
        assert!(scan_materials(&root).is_err(), "{point}");
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_planning_propagates_manifest_and_prior_inventory_failures() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let workspace = Workspace::initialize(&root, "Project").expect("initialize");
    inject_storage_failure("inspect record#2");
    assert!(
        Workspace::plan_retrofit(&root, "Project", "inventory", "2026-07-18T20:00:00Z", None,)
            .is_err()
    );
    fs::write(workspace.state.join("inventories/bad.json"), b"{}").expect("bad inventory");
    assert!(
        Workspace::plan_retrofit(&root, "Project", "inventory", "2026-07-18T20:00:00Z", None,)
            .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn inventory_application_propagates_each_post_scan_authority_failure() {
    for point in [
        "inspect record#2",
        "create project directory",
        "open workspace write lock",
        "publish canonical record",
    ] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let workspace = Workspace::initialize(&root, "Project").expect("initialize");
        let plan =
            Workspace::plan_retrofit(&root, "Project", "inventory", "2026-07-18T20:00:00Z", None)
                .expect("plan");
        if point == "create project directory" {
            fs::remove_dir(workspace.state.join("inventories")).expect("remove inventories");
        }
        inject_storage_failure(point);
        assert!(
            Workspace::apply_inventory_plan(&root, plan).is_err(),
            "{point}"
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn inventory_application_propagates_exact_root_record_and_latest_failures() {
    for scenario in ["exact root", "identical record", "latest inventory"] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let workspace = Workspace::initialize(&root, "Project").expect("initialize");
        let plan =
            Workspace::plan_retrofit(&root, "Project", "inventory", "2026-07-18T20:00:00Z", None)
                .expect("plan");
        match scenario {
            "exact root" => {
                fs::write(workspace.state.join("manifest.json"), b"{").expect("corrupt")
            }
            "identical record" => {
                Workspace::apply_inventory_plan(&root, plan.clone()).expect("seed");
                inject_storage_failure("inspect record#3");
            }
            _ => fs::write(workspace.state.join("inventories/bad.json"), b"{}")
                .expect("bad inventory"),
        }
        assert!(
            Workspace::apply_inventory_plan(&root, plan).is_err(),
            "{scenario}"
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn inventory_read_and_latest_snapshot_failures_are_explicit() {
    let root = temporary();
    let workspace = Workspace::initialize(&root, "Project").expect("initialize");
    fs::write(workspace.state.join("inventories/bad.json"), b"{}").expect("bad inventory");
    assert!(workspace.latest_inventory().is_err());
    let invalid = root.join("invalid-plan.json");
    fs::write(&invalid, b"{}").expect("invalid plan");
    assert!(Workspace::read_inventory_plan(&invalid).is_err());
    inject_storage_failure("inspect workspace path");
    assert!(Workspace::read_inventory_plan(&invalid).is_err());
    inject_storage_failure("inspect workspace path");
    assert!(Workspace::at_exact_root(&root).is_err());
    fs::remove_dir_all(root).expect("remove fixture");

    let plan_root = temporary();
    fs::write(plan_root.join("note.md"), b"note").expect("note");
    let mut plan = Workspace::plan_retrofit(
        &plan_root,
        "Project",
        "inventory",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan");
    plan.id = "INVALID".to_owned();
    let invalid = plan_root.join("invalid-semantic-plan.json");
    fs::write(&invalid, serde_json::to_vec(&plan).unwrap()).expect("invalid plan");
    assert!(Workspace::read_inventory_plan(&invalid).is_err());
    fs::remove_dir_all(plan_root).expect("remove fixture");
}
