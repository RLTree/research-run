use std::error::Error as _;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use research_run::Error;

use super::{TempDir, initialize, run};

#[test]
fn public_errors_render_without_exposing_hidden_state() {
    let cases = [
        (
            Error::io(
                "read record",
                PathBuf::from("safe/record.json"),
                io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
            ),
            "cannot read record safe/record.json: denied",
        ),
        (
            Error::invalid("claim", "missing owner"),
            "invalid claim: missing owner",
        ),
        (
            Error::MalformedJson {
                path: PathBuf::from("safe/record.json"),
            },
            "malformed JSON in safe/record.json",
        ),
        (
            Error::NotFound("record not found".to_owned()),
            "record not found",
        ),
        (
            Error::Conflict("record conflicts".to_owned()),
            "record conflicts",
        ),
        (
            Error::AmbiguousEffect("publication is ambiguous".to_owned()),
            "publication is ambiguous",
        ),
        (
            Error::Budget("record budget exceeded".to_owned()),
            "record budget exceeded",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
        if matches!(error, Error::Io { .. }) {
            assert_eq!(error.source().expect("I/O source").to_string(), "denied");
        } else {
            assert!(error.source().is_none());
        }
    }
}

#[test]
fn structured_input_read_failures_propagate_from_file_and_stdin() {
    let root = initialize("structured-input-faults");
    let input = root.0.join("knowledge.json");
    std::fs::write(
        &input,
        br#"{"schema_version":1,"kind":"knowledge","id":"knowledge-one","record_type":"observation","title":"Observation","body":"Body","occurred_at":"2026-07-18T20:00:00Z","state":"open","authorship":"human"}"#,
    )
    .expect("structured input");
    assert!(
        !run(
            &root.0,
            &["knowledge", "add", "--input", input.to_str().unwrap()],
            Some("read structured file"),
        )
        .status
        .success()
    );
    assert!(
        !run(
            &root.0,
            &["knowledge", "add", "--input", input.to_str().unwrap()],
            Some("structured post-read budget"),
        )
        .status
        .success()
    );
    assert!(
        !run(
            &root.0,
            &["knowledge", "add", "--input", "-"],
            Some("read structured stdin"),
        )
        .status
        .success()
    );
}

#[test]
fn expanded_command_wrappers_propagate_owned_failures() {
    let outside = TempDir::new("expanded-wrapper-faults");
    let knowledge = outside.0.join("knowledge.json");
    std::fs::write(
        &knowledge,
        br#"{"schema_version":1,"kind":"knowledge","id":"knowledge-one","record_type":"observation","title":"Observation","body":"Body","occurred_at":"2026-07-18T20:00:00Z","state":"open","authorship":"human"}"#,
    )
    .expect("knowledge input");
    let relationship = outside.0.join("relationship.json");
    std::fs::write(
        &relationship,
        br#"{"schema_version":1,"kind":"relationship","id":"relationship-one","relationship":"related-to","from":{"kind":"knowledge","id":"knowledge-one"},"to":{"kind":"knowledge","id":"knowledge-two"},"rationale":"Rationale","occurred_at":"2026-07-18T20:00:00Z","authorship":"human"}"#,
    )
    .expect("relationship input");
    for args in [
        vec!["knowledge", "add", "--input", knowledge.to_str().unwrap()],
        vec![
            "relationship",
            "add",
            "--input",
            relationship.to_str().unwrap(),
        ],
    ] {
        assert!(!run(&outside.0, &args, None).status.success(), "{args:?}");
    }
    std::fs::remove_file(&relationship).expect("remove relationship input");
    assert!(
        !run(
            &outside.0,
            &[
                "relationship",
                "add",
                "--input",
                relationship.to_str().unwrap(),
            ],
            None,
        )
        .status
        .success()
    );

    let malformed = outside.0.join("malformed.json");
    std::fs::write(&malformed, b"{").expect("malformed plan");
    for args in [
        vec![
            "retrofit",
            "apply",
            ".",
            "--input",
            malformed.to_str().unwrap(),
        ],
        vec![
            "migrate",
            "apply",
            ".",
            "--input",
            malformed.to_str().unwrap(),
        ],
        vec![
            "migrate",
            "plan",
            ".",
            "--id",
            "migration-one",
            "--migrated-at",
            "2026-07-18T20:00:00Z",
        ],
    ] {
        assert!(!run(&outside.0, &args, None).status.success(), "{args:?}");
    }
}

#[cfg(unix)]
#[test]
fn installed_output_propagates_closed_stdout_and_stderr() {
    let root = initialize("closed-output");
    let binary = std::env::var("CARGO_BIN_EXE_research-run").expect("binary path");
    for command in [vec!["status"], vec!["validate"], vec!["validate", "--json"]] {
        for close_stderr in [false, true] {
            assert_closed_output(&binary, &root.0, &command, close_stderr);
        }
    }

    let invalid = initialize("closed-invalid-output");
    std::fs::write(
        invalid.0.join(".research-run/evidence/invalid.json"),
        br#"{"schema_version":1,"kind":"evidence","id":"invalid","claim_id":"missing","source_id":null,"experiment_id":null,"artifact":null,"stance":"context","specific_evidence":"Specific","authorship":"human"}"#,
    )
    .expect("invalid reference fixture");
    assert_closed_output(&binary, &invalid.0, &["validate"], false);
}

#[cfg(unix)]
fn assert_closed_output(binary: &str, root: &std::path::Path, args: &[&str], close_stderr: bool) {
    use std::net::Shutdown;
    use std::os::fd::OwnedFd;
    use std::os::unix::net::UnixStream;

    let (stdout_reader, stdout_writer) = UnixStream::pair().expect("stdout socket pair");
    stdout_writer
        .shutdown(Shutdown::Write)
        .expect("close stdout writer");
    drop(stdout_reader);
    let mut command = Command::new(binary);
    command
        .current_dir(root)
        .args(args)
        .stdout(Stdio::from(OwnedFd::from(stdout_writer)));
    if close_stderr {
        let (stderr_reader, stderr_writer) = UnixStream::pair().expect("stderr socket pair");
        stderr_writer
            .shutdown(Shutdown::Write)
            .expect("close stderr writer");
        drop(stderr_reader);
        command.stderr(Stdio::from(OwnedFd::from(stderr_writer)));
    } else {
        command.stderr(Stdio::piped());
    }
    let mut child = command.spawn().expect("spawn projection");
    assert!(
        !child.wait().expect("projection result").success(),
        "{binary} {args:?} unexpectedly succeeded with close_stderr={close_stderr}"
    );
}
