use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "research-run-adversarial-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create temporary test directory");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn cli(cwd: &Path, args: &[&str]) -> Output {
    Command::new(std::env::var("CARGO_BIN_EXE_research-run").expect("binary path"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run research-run")
}

fn succeeds(cwd: &Path, args: &[&str]) -> Output {
    let output = cli(cwd, args);
    assert!(
        output.status.success(),
        "command {args:?} failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn workspace(label: &str) -> TempDir {
    let temporary = TempDir::new(label);
    succeeds(
        &temporary.0,
        &["init", ".", "--name", "Adversarial fixture"],
    );
    temporary
}

#[test]
fn ai_drafts_evidence_and_retry_cannot_promote_a_claim() {
    let project = workspace("ai-promotion");
    succeeds(
        &project.0,
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
        &project.0,
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
    succeeds(&project.0, &evidence);
    succeeds(&project.0, &evidence);
    let status: Value = serde_json::from_slice(&succeeds(&project.0, &["status", "--json"]).stdout)
        .expect("status JSON");
    assert_eq!(status["claims"][0]["assessment"], "unreviewed");

    succeeds(
        &project.0,
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
    let reviewed: Value =
        serde_json::from_slice(&succeeds(&project.0, &["status", "--json"]).stdout)
            .expect("reviewed status JSON");
    assert_eq!(reviewed["claims"][0]["assessment"], "supported");
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
fn status_projection_is_byte_deterministic() {
    let project = workspace("determinism");
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
    let first = succeeds(&project.0, &["status", "--json"]).stdout;
    let second = succeeds(&project.0, &["status", "--json"]).stdout;
    assert_eq!(first, second);
}
