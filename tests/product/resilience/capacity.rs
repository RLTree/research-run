use super::*;

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
    for index in 0..RECORD_CAPACITY {
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
    assert_capacity_error(add_source(&root.0, "source-overflow", None));
}

fn claim_capacity() {
    let root = initialize("claim-capacity");
    seed_claims(&root.0);
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
    for index in 0..RECORD_CAPACITY {
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
    for index in 0..RECORD_CAPACITY {
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

fn review_capacity() {
    let root = initialize("review-capacity");
    seed_claims(&root.0);
    for index in 0..RECORD_CAPACITY {
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

fn seed_claims(root: &Path) {
    for index in 0..RECORD_CAPACITY {
        let id = format!("claim-{index}");
        write_record(root, "claims", &id, claim_value(&id));
    }
}

fn claim_value(id: &str) -> serde_json::Value {
    json!({
        "schema_version": 1, "kind": "claim", "id": id, "text": "Claim", "scope": "Scope",
        "owner": "Owner", "authorship": "human"
    })
}

fn write_record(root: &Path, directory: &str, id: &str, value: serde_json::Value) {
    let path = root
        .join(".research-run")
        .join(directory)
        .join(format!("{id}.json"));
    fs::write(path, serde_json::to_vec(&value).expect("record JSON")).expect("write record");
}

fn assert_capacity_error(output: Output) {
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("record budget"));
}
