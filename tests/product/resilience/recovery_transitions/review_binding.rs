use super::*;

#[test]
fn single_pending_review_recovers_without_false_duplicate() {
    let root = initialize("single-pending-review");
    super::super::recovery::add_claim(&root.0);
    assert!(
        run(
            &root.0,
            &[
                "review",
                "add",
                "--id",
                "review-one",
                "--claim",
                "claim-one",
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
    let reviews = root.0.join(".research-run/reviews");
    fs::rename(
        reviews.join("review-one.json"),
        reviews.join(".review-one.json.9.7.tmp"),
    )
    .expect("pending review");
    assert!(run(&root.0, &["recover", "--json"], None).status.success());
}
