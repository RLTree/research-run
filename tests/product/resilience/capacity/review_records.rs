use super::*;

pub(super) fn review_capacity() {
    let root = initialize("review-capacity");
    seed_claims(&root.0, RECORD_CAPACITY);
    for index in 0..RECORD_CAPACITY - 1 {
        write_record(
            &root.0,
            "reviews",
            &format!("review-{index}"),
            json!({
                "schema_version": 1, "kind": "review", "id": format!("review-{index}"),
                "claim_id": format!("claim-{index}"), "evidence_ids": [], "decision": "limited",
                "rationale": "Rationale", "reviewer": "Reviewer"
            }),
        );
    }
    assert!(
        run(
            &root.0,
            &[
                "review",
                "add",
                "--id",
                "review-capacity",
                "--claim",
                "claim-9999",
                "--decision",
                "limited",
                "--rationale",
                "Rationale",
                "--reviewer",
                "Reviewer",
            ],
            None,
        )
        .status
        .success()
    );
    assert_capacity_error(run(
        &root.0,
        &[
            "review",
            "add",
            "--id",
            "review-overflow",
            "--claim",
            "claim-0",
            "--decision",
            "limited",
            "--rationale",
            "Rationale",
            "--reviewer",
            "Reviewer",
        ],
        None,
    ));
}

pub(super) fn seed_claims(root: &Path, count: usize) {
    for index in 0..count {
        let id = format!("claim-{index}");
        write_record(root, "claims", &id, claim_value(&id));
    }
}

pub(super) fn claim_value(id: &str) -> serde_json::Value {
    json!({
        "schema_version": 1, "kind": "claim", "id": id, "text": "Claim", "scope": "Scope",
        "owner": "Owner", "authorship": "human"
    })
}

pub(super) fn write_record(root: &Path, directory: &str, id: &str, value: serde_json::Value) {
    let path = root
        .join(".research-run")
        .join(directory)
        .join(format!("{id}.json"));
    fs::write(path, serde_json::to_vec(&value).expect("record JSON")).expect("write record");
}

pub(super) fn assert_capacity_error(output: Output) {
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("record budget"));
}
