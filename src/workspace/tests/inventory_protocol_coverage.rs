use super::*;

#[test]
fn inventory_retry_propagates_both_snapshot_and_identity_reads() {
    let root = temporary();
    fs::write(root.join("note.md"), b"note").expect("note");
    Workspace::initialize(&root, "Inventory identity reads").expect("initialize");
    let plan = Workspace::plan_retrofit(
        &root,
        "Inventory identity reads",
        "inventory",
        "2026-07-18T20:00:00Z",
        None,
    )
    .expect("plan");
    Workspace::apply_inventory_plan(&root, plan.clone()).expect("seed inventory");
    let inventory_failures = faults()
        .into_iter()
        .filter(|fault| {
            inject_storage_failure(fault);
            Workspace::apply_inventory_plan(&root, plan.clone())
                .is_err_and(|error| error.to_string().contains("inventories/inventory.json"))
        })
        .count();
    assert!(inventory_failures >= 2);
    fs::remove_dir_all(root).expect("remove fixture");
}

fn faults() -> [&'static str; 16] {
    [
        "inspect record",
        "inspect record#2",
        "inspect record#3",
        "inspect record#4",
        "inspect record#5",
        "inspect record#6",
        "inspect record#7",
        "inspect record#8",
        "inspect record#9",
        "inspect record#10",
        "inspect record#11",
        "inspect record#12",
        "inspect record#13",
        "inspect record#14",
        "inspect record#15",
        "inspect record#16",
    ]
}
