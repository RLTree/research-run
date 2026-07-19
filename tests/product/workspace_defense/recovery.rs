use super::*;

#[path = "recovery/conflict_cleanup.rs"]
mod conflict_cleanup;

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
fn recovery_validates_later_directories_before_any_publication() {
    let project = workspace("recovery-batch-validation");
    let source_pending = project
        .0
        .join(".research-run/sources/.source-early.json.999.0.tmp");
    fs::write(
        &source_pending,
        r#"{
  "schema_version": 1,
  "kind": "source",
  "id": "source-early",
  "citation": "Valid early source",
  "locator": "local:early",
  "provenance": "human",
  "notes": ""
}
"#,
    )
    .expect("write valid early pending fixture");
    let evidence_pending = project
        .0
        .join(".research-run/evidence/.evidence-late.json.999.1.tmp");
    fs::write(
        &evidence_pending,
        r#"{
  "schema_version": 1,
  "kind": "evidence",
  "id": "evidence-late",
  "claim_id": "missing-claim",
  "source_id": "source-early",
  "experiment_id": null,
  "artifact": null,
  "stance": "supports",
  "specific_evidence": "Invalid later evidence",
  "authorship": "human"
}
"#,
    )
    .expect("write invalid later pending fixture");
    let output = cli(&project.0, &["recover", "--json"]);
    assert!(!output.status.success());
    assert!(source_pending.exists());
    assert!(evidence_pending.exists());
    assert!(
        !project
            .0
            .join(".research-run/sources/source-early.json")
            .exists()
    );
    assert!(
        !project
            .0
            .join(".research-run/evidence/evidence-late.json")
            .exists()
    );
}
