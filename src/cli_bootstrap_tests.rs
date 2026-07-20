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
            false,
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
            false,
        )
        .is_err()
    );
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn lifecycle_and_inventory_bootstrap_reject_unreachable_cli_shapes() {
    let root = std::env::temp_dir().join(format!(
        "research-run-bootstrap-shapes-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&root).expect("fixture");
    assert!(super::lifecycle_commands::initialize(&root, "Missing", None, None, false).is_err());

    let plan = |review_authority_id, review_authority_public_key, without_review_authority| {
        super::inventory_arguments::InventoryCommand::Plan {
            path: root.clone(),
            name: "Project".to_owned(),
            id: "inventory-one".to_owned(),
            observed_at: "2026-07-18T20:00:00Z".to_owned(),
            review_authority_id,
            review_authority_public_key,
            without_review_authority,
        }
    };
    assert!(
        super::inventory_commands::execute(plan(None, None, true), true).is_err(),
        "reconciliation accepted retrofit bootstrap"
    );
    assert!(
        super::inventory_commands::execute(plan(Some("test-human".to_owned()), None, false), false)
            .is_err(),
        "partial authority pair was accepted"
    );
    assert!(
        super::inventory_commands::execute(
            plan(
                Some("test-human".to_owned()),
                Some(root.join("missing.pub").to_string_lossy().into_owned()),
                false,
            ),
            false,
        )
        .is_err(),
        "missing authority key was accepted"
    );
    let invalid = root.join("invalid-inventory.pub");
    fs::write(&invalid, b"not-a-key").expect("invalid key");
    assert!(
        super::inventory_commands::execute(
            plan(
                Some("test-human".to_owned()),
                Some(invalid.to_string_lossy().into_owned()),
                false,
            ),
            false,
        )
        .is_err(),
        "invalid inventory authority key was accepted"
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
            false,
        )
        .is_err()
    );
}
