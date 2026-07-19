use super::*;

#[path = "capacity/review_records.rs"]
mod review_records;

use review_records::{
    assert_capacity_error, claim_value, review_capacity, seed_claims, write_record,
};

const RECORD_CAPACITY: usize = 10_000;

#[test]
fn each_write_boundary_rejects_the_first_record_beyond_capacity() {
    source_capacity();
    claim_capacity();
    experiment_capacity();
    evidence_capacity();
    review_capacity();
}

fn source_capacity() {
    let root = initialize("source-capacity");
    for index in 0..RECORD_CAPACITY - 1 {
        write_record(
            &root.0,
            "sources",
            &format!("source-{index}"),
            json!({
                "schema_version": 1, "kind": "source", "id": format!("source-{index}"),
                "citation": "Citation", "locator": "local:source",
                "provenance": "human", "notes": ""
            }),
        );
    }
    assert!(
        add_source(&root.0, "source-capacity", None)
            .status
            .success()
    );
    assert_capacity_error(add_source(&root.0, "source-overflow", None));
}

fn claim_capacity() {
    let root = initialize("claim-capacity");
    seed_claims(&root.0, RECORD_CAPACITY - 1);
    assert!(
        run(
            &root.0,
            &[
                "claim",
                "add",
                "--id",
                "claim-capacity",
                "--text",
                "Claim",
                "--scope",
                "Scope",
                "--owner",
                "Owner",
                "--authorship",
                "human",
            ],
            None,
        )
        .status
        .success()
    );
    assert_capacity_error(run(
        &root.0,
        &[
            "claim",
            "add",
            "--id",
            "claim-overflow",
            "--text",
            "Claim",
            "--scope",
            "Scope",
            "--owner",
            "Owner",
            "--authorship",
            "human",
        ],
        None,
    ));
}

fn experiment_capacity() {
    let root = initialize("experiment-capacity");
    for index in 0..RECORD_CAPACITY - 1 {
        write_record(
            &root.0,
            "experiments",
            &format!("experiment-{index}"),
            json!({
                "schema_version": 1, "kind": "experiment", "id": format!("experiment-{index}"),
                "question": "Question?", "method_ref": "method.md", "observations": ["Observed"],
                "interpretation": "Interpretation", "limitations": ["Limited"],
                "outcome": "inconclusive", "next_move": "Repeat", "artifacts": []
            }),
        );
    }
    assert!(
        run(
            &root.0,
            &[
                "experiment",
                "add",
                "--id",
                "experiment-capacity",
                "--question",
                "Question?",
                "--method-ref",
                "method.md",
                "--observation",
                "Observed",
                "--interpretation",
                "Interpretation",
                "--limitation",
                "Limited",
                "--outcome",
                "inconclusive",
                "--next-move",
                "Repeat",
            ],
            None,
        )
        .status
        .success()
    );
    assert_capacity_error(run(
        &root.0,
        &[
            "experiment",
            "add",
            "--id",
            "experiment-overflow",
            "--question",
            "Question?",
            "--method-ref",
            "method.md",
            "--observation",
            "Observed",
            "--interpretation",
            "Interpretation",
            "--limitation",
            "Limited",
            "--outcome",
            "inconclusive",
            "--next-move",
            "Repeat",
        ],
        None,
    ));
}

fn evidence_capacity() {
    let root = initialize("evidence-capacity");
    write_record(&root.0, "claims", "claim-one", claim_value("claim-one"));
    write_record(
        &root.0,
        "sources",
        "source-one",
        json!({
            "schema_version": 1, "kind": "source", "id": "source-one",
            "citation": "Citation", "locator": "local:source", "provenance": "human", "notes": ""
        }),
    );
    for index in 0..RECORD_CAPACITY - 1 {
        write_record(
            &root.0,
            "evidence",
            &format!("evidence-{index}"),
            json!({
                "schema_version": 1, "kind": "evidence", "id": format!("evidence-{index}"),
                "claim_id": "claim-one", "source_id": "source-one", "experiment_id": null,
                "artifact": null, "stance": "supports", "specific_evidence": "Specific",
                "authorship": "human"
            }),
        );
    }
    assert!(
        run(
            &root.0,
            &[
                "evidence",
                "add",
                "--id",
                "evidence-capacity",
                "--claim",
                "claim-one",
                "--source",
                "source-one",
                "--stance",
                "supports",
                "--specific-evidence",
                "Specific",
                "--authorship",
                "human",
            ],
            None,
        )
        .status
        .success()
    );
    assert_capacity_error(run(
        &root.0,
        &[
            "evidence",
            "add",
            "--id",
            "evidence-overflow",
            "--claim",
            "claim-one",
            "--source",
            "source-one",
            "--stance",
            "supports",
            "--specific-evidence",
            "Specific",
            "--authorship",
            "human",
        ],
        None,
    ));
}
