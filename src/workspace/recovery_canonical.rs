use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::domain::{CanonicalRecord, ProjectManifest};
use crate::{Error, Result};

use super::publication::canonical_json_bytes;
use super::recovery_plan::PendingRecord;
use super::storage::{ReadBudget, parse_json, read_bounded, read_json_with_budget};
use super::{MAX_RECORD_BYTES, Workspace};

pub(super) fn canonical_manifest(
    workspace: &Workspace,
    pending: &mut [PendingRecord],
    budget: &mut ReadBudget,
) -> Result<ProjectManifest> {
    let target = workspace.state.join("manifest.json");
    let mut candidate = None;
    for record in pending.iter_mut() {
        budget.consume(&record.path, record.bytes.len() as u64)?;
        let parsed: ProjectManifest = parse_json(&record.bytes, &record.target)?;
        parsed.validate()?;
        record.bytes = canonical_json_bytes(&parsed);
        candidate.get_or_insert(parsed);
    }
    let unique = unique_pending(pending)?;
    if unique.iter().any(|record| record.target != target) {
        return Err(Error::invalid(
            "recovery target",
            "project manifest must publish only as manifest.json",
        ));
    }
    if target.exists() {
        for record in unique {
            ensure_identical_target(record)?;
        }
        let manifest = read_json_with_budget::<ProjectManifest>(&target, budget)?;
        manifest.validate()?;
        return Ok(manifest);
    }
    candidate.ok_or_else(|| Error::NotFound("recovery requires a project manifest".to_owned()))
}

pub(super) fn canonical_records<T>(
    workspace: &Workspace,
    directory: &str,
    pending: &mut [PendingRecord],
    budget: &mut ReadBudget,
) -> Result<Vec<T>>
where
    T: CanonicalRecord + DeserializeOwned + Serialize,
{
    let mut candidates = BTreeMap::<std::path::PathBuf, T>::new();
    for record in pending.iter_mut() {
        budget.consume(&record.path, record.bytes.len() as u64)?;
        let candidate: T = parse_json(&record.bytes, &record.target)?;
        candidate.validate()?;
        if record.target.file_stem().and_then(OsStr::to_str) != Some(candidate.id()) {
            return Err(Error::invalid(
                "recovery target",
                "filename does not match record id",
            ));
        }
        record.bytes = bounded_canonical_json(&candidate)?;
        candidates.entry(record.target.clone()).or_insert(candidate);
    }
    let mut records = workspace.load_records(directory, true, budget)?;
    for record in unique_pending(pending)? {
        if record.target.exists() {
            ensure_identical_target(record)?;
            continue;
        }
        records.push(
            candidates
                .remove(&record.target)
                .expect("every canonical pending target retains its typed record"),
        );
    }
    Ok(records)
}

fn bounded_canonical_json(value: &impl Serialize) -> Result<Vec<u8>> {
    let bytes = canonical_json_bytes(value);
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(Error::Budget(format!(
            "canonical recovery record exceeds the {MAX_RECORD_BYTES} byte budget"
        )));
    }
    Ok(bytes)
}

fn unique_pending(pending: &[PendingRecord]) -> Result<Vec<&PendingRecord>> {
    let mut by_target = BTreeMap::<&Path, &PendingRecord>::new();
    for record in pending {
        if let Some(existing) = by_target.get(record.target.as_path())
            && existing.bytes != record.bytes
        {
            return Err(Error::AmbiguousEffect(format!(
                "conflicting pending publications target {}; inspect them before recovery",
                record.target.display()
            )));
        }
        by_target.insert(&record.target, record);
    }
    Ok(by_target.into_values().collect())
}

fn ensure_identical_target(record: &PendingRecord) -> Result<()> {
    if read_bounded(&record.target)? == record.bytes {
        Ok(())
    } else {
        Err(Error::AmbiguousEffect(format!(
            "recovery conflict for {}; inspect both files",
            record.target.display()
        )))
    }
}
