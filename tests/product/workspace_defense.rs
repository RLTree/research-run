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

#[path = "workspace_defense/claims.rs"]
mod claims;
#[path = "workspace_defense/locking.rs"]
mod locking;
#[path = "workspace_defense/projection.rs"]
mod projection;
#[path = "workspace_defense/recovery.rs"]
mod recovery;
#[path = "workspace_defense/review_binding.rs"]
mod review_binding;
