use super::*;
use serde_json::json;

#[test]
fn recovery_rejects_history_cycles_before_publication() {
    let project = workspace("recovery-cycle");
    let state = project.0.join(".research-run");
    for id in ["record-old", "record-new"] {
        write_json(
            &state.join(format!("knowledge/{id}.json")),
            json!({
                "schema_version": 1, "kind": "knowledge", "id": id,
                "record_type": "observation", "title": id, "body": "",
                "occurred_at": "2026-07-20T00:00:00Z",
                "state": "open", "authorship": "human"
            }),
        );
    }
    write_json(
        &state.join("relationships/revision-forward.json"),
        relationship("revision-forward", "record-new", "record-old"),
    );
    let pending = state.join("relationships/.revision-cycle.json.99.1.tmp");
    write_json(
        &pending,
        relationship("revision-cycle", "record-old", "record-new"),
    );

    let output = cli(&project.0, &["recover", "--json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("acyclic"));
    assert!(pending.exists());
    assert!(!state.join("relationships/revision-cycle.json").exists());
}

#[test]
fn recovery_rejects_forged_inventory_authority() {
    let project = workspace("recovery-inventory-authority");
    let state = project.0.join(".research-run");
    let pending = state.join("inventories/.inventory-forged.json.99.1.tmp");
    write_json(
        &pending,
        json!({
            "schema_version": 1, "kind": "inventory", "id": "inventory-forged",
            "project_name": "Adversarial fixture", "project_id": "another-project",
            "observed_at": "2026-07-20T00:00:00Z",
            "previous_snapshot_id": null, "entries": [], "changes": []
        }),
    );

    let output = cli(&project.0, &["recover", "--json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("inventory identity"));
    assert!(pending.exists());
    assert!(!state.join("inventories/inventory-forged.json").exists());
}

#[test]
fn recovery_rejects_forged_inventory_content_with_valid_identity() {
    let project = workspace("recovery-inventory-content");
    fs::write(project.0.join("observation.md"), b"observed").expect("material");
    let planned = succeeds(
        &project.0,
        &[
            "retrofit",
            "plan",
            ".",
            "--name",
            "Adversarial fixture",
            "--id",
            "inventory-forged-content",
            "--observed-at",
            "2026-07-20T00:00:00Z",
        ],
    );
    let mut inventory: Value = serde_json::from_slice(&planned.stdout).expect("inventory plan");
    inventory["kind"] = Value::String("inventory".to_owned());
    inventory["entries"][0]["sha256"] = Value::String("a".repeat(64));
    let state = project.0.join(".research-run");
    let pending = state.join("inventories/.inventory-forged-content.json.99.1.tmp");
    write_json(&pending, inventory);

    let output = cli(&project.0, &["recover", "--json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("material authority"));
    assert!(pending.exists());
    assert!(
        !state
            .join("inventories/inventory-forged-content.json")
            .exists()
    );
}

#[test]
fn recovery_accepts_initial_inventory_bound_to_current_materials() {
    let project = workspace("recovery-valid-inventory");
    fs::write(project.0.join("observation.md"), b"observed").expect("material");
    let planned = succeeds(
        &project.0,
        &[
            "retrofit",
            "plan",
            ".",
            "--name",
            "Adversarial fixture",
            "--id",
            "inventory-valid",
            "--observed-at",
            "2026-07-20T00:00:00Z",
        ],
    );
    let mut inventory: Value = serde_json::from_slice(&planned.stdout).expect("inventory plan");
    inventory["kind"] = Value::String("inventory".to_owned());
    let state = project.0.join(".research-run");
    let pending = state.join("inventories/.inventory-valid.json.99.1.tmp");
    write_json(&pending, inventory);

    succeeds(&project.0, &["recover", "--json"]);
    assert!(!pending.exists());
    assert!(state.join("inventories/inventory-valid.json").exists());
}

#[test]
fn recovery_rejects_forged_migration_authority() {
    let project = workspace("recovery-migration-authority");
    let state = project.0.join(".research-run");
    let pending = state.join("migrations/.migration-forged.json.99.1.tmp");
    write_json(
        &pending,
        json!({
            "schema_version": 1, "kind": "migration", "id": "migration-forged",
            "project_id": "another-project", "from_format": "research-run-v0.1",
            "to_format": "research-run-v0.1-extended",
            "migrated_at": "2026-07-20T00:00:00Z",
            "authority_files": 1, "authority_bytes": 1,
            "authority_sha256": "a".repeat(64)
        }),
    );

    let output = cli(&project.0, &["recover", "--json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("migration identity"));
    assert!(pending.exists());
    assert!(!state.join("migrations/migration-forged.json").exists());
}

#[test]
fn recovery_rejects_forged_migration_digest_with_valid_identity() {
    let project = workspace("recovery-migration-digest");
    let planned = succeeds(
        &project.0,
        &[
            "migrate",
            "plan",
            ".",
            "--id",
            "migration-forged-digest",
            "--migrated-at",
            "2026-07-20T00:00:00Z",
        ],
    );
    let mut migration: Value = serde_json::from_slice(&planned.stdout).expect("migration plan");
    migration["kind"] = Value::String("migration".to_owned());
    migration["authority_sha256"] = Value::String("a".repeat(64));
    let state = project.0.join(".research-run");
    let pending = state.join("migrations/.migration-forged-digest.json.99.1.tmp");
    write_json(&pending, migration);

    let output = cli(&project.0, &["recover", "--json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("canonical authority"));
    assert!(pending.exists());
    assert!(
        !state
            .join("migrations/migration-forged-digest.json")
            .exists()
    );
}

#[test]
fn recovery_accepts_a_migration_bound_to_current_authority() {
    let project = workspace("recovery-valid-migration");
    let planned = succeeds(
        &project.0,
        &[
            "migrate",
            "plan",
            ".",
            "--id",
            "migration-valid",
            "--migrated-at",
            "2026-07-20T00:00:00Z",
        ],
    );
    let mut migration: Value = serde_json::from_slice(&planned.stdout).expect("migration plan");
    migration["kind"] = Value::String("migration".to_owned());
    let state = project.0.join(".research-run");
    let pending = state.join("migrations/.migration-valid.json.99.1.tmp");
    write_json(&pending, migration);

    succeeds(&project.0, &["recover", "--json"]);
    assert!(!pending.exists());
    assert!(state.join("migrations/migration-valid.json").exists());
}

#[test]
fn invalid_existing_manifest_blocks_every_recovery_effect() {
    let project = workspace("recovery-invalid-manifest");
    let state = project.0.join(".research-run");
    let manifest = state.join("manifest.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    value["declared_roots"] = json!(["outside"]);
    write_json(&manifest, value);
    let pending = state.join("sources/.source-late.json.99.1.tmp");
    write_json(
        &pending,
        json!({
            "schema_version": 1, "kind": "source", "id": "source-late",
            "citation": "Pending source", "locator": "local:pending",
            "provenance": "human", "notes": ""
        }),
    );

    let output = cli(&project.0, &["recover", "--json"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("declared_roots"));
    assert!(pending.exists());
    assert!(!state.join("sources/source-late.json").exists());
}

fn relationship(id: &str, from: &str, to: &str) -> Value {
    json!({
        "schema_version": 1, "kind": "relationship", "id": id,
        "relationship": "revises",
        "from": {"kind": "knowledge", "id": from},
        "to": {"kind": "knowledge", "id": to},
        "rationale": "History", "occurred_at": "2026-07-20T00:01:00Z",
        "authorship": "human"
    })
}

fn write_json(path: &Path, value: Value) {
    fs::write(path, serde_json::to_vec_pretty(&value).unwrap()).expect("write JSON fixture");
}
