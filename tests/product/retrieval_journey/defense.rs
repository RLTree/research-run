use super::*;

#[test]
fn retrieval_rejects_empty_queries_and_invalid_limits() {
    let temporary = TempDir::new("retrieval-defense");
    let project = setup(&temporary);
    for args in [
        vec!["search", "   "],
        vec!["list", "--limit", "0"],
        vec!["recent", "--limit", "257"],
        vec!["timeline", "--limit", "0"],
        vec!["unresolved", "--limit", "0"],
        vec!["blockers", "--limit", "0"],
        vec!["next", "--limit", "0"],
        vec![
            "related",
            "--kind",
            "knowledge",
            "--id",
            "x",
            "--limit",
            "0",
        ],
        vec!["context", "--limit", "0"],
        vec!["context", "--query", "   "],
        vec![
            "handoff",
            "create",
            "--id",
            "handoff-invalid",
            "--generated-at",
            "2026-07-18T20:00:00Z",
            "--limit",
            "0",
        ],
    ] {
        assert!(!cli(&project, &args).status.success(), "{args:?}");
    }
    let missing = cli(
        &project,
        &["show", "--kind", "knowledge", "--id", "not-present"],
    );
    assert_eq!(missing.status.code(), Some(3));
    let empty = TempDir::new("retrieval-empty");
    let empty_project = empty.0.join("project");
    succeeds(
        &empty.0,
        &unanchored_init(empty_project.to_str().unwrap(), "Empty"),
    );
    let human = succeeds(&empty_project, &["list", "--human"]);
    assert!(String::from_utf8_lossy(&human.stdout).contains("- None."));
}
