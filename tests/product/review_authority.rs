use research_run::domain::{Assessment, CanonicalRecord, ReviewDecision};

#[test]
fn each_review_authority_field_fails_independently() {
    let review = ReviewDecision {
        schema_version: 1,
        kind: "review".to_owned(),
        id: "review-one".to_owned(),
        claim_id: "claim-one".to_owned(),
        evidence_ids: Vec::new(),
        decision: Assessment::Limited,
        rationale: "Rationale".to_owned(),
        reviewer: "Reviewer".to_owned(),
    };
    for candidate in [
        ReviewDecision {
            schema_version: 2,
            ..review.clone()
        },
        ReviewDecision {
            kind: "wrong".to_owned(),
            ..review.clone()
        },
        ReviewDecision {
            id: "Invalid".to_owned(),
            ..review.clone()
        },
        ReviewDecision {
            claim_id: "Invalid".to_owned(),
            ..review.clone()
        },
        ReviewDecision {
            evidence_ids: vec!["Invalid".to_owned()],
            ..review.clone()
        },
        ReviewDecision {
            evidence_ids: vec!["evidence-two".to_owned(), "evidence-one".to_owned()],
            ..review.clone()
        },
        ReviewDecision {
            evidence_ids: (0..257)
                .map(|index| format!("evidence-{index:03}"))
                .collect(),
            ..review.clone()
        },
        ReviewDecision {
            decision: Assessment::Supported,
            ..review.clone()
        },
        ReviewDecision {
            rationale: String::new(),
            ..review.clone()
        },
        ReviewDecision {
            reviewer: String::new(),
            ..review.clone()
        },
    ] {
        assert!(candidate.validate().is_err());
    }
}
