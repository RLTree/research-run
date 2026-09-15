use super::*;
use std::path::{Path, PathBuf};

fn body_record(temporary: &TempDir, project: &Path, id: &str, body: &str) {
    let mut record = knowledge(id, "observation", "Synthetic note", "2026-07-18T20:00:00Z");
    record["body"] = json!(body);
    add(temporary, project, "knowledge", record);
}

fn canonical_bytes(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(canonical_bytes(&path));
        } else {
            files.push((path.clone(), fs::read(path).unwrap()));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

#[test]
fn tail_text_search_context_and_handoff_preserve_canonical_bytes() {
    let temporary = TempDir::new("tail-text-handoff");
    let project = setup(&temporary);
    let body = format!("prefixneedle {} tailneedle", "x".repeat(65536 - 24));
    assert_eq!(body.len(), 65536);
    body_record(&temporary, &project, "tail-record", &body);
    let before = canonical_bytes(&project.join(".research-run"));
    let shown = json_output(
        &project,
        &["show", "--kind", "knowledge", "--id", "tail-record"],
    );
    let prefix = json_output(&project, &["search", "prefixneedle"]);
    assert_eq!(prefix["items"][0]["summary"], shown["summary"]);
    let search = json_output(&project, &["search", " tailneedle "]);
    assert_eq!(search["total_matches"], 1);
    let item = &search["items"][0];
    assert_eq!(item["id"], "tail-record");
    assert_eq!(item["matched_by"], json!(["summary"]));
    let excerpt = item["summary"].as_str().unwrap();
    assert!(excerpt.ends_with("tailneedle"));
    assert!(excerpt.chars().count() <= 512);
    let context = json_output(&project, &["context", "--query", "tailneedle"]);
    assert_eq!(context["matches"], search["items"]);
    assert_eq!(before, canonical_bytes(&project.join(".research-run")));
    assert_handoff(&temporary, &project, &context);
    for (path, bytes) in before {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    succeeds(&project, &["validate", "--json"]);
}

fn assert_handoff(temporary: &TempDir, project: &Path, context: &Value) {
    let handoff = json_output(
        project,
        &[
            "handoff",
            "create",
            "--id",
            "handoff-tail",
            "--generated-at",
            "2026-07-18T21:00:00Z",
            "--query",
            "tailneedle",
        ],
    );
    assert_eq!(handoff["schema_version"], 2);
    assert_eq!(handoff["context"], *context);
    let path = temporary.0.join("tail-handoff.json");
    fs::write(&path, serde_json::to_vec(&handoff).unwrap()).unwrap();
    let inspected = json_output(
        &temporary.0,
        &["handoff", "inspect", "--input", path.to_str().unwrap()],
    );
    assert_eq!(inspected, handoff);
}

#[test]
fn tail_text_order_limits_and_history_remain_visible() {
    let temporary = TempDir::new("tail-text-history");
    let project = setup(&temporary);
    for id in ["tail-z", "tail-a", "tail-m"] {
        body_record(
            &temporary,
            &project,
            id,
            &format!("{} orderedneedle", "x".repeat(600)),
        );
    }
    add(
        &temporary,
        &project,
        "relationship",
        relationship("tail-revision", "tail-z", "tail-a"),
    );
    let mut invalidation = relationship("tail-invalidation", "tail-z", "tail-m");
    invalidation["relationship"] = json!("invalidates");
    add(&temporary, &project, "relationship", invalidation);
    let full = json_output(&project, &["search", "orderedneedle"]);
    assert_eq!(full["total_matches"], 3);
    let items = full["items"].as_array().unwrap();
    assert_eq!(
        items
            .iter()
            .map(|v| v["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["tail-a", "tail-m", "tail-z"]
    );
    assert_eq!(items[0]["stale"], true);
    assert_eq!(items[1]["invalidated"], true);
    let limited = json_output(&project, &["search", "orderedneedle", "--limit", "1"]);
    assert_eq!(limited["total_matches"], 3);
    assert_eq!(limited["items"], json!([items[0]]));
    let context = json_output(
        &project,
        &["context", "--query", "orderedneedle", "--limit", "1"],
    );
    assert_eq!(context["matches"], limited["items"]);
    assert_eq!(
        json_output(&project, &["search", "absentneedle"])["total_matches"],
        0
    );
    let human = succeeds(&project, &["search", "orderedneedle", "--human"]);
    assert!(String::from_utf8_lossy(&human.stdout).contains("summary (knowledge body)"));
}

#[test]
fn tail_text_unicode_literal_excerpts_and_terminal_escaping() {
    let temporary = TempDir::new("tail-text-unicode");
    let project = setup(&temporary);
    for (id, body, query, literal) in [
        (
            "tail-expanded",
            format!("{} İSTANBUL 🧬 e\u{301}", "x".repeat(600)),
            "i\u{307}stanbul",
            "İSTANBUL",
        ),
        (
            "tail-emoji",
            format!("{} 🧬e\u{301}marker", "x".repeat(600)),
            "🧬e\u{301}",
            "🧬e\u{301}",
        ),
        (
            "tail-sigma",
            format!("{} AΣ{}B", "x".repeat(600), "\u{301}".repeat(600)),
            "aσ",
            "AΣ",
        ),
        (
            "tail-boundary",
            format!("{} boundaryneedle", "x".repeat(505)),
            "boundaryneedle",
            "boundaryneedle",
        ),
        (
            "tail-control",
            format!("{} controlneedle\u{1b}]52;forged\u{7}", "x".repeat(600)),
            "controlneedle",
            "controlneedle",
        ),
    ] {
        body_record(&temporary, &project, id, &body);
        let search = json_output(&project, &["search", query]);
        assert_eq!(search["total_matches"], 1, "{id}");
        let item = &search["items"][0];
        assert_eq!(item["id"], id);
        let excerpt = item["summary"].as_str().unwrap();
        assert!(excerpt.contains(literal), "{id}: {excerpt}");
        assert!(excerpt.chars().count() <= 512);
        assert_eq!(item["matched_by"], json!(["summary"]));
        let context = json_output(&project, &["context", "--query", query]);
        assert_eq!(context["matches"], search["items"]);
        let human = succeeds(&project, &["search", query, "--human"]);
        assert!(!human.stdout.contains(&0x1b));
        assert!(!human.stdout.contains(&0x07));
    }
}

#[path = "tail_text_compat.rs"]
mod compatibility;
