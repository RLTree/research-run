use std::error::Error as _;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use research_run::Error;

use super::initialize;

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
