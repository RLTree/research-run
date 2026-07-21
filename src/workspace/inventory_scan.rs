use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::{
    BoundaryEntry, InventoryBoundary, InventoryEntry, InventoryLimits, InventoryPolicy,
    MaterialEntry, ProjectManifest,
};
use crate::{Error, Result};

use super::path_safety::{absolute_path, reject_symlink_chain};
use super::storage::map_io;
use super::{MAX_INVENTORY_RECORD_BYTES, injected_storage_failure};

mod boundary;
mod material;

pub(super) use material::{classify_material, hash_file};

#[cfg(test)]
pub(super) fn scan_materials(root: &Path) -> Result<(PathBuf, Vec<InventoryEntry>)> {
    let parent = ProjectManifest {
        schema_version: 1,
        kind: "project-manifest".to_owned(),
        project_id: "scan-parent".to_owned(),
        workspace_id: "a".repeat(64),
        name: "Scan parent".to_owned(),
        declared_roots: vec![".".to_owned()],
        review_authority_id: None,
        review_authority_fingerprint: None,
    };
    scan_materials_with_policy(root, None, &parent)
}

pub(super) fn scan_materials_with_policy(
    root: &Path,
    policy: Option<&InventoryPolicy>,
    parent: &ProjectManifest,
) -> Result<(PathBuf, Vec<InventoryEntry>)> {
    let root = absolute_path(root)?;
    reject_symlink_chain(&root)?;
    if !root.is_dir() {
        return Err(Error::invalid("retrofit target", "must be a directory"));
    }
    if let Some(policy) = policy {
        policy.validate()?;
    }
    let limits = policy.map_or_else(InventoryLimits::legacy, |value| value.limits);
    let boundaries = policy
        .map(|value| {
            value
                .boundaries
                .iter()
                .map(|boundary| (boundary.path().to_owned(), boundary.clone()))
                .collect()
        })
        .unwrap_or_default();
    let mut scanner = Scanner {
        root: &root,
        parent,
        limits,
        boundaries,
        seen_boundaries: BTreeSet::new(),
        entries: Vec::new(),
        indexed_bytes: 0,
        metadata_bytes: 0,
    };
    scanner.collect(&root)?;
    if scanner.seen_boundaries.len() != scanner.boundaries.len() {
        let missing = scanner
            .boundaries
            .keys()
            .find(|path| !scanner.seen_boundaries.contains(*path))
            .expect("unequal boundary sets retain a missing path");
        return Err(Error::NotFound(format!(
            "declared inventory boundary is missing: {missing}"
        )));
    }
    scanner
        .entries
        .sort_by(|left, right| left.path().cmp(right.path()));
    validate_child_identities(&scanner.entries, parent)?;
    let entries = std::mem::take(&mut scanner.entries);
    drop(scanner);
    Ok((root, entries))
}

struct Scanner<'a> {
    root: &'a Path,
    parent: &'a ProjectManifest,
    limits: InventoryLimits,
    boundaries: BTreeMap<String, InventoryBoundary>,
    seen_boundaries: BTreeSet<String>,
    entries: Vec<InventoryEntry>,
    indexed_bytes: u64,
    metadata_bytes: u64,
}

impl Scanner<'_> {
    fn collect(&mut self, directory: &Path) -> Result<()> {
        reject_symlink_chain(directory)?;
        let mut entries = map_io(
            fs::read_dir(directory),
            "read retrofit directory",
            directory,
        )?
        .map(|entry| map_io(entry, "read retrofit entry", directory))
        .collect::<Result<Vec<_>>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            self.collect_entry(&entry)?;
        }
        Ok(())
    }

    fn collect_entry(&mut self, entry: &fs::DirEntry) -> Result<()> {
        let path = entry.path();
        let relative_path = path.strip_prefix(self.root).expect("entry is rooted");
        let relative = relative_path
            .to_str()
            .filter(|_| !injected_storage_failure("material path UTF-8"))
            .ok_or_else(|| Error::invalid("material path", "must be UTF-8"))?
            .replace(std::path::MAIN_SEPARATOR, "/");
        let metadata = map_io(fs::symlink_metadata(&path), "inspect retrofit entry", &path)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::invalid(
                "retrofit target",
                format!("symlink is forbidden: {}", path.display()),
            ));
        }
        if relative_path.components().count() == 1
            && matches!(entry.file_name().to_str(), Some(".git" | ".research-run"))
        {
            return Ok(());
        }
        if let Some(declaration) = self.boundaries.get(&relative).cloned() {
            let observed = boundary::observe(
                self.root,
                &path,
                &relative,
                &metadata,
                &declaration,
                self.parent,
            )?;
            self.seen_boundaries.insert(relative);
            return self.push(InventoryEntry::Boundary(observed));
        }
        if entry.file_name() == OsStr::new(".research-run") {
            return Err(Error::invalid(
                "nested workspace boundary",
                format!("undeclared Research Run workspace at {}", path.display()),
            ));
        }
        if metadata.is_dir() {
            self.collect(&path)
        } else if metadata.is_file() {
            let (bytes, sha256) = hash_file(&path, self.limits.max_file_bytes)?;
            self.indexed_bytes = add_inventory_bytes_with_limit(
                self.indexed_bytes,
                bytes,
                self.limits.max_total_bytes,
            )?;
            self.push(
                MaterialEntry {
                    class: classify_material(&relative),
                    path: relative,
                    bytes,
                    sha256,
                }
                .into(),
            )
        } else {
            Err(Error::invalid(
                "retrofit target",
                format!("unsupported filesystem entry: {}", path.display()),
            ))
        }
    }

    fn push(&mut self, entry: InventoryEntry) -> Result<()> {
        enforce_entry_count(self.entries.len() + 1, self.limits.max_entries)?;
        let serialized = serde_json::to_vec(&entry)
            .expect("inventory entries contain only JSON-representable data");
        self.metadata_bytes = self
            .metadata_bytes
            .checked_add(serialized.len() as u64)
            .ok_or_else(|| Error::Budget("inventory metadata budget overflowed".to_owned()))?;
        if self.metadata_bytes > MAX_INVENTORY_RECORD_BYTES {
            return Err(Error::Budget(format!(
                "inventory metadata exceeds the {MAX_INVENTORY_RECORD_BYTES} byte record budget"
            )));
        }
        self.entries.push(entry);
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn enforce_file_count(count: usize) -> Result<()> {
    enforce_entry_count(count, InventoryLimits::legacy().max_entries)
}

pub(super) fn enforce_entry_count(count: usize, maximum: u64) -> Result<()> {
    if u64::try_from(count).unwrap_or(u64::MAX) > maximum {
        return Err(Error::Budget(format!(
            "material inventory exceeds the {maximum} entry budget"
        )));
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn add_inventory_bytes(total: u64, bytes: u64) -> Result<u64> {
    add_inventory_bytes_with_limit(total, bytes, InventoryLimits::legacy().max_total_bytes)
}

pub(super) fn add_inventory_bytes_with_limit(total: u64, bytes: u64, maximum: u64) -> Result<u64> {
    let total = total
        .checked_add(bytes)
        .ok_or_else(|| Error::Budget("material inventory byte total overflowed".to_owned()))?;
    if total > maximum || injected_storage_failure("inventory byte budget") {
        return Err(Error::Budget(format!(
            "material inventory exceeds the {maximum} byte scan budget"
        )));
    }
    Ok(total)
}

fn validate_child_identities(entries: &[InventoryEntry], parent: &ProjectManifest) -> Result<()> {
    let mut identities = BTreeSet::new();
    for boundary in entries.iter().filter_map(|entry| match entry {
        InventoryEntry::Boundary(BoundaryEntry::ChildWorkspace {
            workspace_observation,
            ..
        }) => Some(workspace_observation),
        _ => None,
    }) {
        if boundary.project_id == parent.project_id || boundary.workspace_id == parent.workspace_id
        {
            return Err(Error::invalid(
                "child workspace identity",
                "child and parent identities must be distinct",
            ));
        }
        if !identities.insert(&boundary.workspace_id) {
            return Err(Error::invalid(
                "child workspace identity",
                "multiple boundaries name the same workspace",
            ));
        }
    }
    Ok(())
}
