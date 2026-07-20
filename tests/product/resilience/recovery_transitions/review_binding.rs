use super::*;

#[test]
fn single_pending_review_recovers_without_false_duplicate() {
    let root = initialize("single-pending-review");
    super::super::recovery::add_claim(&root.0);
    crate::review_test_signing::add_signed_review(
        &root.0,
        "review-one",
        "claim-one",
        "limited",
        "Rationale",
        "Reviewer",
    );
    let reviews = root.0.join(".research-run/reviews");
    fs::rename(
        reviews.join("review-one.json"),
        reviews.join(".review-one.json.9.7.tmp"),
    )
    .expect("pending review");
    assert!(run(&root.0, &["recover", "--json"], None).status.success());
}
