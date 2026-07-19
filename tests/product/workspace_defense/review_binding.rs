use super::*;

#[test]
fn review_binding_stales_on_same_id_subject_mutation() {
    let project = workspace("review-content-binding");
    add_subject_graph(&project.0);
    add_review(&project.0, "review-first");
    assert_eq!(assessment(&project.0), "supported");

    mutate_json(
        &project.0.join(".research-run/evidence/evidence-one.json"),
        "stance",
        "contradicts",
    );
    assert_stale_subject_is_unreviewed(&project.0, "review-first");

    add_review(&project.0, "review-second");
    assert_eq!(assessment(&project.0), "supported");
    mutate_json(
        &project.0.join(".research-run/sources/source-one.json"),
        "notes",
        "Changed source assessment",
    );
    assert_stale_subject_is_unreviewed(&project.0, "review-second");
}

fn add_subject_graph(project: &Path) {
    succeeds(
        project,
        &[
            "source",
            "add",
            "--id",
            "source-one",
            "--citation",
            "Synthetic source",
            "--locator",
            "local:source",
            "--provenance",
            "human",
        ],
    );
    succeeds(
        project,
        &[
            "claim",
            "add",
            "--id",
            "claim-one",
            "--text",
            "Synthetic claim",
            "--scope",
            "Synthetic only",
            "--owner",
            "Example Researcher",
            "--authorship",
            "human",
        ],
    );
    succeeds(
        project,
        &[
            "evidence",
            "add",
            "--id",
            "evidence-one",
            "--claim",
            "claim-one",
            "--source",
            "source-one",
            "--stance",
            "supports",
            "--specific-evidence",
            "Specific evidence",
            "--authorship",
            "human",
        ],
    );
}

fn add_review(project: &Path, id: &str) {
    succeeds(
        project,
        &[
            "review",
            "add",
            "--id",
            id,
            "--claim",
            "claim-one",
            "--decision",
            "supported",
            "--rationale",
            "Reviewed subject authority",
            "--reviewer",
            "Example Researcher",
        ],
    );
}

fn mutate_json(path: &Path, field: &str, replacement: &str) {
    let mut value: Value = serde_json::from_slice(&fs::read(path).expect("read subject")).unwrap();
    value[field] = Value::String(replacement.to_owned());
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).expect("mutate subject");
}

fn assessment(project: &Path) -> String {
    let status: Value = serde_json::from_slice(&succeeds(project, &["status", "--json"]).stdout)
        .expect("status JSON");
    status["claims"][0]["assessment"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn assert_stale_subject_is_unreviewed(project: &Path, review_id: &str) {
    assert!(!cli(project, &["validate", "--json"]).status.success());
    assert_eq!(assessment(project), "unreviewed");
    let review: Value = serde_json::from_slice(
        &succeeds(project, &["show", "--kind", "review", "--id", review_id]).stdout,
    )
    .expect("review projection");
    assert_eq!(review["stale"], true);
    let handoff: Value = serde_json::from_slice(
        &succeeds(
            project,
            &[
                "handoff",
                "create",
                "--id",
                "handoff-stale",
                "--generated-at",
                "2026-07-18T20:10:00Z",
            ],
        )
        .stdout,
    )
    .expect("handoff JSON");
    assert!(
        handoff["context"]["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| {
                item["kind"] == "review" && item["id"] == review_id && item["stale"] == true
            })
    );
}
