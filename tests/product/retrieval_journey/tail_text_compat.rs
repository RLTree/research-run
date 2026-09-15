use super::*;

#[test]
fn tail_text_typed_identity_and_existing_matches_preserve_projections() {
    let temporary = TempDir::new("tail-text-compat");
    let project = setup(&temporary);
    body_record(
        &temporary,
        &project,
        "shared-id",
        &format!("{} uniquetail", "x".repeat(600)),
    );
    succeeds(
        &project,
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
    let tail = json_output(&project, &["search", "uniquetail"]);
    assert_eq!(tail["total_matches"], 1);
    assert_eq!(tail["items"][0]["kind"], "knowledge");
    let identity = json_output(&project, &["search", "shared-id"]);
    assert_eq!(identity["total_matches"], 2);
    assert_eq!(identity["items"][0]["kind"], "knowledge");
    assert_eq!(identity["items"][1]["kind"], "source");
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

fn unchanged_views(project: &Path) -> Vec<Value> {
    [vec!["context"], vec!["list"], vec!["recent"]]
        .iter()
        .map(|args| json_output(project, args))
        .collect()
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
