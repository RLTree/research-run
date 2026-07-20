use super::*;

#[test]
fn exact_pending_bootstrap_marker_is_recovered_after_pre_link_crash() {
    for cleanup_fault in [None, Some("remove abandoned pending record")] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = retrofit_plan(&root);
        let pending = write_pending_marker(&root, &plan, 0);
        if let Some(fault) = cleanup_fault {
            inject_storage_failure(fault);
            assert!(Workspace::apply_inventory_plan(&root, plan.clone()).is_err());
            assert!(pending.is_file());
        }
        Workspace::apply_inventory_plan(&root, plan).expect("recover exact pending marker");
        assert!(!pending.exists());
        assert!(
            root.join(".research-run/inventories/inventory-one.json")
                .is_file()
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn conflicting_pending_bootstrap_markers_fail_closed() {
    for multiple in [false, true] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = retrofit_plan(&root);
        let pending = write_pending_marker(&root, &plan, 0);
        if multiple {
            write_pending_marker(&root, &plan, 1);
        } else {
            fs::write(&pending, b"{}\n").expect("replace marker");
        }
        assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
        assert!(pending.exists());
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn pending_bootstrap_marker_cannot_coexist_with_other_state() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = retrofit_plan(&root);
    write_pending_marker(&root, &plan, 0);
    fs::write(root.join(".research-run/unexpected"), b"unexpected").expect("unexpected state");
    assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn linked_bootstrap_marker_cleans_only_exact_pending_state() {
    for conflicting in [false, true] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = retrofit_plan(&root);
        let pending = write_pending_marker(&root, &plan, 0);
        let marker = root.join(".research-run/inventory-bootstrap.json");
        if conflicting {
            fs::copy(&pending, &marker).expect("canonical marker fixture");
            fs::write(&pending, b"{}\n").expect("conflicting pending marker");
            assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
            assert!(pending.exists());
        } else {
            fs::hard_link(&pending, &marker).expect("linked marker fixture");
            inject_storage_failure("remove abandoned pending record");
            assert!(Workspace::apply_inventory_plan(&root, plan.clone()).is_err());
            Workspace::apply_inventory_plan(&root, plan).expect("linked cleanup retry");
            assert!(!pending.exists());
        }
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn canonical_marker_conflict_preserves_requested_plan_pending_evidence() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let requested = retrofit_plan(&root);
    let canonical = Workspace::plan_retrofit(
        &root,
        "Canonical",
        "inventory-canonical",
        "2026-07-18T20:00:01Z",
        explicit_unanchored_retrofit(),
    )
    .expect("canonical plan");
    let requested_pending = write_pending_marker(&root, &requested, 0);
    let canonical_pending = write_pending_marker(&root, &canonical, 1);
    fs::rename(
        canonical_pending,
        root.join(".research-run/inventory-bootstrap.json"),
    )
    .expect("canonical marker");

    assert!(Workspace::apply_inventory_plan(&root, requested).is_err());
    assert!(requested_pending.is_file());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn linked_bootstrap_pending_inspection_faults_fail_closed() {
    for fault in [
        "read linked bootstrap state",
        "read linked bootstrap entry",
        "inspect linked bootstrap pending entry",
    ] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = retrofit_plan(&root);
        let pending = write_pending_marker(&root, &plan, 0);
        fs::copy(
            &pending,
            root.join(".research-run/inventory-bootstrap.json"),
        )
        .expect("linked marker");
        inject_storage_failure(fault);
        assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[cfg(unix)]
#[test]
fn pending_bootstrap_marker_symlinks_fail_before_reading() {
    use std::os::unix::fs::symlink;

    for linked in [false, true] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = retrofit_plan(&root);
        let pending = write_pending_marker(&root, &plan, 0);
        if linked {
            fs::copy(
                &pending,
                root.join(".research-run/inventory-bootstrap.json"),
            )
            .expect("marker");
        }
        fs::remove_file(&pending).expect("remove pending");
        symlink(root.join("note.md"), &pending).expect("pending symlink");
        assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

fn retrofit_plan(root: &std::path::Path) -> crate::domain::InventoryPlan {
    Workspace::plan_retrofit(
        root,
        "Project",
        "inventory-one",
        "2026-07-18T20:00:00Z",
        explicit_unanchored_retrofit(),
    )
    .expect("plan")
}

fn write_pending_marker(
    root: &std::path::Path,
    plan: &crate::domain::InventoryPlan,
    sequence: u32,
) -> std::path::PathBuf {
    use serde::Serialize;
    use sha2::{Digest, Sha256};

    #[derive(Serialize)]
    struct Marker {
        schema_version: u32,
        kind: &'static str,
        plan_sha256: String,
    }

    let digest = format!(
        "{:x}",
        Sha256::digest(crate::workspace::publication::canonical_json_bytes(plan))
    );
    let state = root.join(".research-run");
    fs::create_dir_all(&state).expect("state");
    fs::write(state.join("write.lock"), b"").expect("lock");
    let pending = state.join(format!(".inventory-bootstrap.json.7.{sequence}.tmp"));
    fs::write(
        &pending,
        crate::workspace::publication::canonical_json_bytes(&Marker {
            schema_version: 1,
            kind: "inventory-bootstrap",
            plan_sha256: digest,
        }),
    )
    .expect("pending marker");
    pending
}
