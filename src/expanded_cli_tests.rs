use std::fs;
use std::io::{self, Read};

use clap::Parser;

use super::input::{
    is_allowed_platform_alias, read_json_input, read_limited, reject_symlink_chain,
};
use super::{Cli, execute, inject_current_directory};
use crate::domain::RelationshipRecord;
use crate::workspace::Workspace;

struct FailedReader;

impl Read for FailedReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("injected input failure"))
    }
}

#[test]
fn bounded_structured_reader_reports_budget_and_io_failures() {
    assert!(read_limited(&mut io::Cursor::new(b"{}".to_vec())).is_ok());
    assert!(read_limited(&mut io::repeat(0).take(1_048_577)).is_err());
    assert!(read_limited(&mut FailedReader).is_err());
}

#[test]
fn structured_input_reports_non_not_found_metadata_and_open_failures() {
    assert!(read_json_input::<RelationshipRecord>("\0").is_err());
    let root = std::env::temp_dir().join(format!("research-run-input-{}", std::process::id()));
    fs::create_dir_all(&root).expect("input directory");
    let path = root.join("unreadable.json");
    fs::write(&path, b"{}").expect("input file");
    assert!(read_json_input::<RelationshipRecord>(path.to_str().unwrap()).is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("remove permissions");
        assert!(read_json_input::<RelationshipRecord>(path.to_str().unwrap()).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("restore permissions");
    }
    fs::remove_dir_all(root).expect("remove input directory");

    #[cfg(target_os = "macos")]
    {
        let metadata = fs::symlink_metadata("/var").expect("/var metadata");
        assert!(is_allowed_platform_alias(
            std::path::Path::new("/var"),
            &metadata
        ));
    }
}

#[cfg(unix)]
#[test]
fn structured_input_symlink_chain_distinguishes_absence_errors_and_links() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!("research-run-input-path-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).expect("input path root");
    assert!(reject_symlink_chain(&root.join("missing.json")).is_ok());
    assert!(reject_symlink_chain(std::path::Path::new("\0")).is_err());
    let target = root.join("target.json");
    fs::write(&target, b"{}").expect("target");
    let linked = root.join("linked.json");
    symlink(&target, &linked).expect("input symlink");
    assert!(reject_symlink_chain(&linked).is_err());
    assert!(reject_symlink_chain(&target).is_ok());
    fs::remove_dir_all(root).expect("remove input path root");
}

#[test]
fn expanded_cli_routes_execute_against_an_injected_workspace() {
    let root = expanded_workspace();
    assert_expanded_route_failures(&root);
    assert_expanded_route_successes(&root);
    fs::remove_dir_all(root).expect("remove fixture");
}

fn expanded_workspace() -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("research-run-expanded-cli-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).expect("fixture root");
    Workspace::initialize(&root, "Expanded CLI").expect("initialize");
    let knowledge = root.join("knowledge.json");
    fs::write(
        &knowledge,
        br#"{"schema_version":1,"kind":"knowledge","id":"knowledge-one","record_type":"blocker","title":"Blocker","body":"Body","occurred_at":"2026-07-18T20:00:00Z","state":"open","authorship":"human"}"#,
    )
    .expect("knowledge input");
    run_at(
        &root,
        &[
            "research-run",
            "knowledge",
            "add",
            "--input",
            knowledge.to_str().unwrap(),
        ],
    )
    .expect("add knowledge");
    root
}

fn assert_expanded_route_failures(root: &std::path::Path) {
    for args in [
        vec!["research-run", "list", "--limit", "0"],
        vec![
            "research-run",
            "show",
            "--kind",
            "knowledge",
            "--id",
            "missing",
        ],
        vec!["research-run", "search", "   "],
        vec!["research-run", "recent", "--limit", "0"],
        vec!["research-run", "timeline", "--limit", "0"],
        vec!["research-run", "unresolved", "--limit", "0"],
        vec!["research-run", "blockers", "--limit", "0"],
        vec!["research-run", "next", "--limit", "0"],
        vec![
            "research-run",
            "related",
            "--kind",
            "knowledge",
            "--id",
            "knowledge-one",
            "--limit",
            "0",
        ],
        vec!["research-run", "context", "--limit", "0"],
        vec![
            "research-run",
            "handoff",
            "create",
            "--id",
            "handoff-invalid",
            "--generated-at",
            "2026-07-18T20:01:00Z",
            "--limit",
            "0",
        ],
    ] {
        assert!(run_at(root, &args).is_err(), "{args:?}");
    }
}

fn assert_expanded_route_successes(root: &std::path::Path) {
    for args in [
        vec!["research-run", "list"],
        vec![
            "research-run",
            "show",
            "--kind",
            "knowledge",
            "--id",
            "knowledge-one",
        ],
        vec!["research-run", "search", "Blocker"],
        vec!["research-run", "recent"],
        vec!["research-run", "timeline"],
        vec!["research-run", "unresolved"],
        vec!["research-run", "blockers"],
        vec!["research-run", "next"],
        vec![
            "research-run",
            "related",
            "--kind",
            "knowledge",
            "--id",
            "knowledge-one",
        ],
        vec!["research-run", "context"],
        vec![
            "research-run",
            "handoff",
            "create",
            "--id",
            "handoff-one",
            "--generated-at",
            "2026-07-18T20:01:00Z",
        ],
    ] {
        run_at(root, &args).expect("expanded command");
    }
}

fn run_at(root: &std::path::Path, args: &[&str]) -> crate::Result<()> {
    inject_current_directory(root);
    execute(Cli::try_parse_from(args).expect("parse command"))
}
