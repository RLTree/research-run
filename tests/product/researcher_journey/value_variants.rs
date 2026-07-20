use super::*;

#[test]
fn human_commands_cover_every_public_value_variant() {
    let temporary = TempDir::new("human-variants");
    crate::review_test_signing::initialize_with_test_authority(
        &temporary.0,
        &temporary.0,
        "Human variants",
    );
    add_each_source_provenance(&temporary.0);
    add_each_review_assessment(&temporary.0);
    add_each_evidence_stance(&temporary.0);
    add_each_experiment_outcome(&temporary.0);
    exercise_human_projections_and_invalid_artifact(&temporary.0);
}

fn add_each_source_provenance(project: &Path) {
    for (id, provenance) in [
        ("source-human", "human"),
        ("source-imported", "imported"),
        ("source-ai", "ai"),
    ] {
        succeeds(
            project,
            &[
                "source",
                "add",
                "--id",
                id,
                "--citation",
                "Citation",
                "--locator",
                "local:source",
                "--provenance",
                provenance,
            ],
        );
    }
}

fn add_each_review_assessment(project: &Path) {
    for (index, decision) in ["unsupported", "limited", "supported", "contradicted"]
        .into_iter()
        .enumerate()
    {
        let claim = format!("claim-{index}");
        let review = format!("review-{index}");
        succeeds(
            project,
            &[
                "claim",
                "add",
                "--id",
                &claim,
                "--text",
                "Claim",
                "--scope",
                "Scope",
                "--owner",
                "Researcher",
                "--authorship",
                if index == 0 { "ai" } else { "human" },
            ],
        );
        if decision == "supported" {
            succeeds(
                project,
                &[
                    "evidence",
                    "add",
                    "--id",
                    "review-evidence-2",
                    "--claim",
                    &claim,
                    "--source",
                    "source-human",
                    "--stance",
                    "supports",
                    "--specific-evidence",
                    "Evidence required for support.",
                    "--authorship",
                    "human",
                ],
            );
        }
        crate::review_test_signing::add_signed_review(
            project,
            &review,
            &claim,
            decision,
            "Rationale",
            "Researcher",
        );
    }
}

fn add_each_evidence_stance(project: &Path) {
    for (index, stance) in ["supports", "limits", "contradicts", "context"]
        .into_iter()
        .enumerate()
    {
        let evidence = format!("evidence-{index}");
        succeeds(
            project,
            &[
                "evidence",
                "add",
                "--id",
                &evidence,
                "--claim",
                "claim-0",
                "--source",
                "source-human",
                "--stance",
                stance,
                "--specific-evidence",
                "Specific",
                "--authorship",
                "human",
            ],
        );
    }
}

fn add_each_experiment_outcome(project: &Path) {
    for (index, outcome) in ["positive", "negative", "ambiguous", "inconclusive"]
        .into_iter()
        .enumerate()
    {
        let experiment = format!("experiment-{index}");
        succeeds(
            project,
            &[
                "experiment",
                "add",
                "--id",
                &experiment,
                "--question",
                "Question?",
                "--method-ref",
                "protocol.md",
                "--observation",
                "Observation",
                "--interpretation",
                "Interpretation",
                "--limitation",
                "Limitation",
                "--outcome",
                outcome,
                "--next-move",
                "Repeat",
                "--artifact",
                "external:https://example.invalid/a:Public summary",
            ],
        );
    }
}

fn exercise_human_projections_and_invalid_artifact(project: &Path) {
    succeeds(project, &["validate"]);
    succeeds(project, &["status"]);
    succeeds(project, &["recover"]);
    let bad_artifact = cli(
        project,
        &[
            "experiment",
            "add",
            "--id",
            "bad-artifact",
            "--question",
            "Question?",
            "--method-ref",
            "protocol.md",
            "--observation",
            "Observation",
            "--interpretation",
            "Interpretation",
            "--limitation",
            "Limitation",
            "--outcome",
            "positive",
            "--next-move",
            "Repeat",
            "--artifact",
            "unknown:value:description",
        ],
    );
    assert!(!bad_artifact.status.success());
}
