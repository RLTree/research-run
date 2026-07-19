use research_run::domain::{validate_id, validate_workspace_locator};

#[test]
fn identifiers_and_paths_fail_closed() {
    assert!(validate_id("claim-one", "id").is_ok());
    assert!(validate_id(&format!("a{}", "b".repeat(63)), "id").is_ok());
    assert!(validate_id("", "id").is_err());
    assert!(validate_id(&"a".repeat(65), "id").is_err());
    assert!(validate_id("1claim", "id").is_err());
    assert!(validate_id("claim_underscore", "id").is_err());
    assert!(validate_id("../claim", "id").is_err());
    assert!(validate_workspace_locator("").is_err());
    assert!(validate_workspace_locator("artifacts/summary.txt").is_ok());
    assert!(validate_workspace_locator(&"x".repeat(65_537)).is_err());
    assert!(validate_workspace_locator("../secret").is_err());
    assert!(validate_workspace_locator("/tmp/secret").is_err());
}
