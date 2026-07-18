use super::*;

#[test]
fn malformed_input_is_rejected_without_echoing_record_content() {
    let project = workspace("redaction");
    let secret_marker = "RESEARCH_RUN_SECRET_MARKER_7391";
    fs::write(
        project.0.join(".research-run/claims/tampered.json"),
        format!("{{\"secret\":\"{secret_marker}\",BROKEN"),
    )
    .expect("write malformed fixture");
    let output = cli(&project.0, &["validate", "--json"]);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("malformed JSON"));
    assert!(!combined.contains(secret_marker));
}

#[test]
fn interrupted_publication_requires_explicit_validated_recovery() {
    let project = workspace("recovery");
    let pending = project
        .0
        .join(".research-run/sources/.source-recovered.json.999.0.tmp");
    fs::write(
        &pending,
        r#"{
  "schema_version": 1,
  "kind": "source",
  "id": "source-recovered",
  "citation": "Recovered synthetic source",
  "locator": "local:recovered",
  "provenance": "human",
  "notes": ""
}
"#,
    )
    .expect("write pending fixture");

    let blocked = cli(&project.0, &["validate", "--json"]);
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stdout).contains("recover"));
    let recovery: Value =
        serde_json::from_slice(&succeeds(&project.0, &["recover", "--json"]).stdout)
            .expect("recovery JSON");
    assert_eq!(
        recovery["recovered"].as_array().expect("recovered").len(),
        1
    );
    assert!(!pending.exists());
    assert!(
        project
            .0
            .join(".research-run/sources/source-recovered.json")
            .exists()
    );
    succeeds(&project.0, &["validate", "--json"]);
}

#[test]
fn recovery_rejects_pending_records_with_unknown_references() {
    let project = workspace("recovery-reference");
    let pending = project
        .0
        .join(".research-run/evidence/.evidence-invalid.json.999.0.tmp");
    fs::write(
        &pending,
        r#"{
  "schema_version": 1,
  "kind": "evidence",
  "id": "evidence-invalid",
  "claim_id": "missing-claim",
  "source_id": "missing-source",
  "experiment_id": null,
  "artifact": null,
  "stance": "supports",
  "specific_evidence": "This must not publish.",
  "authorship": "human"
}
"#,
    )
    .expect("write invalid pending fixture");
    let output = cli(&project.0, &["recover", "--json"]);
    assert!(!output.status.success());
    assert!(pending.exists());
    assert!(
        !project
            .0
            .join(".research-run/evidence/evidence-invalid.json")
            .exists()
    );
}

#[test]
fn recovery_preflights_conflicting_pending_content_before_any_publication() {
    let project = workspace("recovery-conflict");
    let directory = project.0.join(".research-run/sources");
    for (sequence, citation) in [("0", "First content"), ("1", "Conflicting content")] {
        fs::write(
            directory.join(format!(".source-conflict.json.999.{sequence}.tmp")),
            format!(
                "{{\n  \"schema_version\": 1,\n  \"kind\": \"source\",\n  \"id\": \"source-conflict\",\n  \"citation\": \"{citation}\",\n  \"locator\": \"local:conflict\",\n  \"provenance\": \"human\",\n  \"notes\": \"\"\n}}\n"
            ),
        )
        .expect("write conflicting pending fixture");
    }
    let output = cli(&project.0, &["recover", "--json"]);
    assert!(!output.status.success());
    assert!(!directory.join("source-conflict.json").exists());
}

#[test]
fn explicit_recovery_completes_init_interrupted_before_manifest_publication() {
    let temporary = TempDir::new("recovery-init");
    let project = temporary.0.join("partial-project");
    let state = project.join(".research-run");
    for directory in ["sources", "claims", "evidence", "experiments", "reviews"] {
        fs::create_dir_all(state.join(directory)).expect("create partial workspace");
    }
    fs::write(
        state.join(".manifest.json.999.0.tmp"),
        r#"{
  "schema_version": 1,
  "kind": "project-manifest",
  "project_id": "partial-project",
  "name": "Partial project",
  "declared_roots": ["."]
}
"#,
    )
    .expect("write pending manifest");
    let project_text = project.to_string_lossy();
    succeeds(&temporary.0, &["recover", &project_text, "--json"]);
    succeeds(&project, &["validate", "--json"]);
}

#[cfg(unix)]
#[test]
fn symlinked_record_fails_closed_and_does_not_leak_target_content() {
    use std::os::unix::fs::symlink;

    let project = workspace("symlink");
    let outside = project.0.join("outside.txt");
    let secret_marker = "OUTSIDE_SECRET_MARKER_1824";
    fs::write(&outside, secret_marker).expect("write outside fixture");
    symlink(
        &outside,
        project.0.join(".research-run/claims/claim-link.json"),
    )
    .expect("create symlink fixture");
    let output = cli(&project.0, &["validate", "--json"]);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("symlink"));
    assert!(!combined.contains(secret_marker));
}

#[test]
fn identical_pending_review_is_cleanup_not_a_semantic_conflict() {
    let project = workspace("identical-review");
    succeeds(
        &project.0,
        &[
            "claim",
            "add",
            "--id",
            "claim-one",
            "--text",
            "Claim",
            "--scope",
            "Scope",
            "--owner",
            "Researcher",
            "--authorship",
            "human",
        ],
    );
    succeeds(
        &project.0,
        &[
            "review",
            "add",
            "--id",
            "review-one",
            "--claim",
            "claim-one",
            "--decision",
            "limited",
            "--rationale",
            "Bounded support",
            "--reviewer",
            "Researcher",
        ],
    );
    let reviews = project.0.join(".research-run/reviews");
    let canonical = reviews.join("review-one.json");
    let pending = reviews.join(".review-one.json.999.0.tmp");
    fs::copy(&canonical, &pending).expect("copy identical pending review");
    let recovery: Value =
        serde_json::from_slice(&succeeds(&project.0, &["recover", "--json"]).stdout)
            .expect("recovery result");
    assert_eq!(
        recovery["discarded_identical"]
            .as_array()
            .expect("discarded")
            .len(),
        1
    );
    assert!(!pending.exists());
    succeeds(&project.0, &["validate", "--json"]);
}
