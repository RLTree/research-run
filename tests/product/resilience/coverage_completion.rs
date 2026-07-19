use std::fs;
use std::process::Stdio;

use serde_json::{Value, json};

use super::{TempDir, initialize, run};

#[test]
fn installed_structured_commands_cover_parse_history_and_output_boundaries() {
    let root = initialize("structured-completion");
    let first = root.0.join("first.json");
    let second = root.0.join("second.json");
    write_json(&first, knowledge("knowledge-one"));
    write_json(&second, knowledge("knowledge-two"));
    assert!(add(&root, "knowledge", &first).status.success());
    assert!(add(&root, "knowledge", &second).status.success());

    let relationship = root.0.join("relationship.json");
    fs::write(&relationship, b"{").expect("malformed relationship");
    assert!(!add(&root, "relationship", &relationship).status.success());
    write_json(&relationship, supersession());
    assert!(add(&root, "relationship", &relationship).status.success());

    let third = root.0.join("third.json");
    write_json(&third, knowledge("knowledge-three"));
    assert_closed_stdout(&root, &third);
}

#[test]
fn installed_retrofit_covers_scan_ordering_and_optional_legacy_state() {
    let project = TempDir::new("inventory-completion");
    fs::write(project.0.join("note.md"), b"one").expect("material");
    assert!(
        !run(
            &project.0,
            &retrofit_plan("inventory-fault", "2026-07-18T20:00:00Z"),
            Some("material path UTF-8"),
        )
        .status
        .success()
    );
    apply_plan(
        &project,
        "retrofit",
        "inventory-one",
        "2026-07-18T20:00:00Z",
    );
    fs::write(project.0.join("note.md"), b"two").expect("change one");
    apply_plan(
        &project,
        "reconcile",
        "inventory-two",
        "2026-07-18T20:01:00Z",
    );
    fs::write(project.0.join("note.md"), b"three").expect("change two");
    apply_plan(
        &project,
        "reconcile",
        "inventory-three",
        "2026-07-18T20:02:00Z",
    );

    for directory in ["inventories", "knowledge", "relationships", "migrations"] {
        fs::remove_dir_all(project.0.join(".research-run").join(directory))
            .expect("remove optional legacy directory");
    }
    assert!(run(&project.0, &["status"], None).status.success());
}

fn add(root: &TempDir, kind: &str, input: &std::path::Path) -> std::process::Output {
    run(
        &root.0,
        &[kind, "add", "--input", input.to_str().unwrap()],
        None,
    )
}

fn assert_closed_stdout(root: &TempDir, input: &std::path::Path) {
    use std::net::Shutdown;
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;

    let binary = std::env::var("CARGO_BIN_EXE_research-run").expect("binary path");
    let (reader, writer) = UnixStream::pair().expect("stdout socket pair");
    writer
        .shutdown(Shutdown::Write)
        .expect("close stdout writer");
    drop(reader);
    let status = std::process::Command::new(binary)
        .current_dir(&root.0)
        .args(["knowledge", "add", "--input", input.to_str().unwrap()])
        .stdout(Stdio::from(OwnedFd::from(writer)))
        .status()
        .expect("closed stdout command");
    assert!(!status.success());
}

fn apply_plan(root: &TempDir, command: &str, id: &str, observed_at: &str) {
    let args = if command == "retrofit" {
        retrofit_plan(id, observed_at)
    } else {
        reconcile_plan(id, observed_at)
    };
    let planned = run(&root.0, &args, None);
    assert!(planned.status.success(), "{command} plan");
    let input = root.0.with_extension(format!("{id}.json"));
    fs::write(&input, planned.stdout).expect("plan file");
    let applied = run(
        &root.0,
        &[command, "apply", ".", "--input", input.to_str().unwrap()],
        None,
    );
    assert!(applied.status.success(), "{command} apply");
    fs::remove_file(input).expect("remove plan file");
}

fn retrofit_plan<'a>(id: &'a str, observed_at: &'a str) -> Vec<&'a str> {
    vec![
        "retrofit",
        "plan",
        ".",
        "--name",
        "Coverage",
        "--id",
        id,
        "--observed-at",
        observed_at,
    ]
}

fn reconcile_plan<'a>(id: &'a str, observed_at: &'a str) -> Vec<&'a str> {
    let mut args = retrofit_plan(id, observed_at);
    args[0] = "reconcile";
    args
}

fn write_json(path: &std::path::Path, value: Value) {
    fs::write(path, serde_json::to_vec(&value).unwrap()).expect("JSON fixture");
}

fn knowledge(id: &str) -> Value {
    json!({"schema_version":1,"kind":"knowledge","id":id,"record_type":"observation","title":"Observation","body":"Body","occurred_at":"2026-07-18T20:00:00Z","state":"open","authorship":"human"})
}

fn supersession() -> Value {
    json!({"schema_version":1,"kind":"relationship","id":"supersession-one","relationship":"supersedes","from":{"kind":"knowledge","id":"knowledge-two"},"to":{"kind":"knowledge","id":"knowledge-one"},"rationale":"Replacement","occurred_at":"2026-07-18T20:01:00Z","authorship":"human"})
}
