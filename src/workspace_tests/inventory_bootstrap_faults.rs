use super::*;
use crate::workspace::inventory_authority::inventory_apply_workspace;
use crate::workspace::inventory_bootstrap::finish_inventory_bootstrap;

#[test]
fn bootstrap_scaffold_faults_preserve_a_retryable_boundary() {
    for fault in [
        "read inventory bootstrap state",
        "read inventory bootstrap directory",
        "read inventory bootstrap state entry",
        "inspect inventory bootstrap state entry",
        "inventory bootstrap state conflict",
    ] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = retrofit_plan(&root);
        inject_storage_failure("lock identity");
        assert!(Workspace::apply_inventory_plan(&root, plan.clone()).is_err());

        inject_storage_failure(fault);
        Workspace::apply_inventory_plan(&root, plan.clone()).expect_err("fault");
        Workspace::apply_inventory_plan(&root, plan).expect("exact retry");
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn bootstrap_creation_and_cleanup_faults_are_explicit() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = retrofit_plan(&root);
    inject_storage_failure("create inventory bootstrap state");
    let error = inventory_apply_workspace(&root, &plan).expect_err("create state fault");
    assert!(
        error
            .to_string()
            .contains("create inventory bootstrap state"),
        "unexpected error: {error}"
    );
    assert!(!root.join(".research-run").exists());
    fs::remove_dir_all(root).expect("remove fixture");

    for fault in [
        "remove inventory bootstrap marker",
        "sync inventory bootstrap directory",
    ] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = retrofit_plan(&root);
        inject_storage_failure(fault);
        assert!(Workspace::apply_inventory_plan(&root, plan.clone()).is_err());
        Workspace::apply_inventory_plan(&root, plan).expect("cleanup retry");
        let workspace = Workspace::at_exact_root(&root)
            .expect("inspect")
            .expect("workspace");
        finish_inventory_bootstrap(&workspace).expect("absent marker");
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn bootstrap_marker_identity_requires_every_canonical_field() {
    use serde::Serialize;
    use sha2::{Digest, Sha256};

    #[derive(Serialize)]
    struct Marker<'a> {
        schema_version: u32,
        kind: &'a str,
        plan_sha256: String,
    }

    for (schema_version, kind) in [(2, "inventory-bootstrap"), (1, "other-bootstrap")] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = retrofit_plan(&root);
        let state = root.join(".research-run");
        fs::create_dir(&state).expect("state");
        let plan_sha256 = format!(
            "{:x}",
            Sha256::digest(crate::workspace::publication::canonical_json_bytes(&plan))
        );
        fs::write(
            state.join("inventory-bootstrap.json"),
            crate::workspace::publication::canonical_json_bytes(&Marker {
                schema_version,
                kind,
                plan_sha256,
            }),
        )
        .expect("marker");

        let error = inventory_apply_workspace(&root, &plan).expect_err("marker conflict");
        assert!(
            error
                .to_string()
                .contains("inventory bootstrap marker belongs to a different plan"),
            "unexpected error: {error}"
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[test]
fn new_bootstrap_rejects_an_injected_unexpected_effect() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = retrofit_plan(&root);
    inject_storage_failure("inventory bootstrap state conflict");
    assert!(Workspace::apply_inventory_plan(&root, plan.clone()).is_err());
    Workspace::apply_inventory_plan(&root, plan).expect("retry");
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn bootstrap_post_initialization_lock_failure_is_retryable() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = retrofit_plan(&root);
    inject_storage_failure("lock identity#3");
    let error =
        Workspace::apply_inventory_plan(&root, plan.clone()).expect_err("publication lock fault");
    assert!(
        error
            .to_string()
            .contains("inventory bootstrap for inventory-one may be incomplete"),
        "unexpected error: {error}"
    );
    assert!(
        error
            .to_string()
            .contains("reapply the exact accepted plan"),
        "missing operator recovery action: {error}"
    );
    Workspace::apply_inventory_plan(&root, plan).expect("retry");
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn normal_workspace_handoff_never_republishes_a_losing_plan_marker() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let losing = retrofit_plan(&root);
    let winner = Workspace::plan_retrofit(
        &root,
        "Winner",
        "inventory-winner",
        "2026-07-18T20:00:01Z",
        explicit_unanchored_retrofit(),
    )
    .expect("winner plan");
    Workspace::apply_inventory_plan(&root, winner).expect("winner");
    assert!(Workspace::apply_inventory_plan(&root, losing).is_err());
    assert!(!root.join(".research-run/inventory-bootstrap.json").exists());
    assert!(
        root.join(".research-run/inventories/inventory-winner.json")
            .is_file()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn partial_bootstrap_scaffolds_reject_unexpected_shapes() {
    for lock_is_directory in [false, true] {
        let root = temporary();
        fs::write(root.join("note.md"), b"note").expect("note");
        let plan = retrofit_plan(&root);
        let state = root.join(".research-run");
        fs::create_dir(&state).expect("state");
        if lock_is_directory {
            fs::create_dir(state.join("write.lock")).expect("directory lock");
        } else {
            fs::write(state.join("unexpected"), b"unexpected").expect("unexpected entry");
        }
        assert!(inventory_apply_workspace(&root, &plan).is_err());
        fs::remove_dir_all(root).expect("remove fixture");
    }
}

#[cfg(unix)]
#[test]
fn bootstrap_scaffold_rejects_symlinked_state_entries() {
    use std::os::unix::fs::symlink;

    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = retrofit_plan(&root);
    let state = root.join(".research-run");
    fs::create_dir(&state).expect("state");
    symlink(root.join("note.md"), state.join("write.lock")).expect("symlink lock");
    assert!(inventory_apply_workspace(&root, &plan).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[cfg(unix)]
#[test]
fn bootstrap_rejects_a_symlinked_state_boundary() {
    use std::os::unix::fs::symlink;

    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = retrofit_plan(&root);
    symlink(root.join("note.md"), root.join(".research-run")).expect("symlink state");
    assert!(inventory_apply_workspace(&root, &plan).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn bootstrap_missing_workspace_and_corrupt_marker_fail_closed() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = retrofit_plan(&root);
    let state = root.join(".research-run");
    fs::create_dir(&state).expect("state");
    fs::write(state.join("unexpected"), b"unexpected").expect("unexpected");
    inject_storage_failure("inventory workspace disappeared after inspection");
    assert!(inventory_apply_workspace(&root, &plan).is_err());
    fs::remove_dir_all(root).expect("remove fixture");

    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    let plan = retrofit_plan(&root);
    inject_storage_failure("materials changed under lock");
    assert!(Workspace::apply_inventory_plan(&root, plan.clone()).is_err());
    fs::write(root.join(".research-run/inventory-bootstrap.json"), b"{").expect("corrupt marker");
    assert!(Workspace::apply_inventory_plan(&root, plan).is_err());
    fs::remove_dir_all(root).expect("remove fixture");
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
