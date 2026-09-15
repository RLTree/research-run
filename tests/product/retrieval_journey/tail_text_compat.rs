use super::*;

#[test]
fn tail_text_typed_identity_and_existing_matches_preserve_projections() {
    let temporary = TempDir::new("tail-text-compat");
    let project = setup(&temporary);
    seed_tail_records(&temporary, &project);
    let tail = json_output(&project, &["search", "uniquetail"]);
    assert_eq!(tail["total_matches"], 1);
    assert_eq!(tail["items"][0]["kind"], "knowledge");
    let identity = json_output(&project, &["search", "shared-id"]);
    assert_eq!(identity["total_matches"], 2);
    assert_eq!(identity["items"][0]["kind"], "knowledge");
    assert_eq!(identity["items"][1]["kind"], "source");
    assert_typed_relationship_selection(&temporary, &project);
    let mut record = knowledge(
        "titlemarker-id",
        "observation",
        "titlemarker title",
        "2026-07-18T20:00:00Z",
    );
    record["body"] = json!(format!("{} titlemarker", "x".repeat(600)));
    add(&temporary, &project, "knowledge", record);
    let before = unchanged_views(&project);
    let shown = json_output(
        &project,
        &["show", "--kind", "knowledge", "--id", "titlemarker-id"],
    );
    for (view, field) in before.iter().zip(["matches", "items", "items"]) {
        let item = view[field]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "titlemarker-id")
            .unwrap();
        assert_eq!(item["summary"], shown["summary"]);
        assert_eq!(item["matched_by"], json!([]));
    }
    let search = json_output(&project, &["search", "titlemarker"]);
    assert_eq!(search["total_matches"], 1);
    assert_eq!(search["items"][0]["summary"], shown["summary"]);
    assert_eq!(
        search["items"][0]["matched_by"],
        json!(["id", "title", "authority-path"])
    );
    let context = json_output(&project, &["context", "--query", "titlemarker"]);
    assert_eq!(context["matches"], search["items"]);
    assert_eq!(unchanged_views(&project), before);
}

fn seed_tail_records(temporary: &TempDir, project: &Path) {
    body_record(
        temporary,
        project,
        "shared-id",
        &format!("{} other-tail", "x".repeat(600)),
    );
    succeeds(
        project,
        &[
            "source",
            "add",
            "--id",
            "shared-id",
            "--citation",
            "Synthetic citation",
            "--locator",
            "local:synthetic",
            "--provenance",
            "human",
        ],
    );
    succeeds(
        project,
        &[
            "source",
            "add",
            "--id",
            "source-target",
            "--citation",
            "Synthetic target",
            "--locator",
            "local:synthetic-target",
            "--provenance",
            "human",
        ],
    );
    body_record(
        temporary,
        project,
        "tail-record",
        &format!("{} uniquetail", "x".repeat(600)),
    );
}

fn unchanged_views(project: &Path) -> Vec<Value> {
    [vec!["context"], vec!["list"], vec!["recent"]]
        .iter()
        .map(|args| json_output(project, args))
        .collect()
}

fn typed_relationship(
    id: &str,
    from_kind: &str,
    from_id: &str,
    to_kind: &str,
    to_id: &str,
) -> Value {
    json!({
        "schema_version": 1,
        "kind": "relationship",
        "id": id,
        "relationship": "related-to",
        "from": { "kind": from_kind, "id": from_id },
        "to": { "kind": to_kind, "id": to_id },
        "rationale": "Synthetic typed identity relationship",
        "occurred_at": "2026-07-18T20:05:00Z",
        "authorship": "human"
    })
}

fn assert_typed_relationship_selection(temporary: &TempDir, project: &Path) {
    for relationship in [
        typed_relationship(
            "source-only-link",
            "source",
            "shared-id",
            "source",
            "source-target",
        ),
        typed_relationship(
            "knowledge-outgoing-link",
            "knowledge",
            "shared-id",
            "knowledge",
            "tail-record",
        ),
        typed_relationship(
            "knowledge-incoming-link",
            "knowledge",
            "tail-record",
            "knowledge",
            "shared-id",
        ),
    ] {
        add(temporary, project, "relationship", relationship);
    }
    let context = json_output(project, &["context", "--query", "uniquetail"]);
    let relationships = context["relationships"].as_array().unwrap();
    let ids = relationships
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec!["knowledge-incoming-link", "knowledge-outgoing-link"]
    );
    assert!(!ids.contains(&"source-only-link"));
    let limited = json_output(
        project,
        &["context", "--query", "uniquetail", "--limit", "1"],
    );
    assert_eq!(limited["relationships"][0]["id"], "knowledge-incoming-link");
}

#[test]
fn tail_text_query_byte_budget_and_search_limit_fail_closed() {
    let temporary = TempDir::new("tail-text-query-budget");
    let project = setup(&temporary);
    for query in ["x".repeat(257), format!("{}x", "é".repeat(128))] {
        for args in [
            vec!["search", query.as_str()],
            vec!["context", "--query", query.as_str()],
        ] {
            assert!(!cli(&project, &args).status.success());
        }
    }
    assert!(
        !cli(&project, &["search", "signal", "--limit", "0"])
            .status
            .success()
    );
    assert!(
        !cli(&project, &["context", "--query", "signal", "--limit", "0"])
            .status
            .success()
    );
}
