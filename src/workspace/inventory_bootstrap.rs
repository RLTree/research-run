use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::InventoryPlan;
use crate::{Error, Result};

use super::path_safety::{interrupted_target_name, reject_symlink_chain};
use super::pending_cleanup::PendingCleanup;
use super::publication::canonical_json_bytes;
use super::storage::{ReadBudget, map_io, read_bounded, read_json_with_budget, sync_directory};
use super::{Workspace, injected_storage_failure};

pub(super) const INVENTORY_BOOTSTRAP_MARKER: &str = "inventory-bootstrap.json";

#[derive(Deserialize, Serialize)]
struct InventoryBootstrapMarker {
    schema_version: u32,
    kind: String,
    plan_sha256: String,
}

pub(super) enum InventoryBootstrapScaffold {
    Empty,
    ExactPending(PathBuf),
}

pub(super) fn inspect_inventory_bootstrap_scaffold(
    workspace: &Workspace,
    plan: &InventoryPlan,
) -> Result<Option<InventoryBootstrapScaffold>> {
    if injected_storage_failure("inventory bootstrap state conflict") {
        return Ok(None);
    }
    if injected_storage_failure("read inventory bootstrap state") {
        return Err(Error::io(
            "read inventory bootstrap state",
            &workspace.state,
            io::Error::other("injected storage failure"),
        ));
    }
    let directory = if injected_storage_failure("read inventory bootstrap directory") {
        Err(io::Error::other("injected storage failure"))
    } else {
        fs::read_dir(&workspace.state)
    };
    let entries = map_io(directory, "read bootstrap state", &workspace.state)?;
    let expected = canonical_json_bytes(&inventory_bootstrap_marker(plan));
    let mut pending = None;
    let mut unexpected = false;
    for entry in entries {
        let entry = if injected_storage_failure("read inventory bootstrap state entry") {
            Err(io::Error::other("injected storage failure"))
        } else {
            entry
        };
        let entry = map_io(
            entry,
            "read inventory bootstrap state entry",
            &workspace.state,
        )?;
        let path = entry.path();
        reject_symlink_chain(&path)?;
        let entry_type = if injected_storage_failure("inspect inventory bootstrap state entry") {
            Err(io::Error::other("injected storage failure"))
        } else {
            entry.file_type()
        };
        let entry_type = map_io(entry_type, "inspect bootstrap state entry", &path)?;
        if entry.file_name() == OsStr::new("write.lock") && entry_type.is_file() {
            continue;
        }
        if interrupted_target_name(&entry.file_name()) == Some(INVENTORY_BOOTSTRAP_MARKER)
            && entry_type.is_file()
        {
            if pending.is_some() {
                return Err(Error::AmbiguousEffect(
                    "multiple pending inventory bootstrap markers require manual recovery"
                        .to_owned(),
                ));
            }
            if !read_bounded(&path).is_ok_and(|bytes| bytes == expected) {
                return Err(Error::Conflict(
                    "pending inventory bootstrap marker belongs to a different plan".to_owned(),
                ));
            }
            pending = Some(path);
            continue;
        }
        unexpected = true;
    }
    if pending.is_some() && unexpected {
        return Err(Error::AmbiguousEffect(
            "pending inventory bootstrap marker coexists with other workspace state".to_owned(),
        ));
    }
    if unexpected {
        return Ok(None);
    }
    Ok(Some(pending.map_or(
        InventoryBootstrapScaffold::Empty,
        InventoryBootstrapScaffold::ExactPending,
    )))
}

pub(super) fn clear_exact_pending_bootstrap(scaffold: InventoryBootstrapScaffold) -> Result<()> {
    if let InventoryBootstrapScaffold::ExactPending(path) = scaffold {
        PendingCleanup::new(path).remove()?;
    }
    Ok(())
}

pub(super) fn clear_linked_bootstrap_pending(
    workspace: &Workspace,
    plan: &InventoryPlan,
) -> Result<()> {
    let expected = canonical_json_bytes(&inventory_bootstrap_marker(plan));
    let directory = if injected_storage_failure("read linked bootstrap state") {
        Err(io::Error::other("injected storage failure"))
    } else {
        fs::read_dir(&workspace.state)
    };
    for entry in map_io(directory, "read linked bootstrap state", &workspace.state)? {
        let entry = if injected_storage_failure("read linked bootstrap entry") {
            Err(io::Error::other("injected storage failure"))
        } else {
            entry
        };
        let entry = map_io(entry, "read linked bootstrap entry", &workspace.state)?;
        if interrupted_target_name(&entry.file_name()) != Some(INVENTORY_BOOTSTRAP_MARKER) {
            continue;
        }
        let path = entry.path();
        reject_symlink_chain(&path)?;
        let entry_type = if injected_storage_failure("inspect linked bootstrap pending entry") {
            Err(io::Error::other("injected storage failure"))
        } else {
            entry.file_type()
        };
        if !map_io(entry_type, "inspect linked bootstrap pending entry", &path)?.is_file()
            || !read_bounded(&path).is_ok_and(|bytes| bytes == expected)
        {
            return Err(Error::Conflict(
                "linked inventory bootstrap has a conflicting pending marker".to_owned(),
            ));
        }
        PendingCleanup::new(path).remove()?;
    }
    Ok(())
}

pub(super) fn ensure_inventory_bootstrap_marker(
    workspace: &Workspace,
    plan: &InventoryPlan,
) -> Result<()> {
    let path = workspace.state.join(INVENTORY_BOOTSTRAP_MARKER);
    let expected = inventory_bootstrap_marker(plan);
    if path.exists() {
        let mut budget = ReadBudget::default();
        let actual: InventoryBootstrapMarker = read_json_with_budget(&path, &mut budget)?;
        if actual.schema_version != expected.schema_version
            || actual.kind != expected.kind
            || actual.plan_sha256 != expected.plan_sha256
        {
            return Err(Error::Conflict(
                "inventory bootstrap marker belongs to a different plan".to_owned(),
            ));
        }
        return Ok(());
    }
    workspace
        .publish_value(&path, &expected)
        .map(|_| ())
        .map_err(|error| inventory_bootstrap_error(plan, error))
}

pub(super) fn initialize_inventory_bootstrap(
    root: &Path,
    plan: &InventoryPlan,
) -> Result<Workspace> {
    match (&plan.review_authority, plan.without_review_authority) {
        (Some(authority), false) => Workspace::initialize_for_inventory(
            root,
            &plan.project_name,
            Some(authority),
            &plan.workspace_id,
        ),
        (None, true) => {
            Workspace::initialize_for_inventory(root, &plan.project_name, None, &plan.workspace_id)
        }
        _ => Err(Error::invalid(
            "inventory plan review authority",
            "new-workspace retrofit requires an authority or explicit unanchored opt-out",
        )),
    }
}

pub(super) fn finish_inventory_bootstrap(workspace: &Workspace) -> Result<()> {
    let marker = workspace.state.join(INVENTORY_BOOTSTRAP_MARKER);
    if !marker.exists() {
        return Ok(());
    }
    let removal = if injected_storage_failure("remove inventory bootstrap marker") {
        Err(io::Error::other("injected storage failure"))
    } else {
        fs::remove_file(&marker)
    };
    if let Err(error) = map_io(removal, "remove inventory bootstrap marker", &marker) {
        return Err(Error::AmbiguousEffect(format!(
            "inventory committed but bootstrap marker cleanup failed: {error}"
        )));
    }
    let sync = if injected_storage_failure("sync inventory bootstrap directory") {
        Err(Error::io(
            "sync inventory bootstrap directory",
            &workspace.state,
            io::Error::other("injected storage failure"),
        ))
    } else {
        sync_directory(&workspace.state)
    };
    sync.map_err(|error| {
        Error::AmbiguousEffect(format!(
            "inventory committed but bootstrap marker cleanup is ambiguous: {error}"
        ))
    })
}

pub(super) fn inventory_bootstrap_error(plan: &InventoryPlan, error: Error) -> Error {
    Error::AmbiguousEffect(format!(
        "inventory bootstrap for {} may be incomplete: {error}; reapply the exact accepted plan",
        plan.id
    ))
}

fn inventory_bootstrap_marker(plan: &InventoryPlan) -> InventoryBootstrapMarker {
    InventoryBootstrapMarker {
        schema_version: 1,
        kind: "inventory-bootstrap".to_owned(),
        plan_sha256: format!("{:x}", Sha256::digest(canonical_json_bytes(plan))),
    }
}
