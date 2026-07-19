use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

#[path = "researcher_journey/value_variants.rs"]
mod value_variants;

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) struct TempDir(pub(super) PathBuf);

impl TempDir {
    pub(super) fn new(label: &str) -> Self {
        let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "research-run-{label}-{}-{sequence}",
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

pub(super) fn cli(cwd: &Path, args: &[&str]) -> Output {
    Command::new(std::env::var("CARGO_BIN_EXE_research-run").expect("binary path"))
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("run research-run")
}

pub(super) fn succeeds(cwd: &Path, args: &[&str]) -> Output {
    let output = cli(cwd, args);
    assert!(
        output.status.success(),
        "command {args:?} failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn clean_clone_journey_preserves_negative_results_and_claim_ceiling() {
    let temporary = TempDir::new("journey");
    let project = temporary.0.join("project");
    let project_text = project.to_string_lossy();
    succeeds(
        &temporary.0,
        &["init", &project_text, "--name", "Synthetic assay"],
    );
    add_source_and_claim(&project);
    add_supporting_evidence(&project);
    add_negative_repeat(&project);
    assert_claim_ceiling_before_review(&project);
    add_limited_review(&project);
    assert_limited_status(&project);
}

fn add_source_and_claim(project: &Path) {
    succeeds(
        project,
        &[
            "source",
            "add",
            "--id",
            "source-paper",
            "--citation",
            "Synthetic Paper (2026)",
            "--locator",
            "https://example.invalid/paper",
            "--provenance",
            "human",
        ],
    );
}

fn add_supporting_evidence(project: &Path) {
    succeeds(
        project,
        &[
            "claim",
            "add",
            "--id",
            "claim-binding",
            "--text",
            "The synthetic construct binds the target in this assay.",
            "--scope",
            "Synthetic in-vitro assay only",
            "--owner",
            "Example Researcher",
            "--authorship",
            "human",
        ],
    );
}

fn add_negative_repeat(project: &Path) {
    succeeds(
        project,
        &[
            "evidence",
            "add",
            "--id",
            "evidence-paper",
            "--claim",
            "claim-binding",
            "--source",
            "source-paper",
            "--stance",
            "supports",
            "--specific-evidence",
            "Figure 2 reports binding in the matching synthetic assay.",
            "--authorship",
            "human",
        ],
    );
    succeeds(
        project,
        &[
            "experiment",
            "add",
            "--id",
            "experiment-repeat",
            "--question",
            "Does the effect reproduce?",
            "--method-ref",
            "protocols/synthetic-assay.md",
            "--observation",
            "Signal was indistinguishable from the negative control.",
            "--interpretation",
            "This run does not reproduce the reported effect.",
            "--limitation",
            "One synthetic batch was tested.",
            "--outcome",
            "negative",
            "--next-move",
            "Repeat with an independently prepared batch.",
            "--artifact",
            "workspace:artifacts/summary.txt:Deidentified summary only",
        ],
    );
    succeeds(
        project,
        &[
            "evidence",
            "add",
            "--id",
            "evidence-repeat",
            "--claim",
            "claim-binding",
            "--experiment",
            "experiment-repeat",
            "--stance",
            "contradicts",
            "--specific-evidence",
            "The repeat was negative under the recorded conditions.",
            "--authorship",
            "human",
        ],
    );
}

fn assert_claim_ceiling_before_review(project: &Path) {
    let before: Value = serde_json::from_slice(&succeeds(project, &["status", "--json"]).stdout)
        .expect("status JSON");
    assert_eq!(before["claims"][0]["assessment"], "unreviewed");
    assert!(
        before["claims"][0]["blockers"][0]
            .as_str()
            .expect("blocker")
            .contains("human review")
    );
}

fn add_limited_review(project: &Path) {
    succeeds(
        project,
        &[
            "review",
            "add",
            "--id",
            "review-binding",
            "--claim",
            "claim-binding",
            "--decision",
            "limited",
            "--rationale",
            "The source supports the claim, but the negative repeat limits it.",
            "--reviewer",
            "Example Researcher",
        ],
    );
}

fn assert_limited_status(project: &Path) {
    let validation: Value =
        serde_json::from_slice(&succeeds(project, &["validate", "--json"]).stdout)
            .expect("validation JSON");
    let status: Value = serde_json::from_slice(&succeeds(project, &["status", "--json"]).stdout)
        .expect("status JSON");
    assert_eq!(validation["valid"], true);
    assert_eq!(status["claims"][0]["assessment"], "limited");
    assert_eq!(status["experiments"][0]["outcome"], "negative");
    assert_ne!(
        status["experiments"][0]["observations"][0],
        status["experiments"][0]["interpretation"]
    );
}
