use std::collections::BTreeMap;
use std::ffi::OsStr;

use crate::{Error, Result};

use super::Snapshot;
use super::recovery_plan::PendingRecord;

pub(super) fn validate_pending_review_graphs(
    snapshot: &Snapshot,
    pending: &[PendingRecord],
) -> Result<()> {
    let mut current = BTreeMap::<&str, Vec<&str>>::new();
    for evidence in &snapshot.evidence {
        current
            .entry(evidence.claim_id.as_str())
            .or_default()
            .push(evidence.id.as_str());
    }
    for ids in current.values_mut() {
        ids.sort_unstable();
    }
    for review in &snapshot.reviews {
        let pending_review = pending.iter().any(|record| {
            record.target.file_stem().and_then(OsStr::to_str) == Some(review.id.as_str())
        });
        if !pending_review {
            continue;
        }
        let evidence = current
            .get(review.claim_id.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();
        if !review
            .evidence_ids
            .iter()
            .map(String::as_str)
            .eq(evidence.iter().copied())
        {
            return Err(Error::invalid(
                "review evidence_ids",
                "must exactly match the current claim evidence graph",
            ));
        }
    }
    Ok(())
}
