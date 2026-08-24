use std::fs;

use serde_json::{Value, json};

use super::handoff_journey::create_handoff_fixture;
use super::researcher_journey::{TempDir, cli, succeeds};

#[test]
fn handoff_inspection_rejects_unknown_versions_and_forged_ceiling() {
    let temporary = TempDir::new("handoff-defense");
    let path = temporary.0.join("bad.json");
    let fixture = create_handoff_fixture(&temporary);
    let mut invalid: Value =
        serde_json::from_slice(&fs::read(fixture).expect("read handoff fixture")).unwrap();
    invalid["schema_version"] = json!(99);
    fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(!inspect(&temporary, &path, false).status.success());

    let hostile = "visible\u{1b}]52;forged\u{7}";
    invalid["schema_version"] = json!(1);
    invalid
        .as_object_mut()
        .expect("handoff object")
        .remove("validation");
    invalid["context"]["claim_ceiling"] = json!(hostile);
    fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let forged = inspect(&temporary, &path, true);
    assert!(!forged.status.success());
    assert!(!forged.stdout.contains(&0x1b));
    assert!(!forged.stderr.contains(&0x1b));

    invalid["context"]["claim_ceiling"] = json!(research_run::workspace::CLAIM_CEILING);
    invalid["context"]["project_name"] = json!(hostile);
    fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let escaped = succeeds(
        &temporary.0,
        &[
            "handoff",
            "inspect",
            "--input",
            path.to_str().unwrap(),
            "--human",
        ],
    );
    assert!(!escaped.stdout.contains(&0x1b));
    assert!(!escaped.stdout.contains(&0x07));

    fs::write(&path, b"{").expect("malformed handoff");
    assert!(!inspect(&temporary, &path, false).status.success());
    fs::remove_file(&path).expect("remove handoff");
    assert!(!inspect(&temporary, &path, false).status.success());
}

fn inspect(temporary: &TempDir, path: &std::path::Path, human: bool) -> std::process::Output {
    let mut args = vec!["handoff", "inspect", "--input", path.to_str().unwrap()];
    if human {
        args.push("--human");
    }
    cli(&temporary.0, &args)
}
