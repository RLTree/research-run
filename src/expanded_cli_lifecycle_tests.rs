use std::fs;

use clap::Parser;
use serde_json::json;

use super::{Cli, execute, inject_current_directory, inject_current_directory_failure};
use crate::workspace::Workspace;

#[test]
fn structured_wrappers_cover_success_and_failure() {
    let root = temporary("wrappers");
    Workspace::initialize(&root, "Wrapper routes").expect("initialize");
    let knowledge = root.join("knowledge.json");
    write_json(&knowledge, knowledge_value("knowledge-one", "Body"));
    run_at(
        &root,
        &[
            "research-run",
            "knowledge",
            "add",
            "--input",
            text(&knowledge),
        ],
    )
    .expect("knowledge success");
    write_json(
        &knowledge,
        knowledge_value("knowledge-one", "Conflicting body"),
    );
    assert!(
        run_at(
            &root,
            &[
                "research-run",
                "knowledge",
                "add",
                "--input",
                text(&knowledge)
            ]
        )
        .is_err()
    );
    assert_malformed_knowledge_input(&root, &knowledge);
    let second = root.join("knowledge-two.json");
    write_json(&second, knowledge_value("knowledge-two", "Second body"));
    run_at(
        &root,
        &["research-run", "knowledge", "add", "--input", text(&second)],
    )
    .expect("second knowledge");

    let relationship = root.join("relationship.json");
    write_json(&relationship, relationship_value("knowledge-two"));
    run_at(
        &root,
        &[
            "research-run",
            "relationship",
            "add",
            "--input",
            text(&relationship),
        ],
    )
    .expect("relationship success");
    assert_structured_discovery_precedes_input("knowledge");
    assert_structured_discovery_precedes_input("relationship");
    write_json(&relationship, relationship_value("missing"));
    assert!(
        run_at(
            &root,
            &[
                "research-run",
                "relationship",
                "add",
                "--input",
                text(&relationship)
            ],
        )
        .is_err()
    );
    fs::write(&relationship, b"{").expect("malformed relationship");
    assert_relationship_input_failure(&root, &relationship);

    fs::remove_dir_all(root).expect("remove fixture");
}

fn assert_malformed_knowledge_input(root: &std::path::Path, input: &std::path::Path) {
    fs::write(input, b"{").expect("malformed knowledge");
    assert!(
        run_at(
            root,
            &["research-run", "knowledge", "add", "--input", text(input),],
        )
        .is_err()
    );
}

fn assert_relationship_input_failure(root: &std::path::Path, input: &std::path::Path) {
    assert!(
        run_at(
            root,
            &[
                "research-run",
                "relationship",
                "add",
                "--input",
                text(input),
            ],
        )
        .is_err()
    );
}

fn assert_structured_discovery_precedes_input(kind: &str) {
    inject_current_directory_failure();
    let command = Cli::try_parse_from(["research-run", kind, "add", "--input", "missing.json"])
        .expect("parse structured command");
    assert!(matches!(
        execute(command),
        Err(crate::Error::Io {
            action: "read current directory",
            ..
        })
    ));
}

#[test]
fn handoff_wrapper_covers_success_and_failure() {
    let root = temporary("handoff-wrapper");
    let workspace = Workspace::initialize(&root, "Handoff wrapper").expect("initialize");
    let handoff = root.join("handoff.json");
    let bundle = workspace
        .handoff("handoff-one", "2026-07-18T20:01:00Z", None, 10)
        .expect("handoff");
    write_json(&handoff, serde_json::to_value(bundle).unwrap());
    run_at(
        &root,
        &[
            "research-run",
            "handoff",
            "inspect",
            "--input",
            text(&handoff),
        ],
    )
    .expect("inspect success");
    fs::write(&handoff, b"{").expect("malformed handoff");
    assert!(
        run_at(
            &root,
            &[
                "research-run",
                "handoff",
                "inspect",
                "--input",
                text(&handoff)
            ]
        )
        .is_err()
    );

    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn migration_wrapper_covers_success_and_failure() {
    let root = temporary("migration-wrapper");
    Workspace::initialize(&root, "Migration wrapper").expect("initialize");
    let migration = Workspace::plan_migration(&root, "migration-one", "2026-07-18T20:02:00Z")
        .expect("migration plan");
    let input = temporary("migration-input").join("migration.json");
    write_json(&input, serde_json::to_value(migration).unwrap());
    run_at(
        &root,
        &[
            "research-run",
            "migrate",
            "plan",
            text(&root),
            "--id",
            "migration-cli",
            "--migrated-at",
            "2026-07-18T20:02:00Z",
        ],
    )
    .expect("migration plan command");
    run_at(
        &root,
        &[
            "research-run",
            "migrate",
            "apply",
            text(&root),
            "--input",
            text(&input),
        ],
    )
    .expect("migration apply command");
    fs::write(&input, b"{").expect("malformed migration");
    assert!(
        run_at(
            &root,
            &[
                "research-run",
                "migrate",
                "apply",
                text(&root),
                "--input",
                text(&input)
            ]
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
    fs::remove_dir_all(input.parent().unwrap()).expect("remove input fixture");
}

fn run_at(root: &std::path::Path, args: &[&str]) -> crate::Result<()> {
    inject_current_directory(root);
    execute(Cli::try_parse_from(args).expect("parse command"))
}

fn temporary(label: &str) -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("research-run-cli-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).expect("fixture root");
    root
}

fn text(path: &std::path::Path) -> &str {
    path.to_str().expect("UTF-8 path")
}

fn write_json(path: &std::path::Path, value: serde_json::Value) {
    fs::write(path, serde_json::to_vec(&value).unwrap()).expect("write JSON fixture");
}

fn knowledge_value(id: &str, body: &str) -> serde_json::Value {
    json!({"schema_version":1,"kind":"knowledge","id":id,"record_type":"observation","title":"Observation","body":body,"occurred_at":"2026-07-18T20:00:00Z","state":"open","authorship":"human"})
}

fn relationship_value(to: &str) -> serde_json::Value {
    json!({"schema_version":1,"kind":"relationship","id":format!("relationship-{to}"),"relationship":"related-to","from":{"kind":"knowledge","id":"knowledge-one"},"to":{"kind":"knowledge","id":to},"rationale":"Rationale","occurred_at":"2026-07-18T20:00:00Z","authorship":"human"})
}
