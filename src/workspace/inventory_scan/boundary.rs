use std::fs::Metadata;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::domain::{
    BoundaryEntry, ChildInventoryIdentity, ChildWorkspaceObservation, InspectionStatus,
    InventoryBoundary, InventorySnapshot, ProjectManifest, RootNodeKind, RootObservation,
};
use crate::{Error, Result};

use super::super::publication::canonical_json_bytes;
use super::super::storage::ReadBudget;
use super::super::{STATE_DIRECTORY, Workspace};

pub(super) fn observe(
    parent_root: &Path,
    path: &Path,
    relative: &str,
    metadata: &Metadata,
    declaration: &InventoryBoundary,
    parent: &ProjectManifest,
) -> Result<BoundaryEntry> {
    match declaration {
        InventoryBoundary::ReferenceOnly {
            path: declared_path,
            class,
            rationale,
            reference,
        } => {
            debug_assert_eq!(declared_path, relative);
            let root_observation = if metadata.is_file() {
                RootObservation {
                    node_kind: RootNodeKind::File,
                    bytes: Some(metadata.len()),
                }
            } else if metadata.is_dir() {
                RootObservation {
                    node_kind: RootNodeKind::Directory,
                    bytes: None,
                }
            } else {
                return Err(Error::invalid(
                    "reference-only boundary",
                    format!("must be a regular file or directory: {}", path.display()),
                ));
            };
            Ok(BoundaryEntry::ReferenceOnly {
                path: relative.to_owned(),
                class: *class,
                declared_rationale: rationale.clone(),
                declared_reference: reference.clone(),
                root_observation,
                content_status: InspectionStatus::NotInspected,
            })
        }
        InventoryBoundary::ChildWorkspace {
            path: declared_path,
            rationale,
        } => {
            debug_assert_eq!(declared_path, relative);
            if !metadata.is_dir() {
                return Err(Error::invalid(
                    "child workspace boundary",
                    format!("must be a directory: {}", path.display()),
                ));
            }
            let workspace_observation = observe_child(parent_root, path, relative, parent)?;
            Ok(BoundaryEntry::ChildWorkspace {
                path: relative.to_owned(),
                declared_rationale: rationale.clone(),
                workspace_observation,
                child_material_status: InspectionStatus::NotInspected,
            })
        }
    }
}

fn observe_child(
    parent_root: &Path,
    child_root: &Path,
    relative: &str,
    parent: &ProjectManifest,
) -> Result<ChildWorkspaceObservation> {
    let workspace = Workspace::at_exact_root(child_root)?.ok_or_else(|| {
        Error::NotFound(format!(
            "declared child workspace has no canonical manifest: {}",
            child_root.display()
        ))
    })?;
    let manifest = workspace.read_manifest()?;
    if manifest.workspace_id.is_empty()
        || manifest.project_id == parent.project_id
        || manifest.workspace_id == parent.workspace_id
    {
        return Err(Error::invalid(
            "child workspace identity",
            "child requires a distinct immutable project and workspace identity",
        ));
    }
    let mut budget = ReadBudget::default();
    let inventories: Vec<InventorySnapshot> =
        workspace.load_optional_records("inventories", false, &mut budget)?;
    if inventories
        .iter()
        .any(|inventory| inventory.project_id != manifest.project_id)
    {
        return Err(Error::invalid(
            "child workspace identity",
            "inventory project identity does not match the child manifest",
        ));
    }
    let latest_inventory = inventories
        .iter()
        .max_by(|left, right| (&left.observed_at, &left.id).cmp(&(&right.observed_at, &right.id)))
        .map(|inventory| ChildInventoryIdentity {
            id: inventory.id.clone(),
            observed_at: inventory.observed_at.clone(),
            sha256: digest(&canonical_json_bytes(inventory)),
        });
    let manifest_path = child_root
        .join(STATE_DIRECTORY)
        .join("manifest.json")
        .strip_prefix(parent_root)
        .expect("child manifest remains beneath the parent")
        .to_str()
        .ok_or_else(|| Error::invalid("child manifest path", "must be UTF-8"))?
        .replace(std::path::MAIN_SEPARATOR, "/");
    debug_assert!(manifest_path.starts_with(relative));
    let manifest_sha256 = digest(&canonical_json_bytes(&manifest));
    Ok(ChildWorkspaceObservation {
        manifest_path,
        project_name: manifest.name,
        project_id: manifest.project_id,
        workspace_id: manifest.workspace_id,
        manifest_sha256,
        latest_inventory,
    })
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
