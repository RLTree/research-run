use super::*;

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
