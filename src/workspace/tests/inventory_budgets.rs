use crate::domain::InventoryLimits;
use crate::workspace::inventory_scan::{
    add_inventory_bytes, add_inventory_bytes_with_limit, enforce_entry_count, enforce_file_count,
};

#[test]
fn inventory_count_and_byte_budgets_accept_the_limit_and_reject_the_next_unit() {
    assert!(enforce_file_count(2_048).is_ok());
    assert!(enforce_file_count(2_049).is_err());
    let limit = 512 * 1_048_576;
    assert_eq!(add_inventory_bytes(0, limit).expect("exact limit"), limit);
    assert!(add_inventory_bytes(limit, 1).is_err());
    assert!(add_inventory_bytes(u64::MAX, 1).is_err());
    let default = InventoryLimits::default();
    assert_eq!(
        add_inventory_bytes_with_limit(0, limit + 1, default.max_total_bytes)
            .expect("old aggregate limit plus one is within default"),
        limit + 1
    );
    assert!(enforce_entry_count(20_000, default.max_entries).is_ok());
    assert!(enforce_entry_count(20_001, default.max_entries).is_err());
}
