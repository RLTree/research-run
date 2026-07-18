use std::error::Error as _;
use std::io;
use std::path::PathBuf;

use research_run::Error;

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
