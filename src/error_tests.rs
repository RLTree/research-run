use std::error::Error as _;
use std::io;
use std::path::PathBuf;

use super::Error;

#[test]
fn typed_errors_render_without_record_bodies() {
    let io_error = Error::io("read", "record.json", io::Error::other("failure"));
    assert!(io_error.to_string().contains("cannot read record.json"));
    assert!(io_error.source().is_some());
    for error in [
        Error::invalid("record", "bad shape"),
        Error::MalformedJson {
            path: PathBuf::from("record.json"),
        },
        Error::NotFound("missing".to_owned()),
        Error::Conflict("conflict".to_owned()),
        Error::AmbiguousEffect("ambiguous".to_owned()),
        Error::Budget("budget".to_owned()),
    ] {
        assert!(!error.to_string().is_empty());
        assert!(error.source().is_none());
    }
}
