use std::fs;

use crate::Error;
use crate::workspace::tests::temporary;

use super::*;

#[test]
fn actual_exchange_failure_has_no_non_atomic_fallback() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    let transaction = root.join("transaction");
    fs::write(&target, b"original").unwrap();
    fs::create_dir(&transaction).unwrap();

    let error = exchange(&target, &transaction).unwrap_err();

    assert!(matches!(error, Error::AmbiguousEffect(_)));
    assert_eq!(fs::read(&target).unwrap(), b"original");
    assert!(!transaction.join("exchange").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn exchange_rejects_invalid_leaves_and_replaced_directory_anchors() {
    assert!(validated_leaf(Path::new("")).is_err());
    assert!(validated_leaf(Path::new("OTHER.md")).is_err());

    let root = temporary();
    let original_root = root.with_extension("original-root");
    let handle = open_directory(&root).unwrap();
    fs::rename(&root, &original_root).unwrap();
    fs::create_dir(&root).unwrap();

    assert!(matches!(
        verify_anchor(&root, &handle),
        Err(Error::AmbiguousEffect(_))
    ));

    fs::remove_dir(&root).unwrap();
    fs::rename(&original_root, &root).unwrap();
    let file = root.join("not-a-directory");
    fs::write(&file, b"file").unwrap();
    assert!(open_directory(&file).is_err());

    let original_root = root.with_extension("directory-anchor");
    let handle = open_directory(&root).unwrap();
    fs::rename(&root, &original_root).unwrap();
    fs::write(&root, b"replacement file").unwrap();
    assert!(matches!(
        verify_anchor(&root, &handle),
        Err(Error::AmbiguousEffect(_))
    ));
    fs::remove_file(&root).unwrap();
    fs::rename(&original_root, &root).unwrap();

    let device = Path::new("/dev");
    let root_handle = open_directory(&root).unwrap();
    let device_handle = open_directory(device).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        if root_handle.metadata().unwrap().dev() != device_handle.metadata().unwrap().dev() {
            assert!(ensure_same_device(&root, &root_handle, device, &device_handle).is_err());
        }
    }
    fs::remove_dir_all(root).unwrap();
}
