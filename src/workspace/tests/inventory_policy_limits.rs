use crate::domain::{InventoryLimits, InventoryPolicy};

#[test]
fn policy_supports_explicit_totals_above_sixty_four_gibibytes() {
    let mut policy = InventoryPolicy::default();
    policy.limits.max_file_bytes = 64 * 1024 * 1024 * 1024;
    policy.limits.max_total_bytes = 65 * 1024 * 1024 * 1024;
    assert!(policy.canonicalized().is_ok());

    let default = InventoryLimits::default();
    assert_eq!(default.max_entries, 20_000);
    assert_eq!(default.max_file_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(default.max_total_bytes, 8 * 1024 * 1024 * 1024);
}
