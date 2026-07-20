use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn review_authority_bootstrap_rejects_partial_invalid_and_non_utf8_inputs() {
    let root = std::env::temp_dir().join(format!(
        "research-run-bootstrap-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).expect("fixture");
    assert!(
        super::lifecycle_commands::initialize(
            &root,
            "Partial",
            Some("authority".to_owned()),
            None,
        )
        .is_err()
    );
    let invalid = root.join("invalid.pub");
    fs::write(&invalid, b"not-a-key").expect("invalid key");
    assert_invalid_key(&root, &invalid);
    fs::write(&invalid, [0xff]).expect("non-UTF8 key");
    assert_invalid_key(&root, &invalid);
    fs::write(
        &invalid,
        b"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGqNGVOiQVyevMNkyQjVnsutXzLd1cnernrPi1g9rmyQ",
    )
    .expect("valid key");
    let occupied = root.join("occupied");
    fs::write(&occupied, b"not a directory").expect("occupied root");
    assert!(
        super::lifecycle_commands::initialize(
            &occupied,
            "Occupied",
            Some("authority".to_owned()),
            Some(invalid.to_string_lossy().into_owned()),
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

fn assert_invalid_key(root: &std::path::Path, key: &std::path::Path) {
    assert!(
        super::lifecycle_commands::initialize(
            &root.join("invalid"),
            "Invalid",
            Some("authority".to_owned()),
            Some(key.to_string_lossy().into_owned()),
        )
        .is_err()
    );
}
