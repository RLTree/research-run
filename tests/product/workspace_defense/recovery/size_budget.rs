use super::*;
use serde_json::json;

const MAX_RECORD_BYTES: usize = 1_048_576;

#[test]
fn recovery_rejects_canonical_expansion_before_publication() {
    let project = workspace("recovery-canonical-size");
    let state = project.0.join(".research-run");
    let pending = state.join("experiments/.experiment-large.json.99.1.tmp");
    let target = state.join("experiments/experiment-large.json");
    let value = json!({
        "schema_version": 1,
        "kind": "experiment",
        "id": "experiment-large",
        "question": "Does canonical recovery stay bounded?",
        "method_ref": "protocol.md",
        "observations": vec!["x".repeat(4_087); 256],
        "interpretation": "The compact input expands under pretty serialization.",
        "limitations": ["Synthetic boundary fixture"],
        "outcome": "inconclusive",
        "next_move": "Reject before publication",
        "artifacts": []
    });
    let compact = serde_json::to_vec(&value).expect("compact JSON");
    let canonical_len = serde_json::to_vec_pretty(&value)
        .expect("canonical JSON")
        .len()
        + 1;
    assert!(
        compact.len() <= MAX_RECORD_BYTES,
        "compact={}",
        compact.len()
    );
    assert!(
        canonical_len > MAX_RECORD_BYTES,
        "canonical={canonical_len}"
    );
    fs::write(&pending, compact).expect("pending record");

    let output = cli(&project.0, &["recover", "--json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("canonical recovery record"));
    assert!(pending.exists());
    assert!(!target.exists());
}
