use super::*;

#[test]
fn concurrent_review_commands_leave_one_authoritative_decision() {
    let project = workspace("concurrent-review");
    succeeds(
        &project.0,
        &[
            "claim",
            "add",
            "--id",
            "claim-one",
            "--text",
            "Claim",
            "--scope",
            "Scope",
            "--owner",
            "Researcher",
            "--authorship",
            "human",
        ],
    );
    let first_review = crate::review_test_signing::prepare_signed_review(
        &project.0,
        "review-one",
        "claim-one",
        "limited",
        "Bounded support",
        "Researcher",
    );
    let second_review = crate::review_test_signing::prepare_signed_review(
        &project.0,
        "review-two",
        "claim-one",
        "limited",
        "Bounded support",
        "Researcher",
    );
    let binary = std::env::var("CARGO_BIN_EXE_research-run").expect("binary path");
    let args = |review: &crate::review_test_signing::SignedReview| {
        vec![
            "review".to_owned(),
            "add".to_owned(),
            "--request".to_owned(),
            review.request.to_string_lossy().into_owned(),
            "--signature".to_owned(),
            review.signature.to_string_lossy().into_owned(),
        ]
    };
    let mut first = Command::new(&binary)
        .current_dir(&project.0)
        .args(args(&first_review))
        .spawn()
        .expect("first review");
    let mut second = Command::new(&binary)
        .current_dir(&project.0)
        .args(args(&second_review))
        .spawn()
        .expect("second review");
    let first_status = first.wait().expect("first status");
    let second_status = second.wait().expect("second status");
    assert_ne!(first_status.success(), second_status.success());
    succeeds(&project.0, &["validate", "--json"]);
    assert_eq!(
        fs::read_dir(project.0.join(".research-run/reviews"))
            .expect("reviews")
            .count(),
        1
    );
}
