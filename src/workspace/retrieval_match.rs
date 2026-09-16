use std::collections::BTreeMap;

use super::super::{ProjectionItem, Snapshot};
use super::match_reasons;

/// Match projections first, preserving legacy summaries and reasons. Body fallback
/// borrows only knowledge from the same validated snapshot; kinds cannot collide.
pub(in crate::workspace) fn matching_items(
    items: Vec<ProjectionItem>,
    snapshot: &Snapshot,
    query: &str,
) -> Vec<ProjectionItem> {
    let bodies = snapshot
        .knowledge
        .iter()
        .map(|record| (record.id.as_str(), record.body.as_str()))
        .collect::<BTreeMap<_, _>>();
    items
        .into_iter()
        .filter_map(|mut item| {
            item.matched_by = match_reasons(&item, query);
            if item.matched_by.is_empty()
                && item.kind == "knowledge"
                && let Some(body) = bodies.get(item.id.as_str())
                && let Some(excerpt) = body_excerpt(body, query)
            {
                item.summary = excerpt;
                item.matched_by.push("summary".to_owned());
            }
            (!item.matched_by.is_empty()).then_some(item)
        })
        .collect()
}

/// Matching is authoritative on the complete lowercase body, even if a literal
/// excerpt changes the context of Greek final sigma when lowercased in isolation.
fn body_excerpt(body: &str, query: &str) -> Option<String> {
    let normalized = body.to_lowercase();
    let start = normalized.find(query)?;
    let end = start + query.len();
    if body.is_ascii() {
        return Some(excerpt_window(body, start, end, body.len()));
    }
    let (mut offset, mut first, mut last, mut count) = (0, None, 0, 0);
    for (index, character) in body.chars().enumerate() {
        // Rust's whole-string contextual mapping changes Σ to σ or ς; both
        // have the same UTF-8 width. Scalar lowercase gives the correct width
        // for every other mapping, including the expanding İ -> i + U+0307.
        let width: usize = character.to_lowercase().map(char::len_utf8).sum();
        if offset < end && offset + width > start {
            first.get_or_insert(index);
            last = index + 1;
        }
        offset += width;
        count = index + 1;
    }
    let first = first.expect("nonempty normalized match has a source scalar");
    Some(excerpt_window(body, first, last, count))
}

fn excerpt_window(body: &str, first: usize, last: usize, count: usize) -> String {
    if count <= 512 {
        return body.to_owned();
    }
    // Reserve both ellipses. Lowercasing never removes a source scalar, so a
    // validated <=256-byte query cannot require more than this payload budget.
    let payload = 510;
    let start = first
        .saturating_sub((payload - (last - first)) / 2)
        .min(count - payload);
    let end = start + payload;
    let mut excerpt = String::new();
    if start > 0 {
        excerpt.push('…');
    }
    if body.is_ascii() {
        excerpt.push_str(&body[start..end]);
    } else {
        excerpt.extend(body.chars().skip(start).take(payload));
    }
    if end < count {
        excerpt.push('…');
    }
    excerpt
}

#[cfg(test)]
#[path = "retrieval_match/tests.rs"]
mod tests;
