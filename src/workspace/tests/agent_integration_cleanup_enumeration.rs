#[cfg(unix)]
use std::fs;

#[cfg(unix)]
use super::super::super::tests::temporary;
#[cfg(unix)]
use super::super::completion_io::{list_names, validate_name};
#[cfg(unix)]
use super::super::directories::open_directory;
#[cfg(unix)]
use crate::Error;

#[cfg(unix)]
const COMPLETION_NAMES: [&str; 5] = ["version", "exchange", "original", "reviewed", "completion"];

#[cfg(unix)]
#[test]
fn completion_scan_rejects_children_outside_its_closed_set() {
    let root = temporary();
    let transaction = root.join(".AGENTS.md.87.1.txn");
    fs::create_dir(&transaction).unwrap();
    for name in [
        "version",
        "exchange",
        "original",
        "reviewed",
        "completion",
        "unexpected",
    ] {
        fs::write(transaction.join(name), b"witness").unwrap();
    }
    let directory = open_directory(&transaction).unwrap();

    let result = list_names(&directory, &transaction, &COMPLETION_NAMES);
    let message = result.expect_err("a sixth completion child must fail closed");
    assert!(
        message
            .to_string()
            .contains("completion transaction exceeds its closed 5-child budget")
            || message
                .to_string()
                .contains("unexpected completion transaction child"),
        "{message:?}"
    );
    assert!(transaction.join("unexpected").exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn completion_scan_rejects_an_unexpected_child_below_budget() {
    let root = temporary();
    let transaction = root.join(".AGENTS.md.88.1.txn");
    fs::create_dir(&transaction).unwrap();
    let unexpected = transaction.join("unexpected");
    fs::write(&unexpected, b"witness").unwrap();
    let directory = open_directory(&transaction).unwrap();

    let result = list_names(&directory, &transaction, &COMPLETION_NAMES);
    assert!(
        matches!(
            result,
            Err(Error::Conflict(ref message))
                if message.contains("unexpected completion transaction child")
        ),
        "{result:?}"
    );
    assert!(unexpected.exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn completion_name_validation_rejects_non_utf8_bytes() {
    use std::os::unix::ffi::OsStringExt;

    let invalid_name = std::ffi::OsString::from_vec(vec![b'w', 0xff]);
    let result = validate_name(&invalid_name, 0, &COMPLETION_NAMES);
    assert!(
        matches!(
            result,
            Err(Error::Conflict(ref message))
                if message.contains("completion transaction contains a non-UTF-8 name")
        ),
        "{result:?}"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn completion_scan_rejects_a_non_utf8_child_without_removing_it() {
    use std::os::unix::ffi::OsStringExt;

    let root = temporary();
    let transaction = root.join(".AGENTS.md.89.1.txn");
    fs::create_dir(&transaction).unwrap();
    let invalid_name = std::ffi::OsString::from_vec(vec![b'w', 0xff]);
    let invalid_child = transaction.join(&invalid_name);
    fs::write(&invalid_child, b"witness").unwrap();
    let directory = open_directory(&transaction).unwrap();

    let result = list_names(&directory, &transaction, &COMPLETION_NAMES);
    assert!(
        matches!(
            result,
            Err(Error::Conflict(ref message))
                if message.contains("completion transaction contains a non-UTF-8 name")
        ),
        "{result:?}"
    );
    assert!(invalid_child.exists());
    fs::remove_dir_all(root).unwrap();
}
