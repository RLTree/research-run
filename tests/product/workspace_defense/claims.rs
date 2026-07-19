use super::*;

#[test]
fn ai_drafts_evidence_and_retry_cannot_promote_a_claim() {
    let project = workspace("ai-promotion");
    add_ai_draft_records(&project.0);
    assert_ai_drafts_cannot_promote(&project.0);
    add_human_review(&project.0);
    assert_human_review_controls_promotion(&project.0);
    add_post_review_contradiction(&project.0);
    assert_graph_change_withholds_prior_review(&project.0);
    add_re_review(&project.0);
    assert_human_review_controls_promotion(&project.0);
}

fn add_post_review_contradiction(project: &Path) {
    succeeds(
        project,
        &[
            "evidence",
            "add",
            "--id",
            "evidence-later",
            "--claim",
            "claim-ai",
            "--source",
            "source-ai",
            "--stance",
            "contradicts",
            "--specific-evidence",
            "Later material changes the reviewed graph.",
            "--authorship",
            "human",
        ],
    );
}

fn assert_graph_change_withholds_prior_review(project: &Path) {
    let status: Value = serde_json::from_slice(&succeeds(project, &["status", "--json"]).stdout)
        .expect("stale review status JSON");
    assert_eq!(status["claims"][0]["assessment"], "unreviewed");
    assert!(
        !status["claims"][0]["blockers"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn add_re_review(project: &Path) {
    succeeds(
        project,
        &[
            "review",
            "add",
            "--id",
            "review-later",
            "--claim",
            "claim-ai",
            "--decision",
            "supported",
            "--rationale",
            "A human reviewed the changed evidence graph.",
            "--reviewer",
            "Example Researcher",
        ],
    );
}

fn add_ai_draft_records(project: &Path) {
    succeeds(
        project,
        &[
            "source",
            "add",
            "--id",
            "source-ai",
            "--citation",
            "Synthetic AI draft",
            "--locator",
            "local:draft",
            "--provenance",
            "ai",
        ],
    );
    succeeds(
        project,
        &[
            "claim",
            "add",
            "--id",
            "claim-ai",
            "--text",
            "An unreviewed draft claim.",
            "--scope",
            "Synthetic only",
            "--owner",
            "Example Researcher",
            "--authorship",
            "ai",
        ],
    );
    let evidence = [
        "evidence",
        "add",
        "--id",
        "evidence-ai",
        "--claim",
        "claim-ai",
        "--source",
        "source-ai",
        "--stance",
        "supports",
        "--specific-evidence",
        "AI-selected text appears supportive.",
        "--authorship",
        "ai",
    ];
    succeeds(project, &evidence);
    succeeds(project, &evidence);
}

fn assert_ai_drafts_cannot_promote(project: &Path) {
    let status: Value = serde_json::from_slice(&succeeds(project, &["status", "--json"]).stdout)
        .expect("status JSON");
    assert_eq!(status["claims"][0]["assessment"], "unreviewed");
    assert_eq!(
        status["unreviewed_ai_drafts"]
            .as_array()
            .expect("AI drafts")
            .len(),
        3
    );
}

fn add_human_review(project: &Path) {
    succeeds(
        project,
        &[
            "review",
            "add",
            "--id",
            "review-ai",
            "--claim",
            "claim-ai",
            "--decision",
            "supported",
            "--rationale",
            "A human checked the cited passage within the stated scope.",
            "--reviewer",
            "Example Researcher",
        ],
    );
}

fn assert_human_review_controls_promotion(project: &Path) {
    let reviewed: Value = serde_json::from_slice(&succeeds(project, &["status", "--json"]).stdout)
        .expect("reviewed status JSON");
    assert_eq!(reviewed["claims"][0]["assessment"], "supported");
    assert!(
        reviewed["unreviewed_ai_drafts"]
            .as_array()
            .expect("AI drafts")
            .is_empty()
    );
    assert!(
        reviewed["claim_ceiling"]
            .as_str()
            .expect("claim ceiling")
            .contains("do not establish scientific truth")
    );
}

#[test]
fn unknown_reference_and_path_escape_fail_before_effect() {
    let project = workspace("references");
    succeeds(
        &project.0,
        &[
            "claim",
            "add",
            "--id",
            "claim-one",
            "--text",
            "Synthetic claim.",
            "--scope",
            "Synthetic only",
            "--owner",
            "Example Researcher",
            "--authorship",
            "human",
        ],
    );
    let unknown = cli(
        &project.0,
        &[
            "evidence",
            "add",
            "--id",
            "evidence-unknown",
            "--claim",
            "claim-one",
            "--source",
            "missing-source",
            "--stance",
            "supports",
            "--specific-evidence",
            "Should not publish.",
            "--authorship",
            "human",
        ],
    );
    assert!(!unknown.status.success());
    assert!(
        !project
            .0
            .join(".research-run/evidence/evidence-unknown.json")
            .exists()
    );

    let escape = cli(
        &project.0,
        &[
            "experiment",
            "add",
            "--id",
            "experiment-escape",
            "--question",
            "Can this escape?",
            "--method-ref",
            "protocol.md",
            "--observation",
            "No effect should occur.",
            "--interpretation",
            "Rejected input.",
            "--limitation",
            "Synthetic fixture.",
            "--outcome",
            "ambiguous",
            "--next-move",
            "Use a confined path.",
            "--artifact",
            "workspace:../secret.txt:Forbidden pointer",
        ],
    );
    assert!(!escape.status.success());
    assert!(
        !project
            .0
            .join(".research-run/experiments/experiment-escape.json")
            .exists()
    );
}
