#![cfg(coverage)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "research-run-coverage-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create coverage workspace");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(cwd: &Path, args: &[&str], fault: Option<&str>) -> Output {
    let mut command =
        Command::new(std::env::var("CARGO_BIN_EXE_research-run").expect("binary path"));
    command.current_dir(cwd).args(args);
    if let Some(fault) = fault {
        command.env("RESEARCH_RUN_COVERAGE_FAULT", fault);
    }
    command.output().expect("run coverage command")
}

fn initialize(label: &str) -> TempDir {
    let root = TempDir::new(label);
    assert!(
        run(&root.0, &["init", ".", "--name", "Coverage"], None)
            .status
            .success()
    );
    root
}

fn add_source(root: &Path, id: &str, fault: Option<&str>) -> Output {
    run(
        root,
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
            "human",
        ],
        fault,
    )
}

fn pending_source(root: &Path, id: &str, sequence: u32) -> PathBuf {
    let path = root
        .join(".research-run/sources")
        .join(format!(".{id}.json.9.{sequence}.tmp"));
    let bytes = serde_json::to_vec_pretty(&json!({
        "schema_version": 1,
        "kind": "source",
        "id": id,
        "citation": "Citation",
        "locator": "local:source",
        "provenance": "human",
        "notes": ""
    }))
    .expect("source JSON");
    fs::write(&path, bytes).expect("write pending source");
    path
}

#[path = "resilience/cli_boundaries.rs"]
mod cli_boundaries;
#[path = "resilience/diagnostics.rs"]
mod diagnostics;
#[path = "resilience/filesystem_lifecycle.rs"]
mod filesystem_lifecycle;
#[path = "resilience/record_integrity.rs"]
mod record_integrity;
#[path = "resilience/recovery.rs"]
mod recovery;
#[path = "resilience/recovery_transitions.rs"]
mod recovery_transitions;
#[path = "resilience/reference_integrity.rs"]
mod reference_integrity;
#[path = "resilience/storage.rs"]
mod storage;
