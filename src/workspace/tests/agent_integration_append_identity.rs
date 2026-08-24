use std::fs;

use crate::Error;
use crate::domain::AgentIntegrationOperation;

use super::super::super::agent_integration_publication::publish_instruction;
use super::super::temporary;

const TEST_PLAN_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[cfg(unix)]
#[test]
fn append_rejects_a_multiply_linked_source_before_any_effect() {
    use std::os::unix::fs::MetadataExt;

    let root = temporary();
    let target = root.join("AGENTS.md");
    let alias = root.join("AGENTS.alias");
    fs::write(&target, b"original instructions").unwrap();
    fs::hard_link(&target, &alias).unwrap();
    let original_mode = fs::metadata(&target).unwrap().mode();

    let error = publish_instruction(
        &target,
        b"reviewed instructions",
        AgentIntegrationOperation::Append,
        b"original instructions",
        TEST_PLAN_SHA256,
    )
    .expect_err("multiply linked source must be rejected before exchange");

    assert!(matches!(error, Error::Conflict(_)), "{error:?}");
    assert_eq!(fs::read(&target).unwrap(), b"original instructions");
    assert_eq!(fs::read(&alias).unwrap(), b"original instructions");
    assert_eq!(fs::metadata(&target).unwrap().mode(), original_mode);
    assert!(!fs::read_dir(&root).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".AGENTS.md.")
    }));
    fs::remove_dir_all(root).unwrap();
}
