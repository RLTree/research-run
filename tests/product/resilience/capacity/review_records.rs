use super::*;

pub(super) fn review_capacity() {
    let root = initialize("review-capacity");
    seed_claims(&root.0, RECORD_CAPACITY);
    let signer = crate::review_test_signing::ReviewSigner::for_project(&root.0);
    for index in 0..RECORD_CAPACITY - 1 {
        write_record(
            &root.0,
            "reviews",
            &format!("review-{index}"),
            signer.record(
                &format!("review-{index}"),
                &format!("claim-{index}"),
                "limited",
            ),
        );
    }
    crate::review_test_signing::add_signed_review(
        &root.0,
        "review-capacity",
        "claim-9999",
        "limited",
        "Rationale",
        "Reviewer",
    );
    let overflow = crate::review_test_signing::prepare_signed_review(
        &root.0,
        "review-overflow",
        "claim-0",
        "limited",
        "Rationale",
        "Reviewer",
    );
    assert_capacity_error(run(
        &root.0,
        &[
            "review",
            "add",
            "--request",
            &overflow.request.to_string_lossy(),
            "--signature",
            &overflow.signature.to_string_lossy(),
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
