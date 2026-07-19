use super::*;

pub(super) fn reject_invalid_pending_reviews() {
    let review = initialize("pending-review-missing-claim");
    pending(
        &review.0,
        "reviews",
        "review-one",
        json!({
            "schema_version": 1, "kind": "review", "id": "review-one",
            "claim_id": "claim-missing", "evidence_ids": [], "decision": "limited",
            "rationale": "Rationale", "reviewer": "Reviewer"
        }),
    );
    assert!(
        !run(&review.0, &["recover", "--json"], None)
            .status
            .success()
    );

    let corrupt_reviews = initialize("pending-review-load-failure");
    super::recovery::add_claim(&corrupt_reviews.0);
    fs::write(
        corrupt_reviews.0.join(".research-run/reviews/bad.json"),
        b"not-json",
    )
    .expect("corrupt review");
    pending(
        &corrupt_reviews.0,
        "reviews",
        "review-one",
        json!({
            "schema_version": 1, "kind": "review", "id": "review-one",
            "claim_id": "claim-one", "evidence_ids": [], "decision": "limited",
            "rationale": "Rationale", "reviewer": "Reviewer"
        }),
    );
    assert!(
        !run(&corrupt_reviews.0, &["recover", "--json"], None)
            .status
            .success()
    );

    let review_read_fault = initialize("pending-review-read-fault");
    super::recovery::add_claim(&review_read_fault.0);
    pending(
        &review_read_fault.0,
        "reviews",
        "review-one",
        json!({
            "schema_version": 1, "kind": "review", "id": "review-one",
            "claim_id": "claim-one", "evidence_ids": [], "decision": "limited",
            "rationale": "Rationale", "reviewer": "Reviewer"
        }),
    );
    assert!(
        !run(
            &review_read_fault.0,
            &["recover", "--json"],
            Some("read record directory#6")
        )
        .status
        .success()
    );

    reject_mismatched_pending_review_graph();
    reject_malformed_pending_review_authority();
}

fn reject_malformed_pending_review_authority() {
    for (label, target_id, record_id, decision) in [
        (
            "pending-supported-without-evidence",
            "review-one",
            "review-one",
            "supported",
        ),
        (
            "pending-review-filename-mismatch",
            "review-one",
            "review-other",
            "limited",
        ),
    ] {
        let root = initialize(label);
        super::recovery::add_claim(&root.0);
        pending(
            &root.0,
            "reviews",
            target_id,
            json!({
                "schema_version": 1, "kind": "review", "id": record_id,
                "claim_id": "claim-one", "evidence_ids": [], "decision": decision,
                "rationale": "Rationale", "reviewer": "Reviewer"
            }),
        );
        assert!(!run(&root.0, &["recover", "--json"], None).status.success());
    }
}

fn reject_mismatched_pending_review_graph() {
    let mismatched_graph = initialize("pending-review-graph-mismatch");
    super::recovery::add_claim(&mismatched_graph.0);
    pending(
        &mismatched_graph.0,
        "reviews",
        "review-one",
        json!({
            "schema_version": 1, "kind": "review", "id": "review-one",
            "claim_id": "claim-one", "evidence_ids": ["evidence-missing"],
            "decision": "limited", "rationale": "Rationale", "reviewer": "Reviewer"
        }),
    );
    assert!(
        !run(&mismatched_graph.0, &["recover", "--json"], None)
            .status
            .success()
    );
}

#[test]
fn recovery_plan_propagates_each_storage_transition() {
    let symlink_race = initialize("pending-record-symlink-race");
    assert!(
        !run(
            &symlink_race.0,
            &["recover", "--json"],
            Some("pending record symlink race")
        )
        .status
        .success()
    );

    let preflight = initialize("preflight-target-read");
    let pending_path = pending(
        &preflight.0,
        "sources",
        "source-one",
        pending_source_value("source-one"),
    );
    fs::copy(
        &pending_path,
        preflight.0.join(".research-run/sources/source-one.json"),
    )
    .expect("canonical source");
    assert!(
        !run(
            &preflight.0,
            &["recover", "--json"],
            Some("inspect record#4")
        )
        .status
        .success()
    );

    for fault in [
        "inspect workspace path#3",
        "inspect workspace path#14",
        "inspect record#2",
        "open record",
        "inspect opened record",
        "open record directory for sync#1",
        "open record directory for sync#2",
    ] {
        let root = initialize(fault);
        pending(
            &root.0,
            "sources",
            "source-one",
            pending_source_value("source-one"),
        );
        assert!(
            !run(&root.0, &["recover", "--json"], Some(fault))
                .status
                .success(),
            "recover ignored {fault}"
        );
    }

    let discard = initialize("discard-second-read");
    let pending_path = pending(
        &discard.0,
        "sources",
        "source-one",
        pending_source_value("source-one"),
    );
    fs::copy(
        &pending_path,
        discard.0.join(".research-run/sources/source-one.json"),
    )
    .expect("canonical source");
    assert!(
        !run(&discard.0, &["recover", "--json"], Some("inspect record#5"))
            .status
            .success()
    );
}
