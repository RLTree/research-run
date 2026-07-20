use super::{TempDir, run};

#[test]
fn installed_inventory_reader_rejects_semantically_invalid_plan() {
    let root = TempDir::new("semantic-inventory-plan");
    std::fs::write(root.0.join("note.md"), b"note").expect("note");
    let plan = run(
        &root.0,
        &[
            "retrofit",
            "plan",
            ".",
            "--name",
            "Semantic plan",
            "--id",
            "inventory-one",
            "--observed-at",
            "2026-07-18T20:00:00Z",
            "--without-review-authority",
        ],
        None,
    );
    assert!(plan.status.success());
    let mut value: serde_json::Value = serde_json::from_slice(&plan.stdout).expect("plan JSON");
    value["id"] = serde_json::json!("INVALID");
    let input = root.0.join("invalid-plan.json");
    std::fs::write(&input, serde_json::to_vec(&value).unwrap()).expect("plan input");
    assert!(
        !run(
            &root.0,
            &["retrofit", "apply", ".", "--input", input.to_str().unwrap()],
            None,
        )
        .status
        .success()
    );
}

#[test]
fn installed_retrofit_scan_propagates_both_file_symlink_rechecks() {
    let root = TempDir::new("retrofit-symlink-rechecks");
    std::fs::create_dir(root.0.join("nested")).expect("nested");
    std::fs::write(root.0.join("nested/note.md"), b"note").expect("note");
    for fault in ["material pre-hash path", "material post-hash path"] {
        let output = run(
            &root.0,
            &[
                "retrofit",
                "plan",
                ".",
                "--name",
                "Retrofit fault",
                "--id",
                "inventory-one",
                "--observed-at",
                "2026-07-18T20:00:00Z",
                "--without-review-authority",
            ],
            Some(fault),
        );
        assert!(!output.status.success(), "{fault}");
    }
}
