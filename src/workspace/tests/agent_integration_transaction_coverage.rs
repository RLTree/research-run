use std::fs;

use crate::Error;
use crate::workspace::tests::temporary;

use super::*;

#[test]
fn transaction_collision_and_malformed_cleanup_remain_explicit() {
    let root = temporary();
    let target = root.join("AGENTS.md");
    let transaction = root.join("transaction");
    fs::write(&target, b"original").unwrap();
    fs::create_dir(&transaction).unwrap();
    assert!(create_transaction(&transaction).is_err());

    fs::create_dir(transaction.join("version")).unwrap();
    for name in ["exchange", "original", "reviewed"] {
        fs::write(transaction.join(name), name.as_bytes()).unwrap();
    }
    let handles = open_exchange_handles(&target, &transaction).unwrap();
    assert!(matches!(
        cleanup_staging_transaction(&target, &transaction, &handles),
        Err(Error::AmbiguousEffect(_))
    ));
    assert!(matches!(
        abort_transaction(
            &target,
            &transaction,
            &handles,
            Error::Conflict("staging".to_owned())
        ),
        Error::AmbiguousEffect(_)
    ));
    fs::remove_dir_all(root).unwrap();
}
