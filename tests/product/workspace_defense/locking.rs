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
    let binary = std::env::var("CARGO_BIN_EXE_research-run").expect("binary path");
    let args = |id: &str| {
        vec![
            "review".to_owned(),
            "add".to_owned(),
            "--id".to_owned(),
            id.to_owned(),
            "--claim".to_owned(),
            "claim-one".to_owned(),
            "--decision".to_owned(),
            "limited".to_owned(),
            "--rationale".to_owned(),
            "Bounded support".to_owned(),
            "--reviewer".to_owned(),
            "Researcher".to_owned(),
        ]
    };
    let mut first = Command::new(&binary)
        .current_dir(&project.0)
        .args(args("review-one"))
        .spawn()
        .expect("first review");
    let mut second = Command::new(&binary)
        .current_dir(&project.0)
        .args(args("review-two"))
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
