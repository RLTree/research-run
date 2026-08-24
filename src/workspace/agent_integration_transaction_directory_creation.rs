#[cfg(unix)]
use std::fs;
use std::path::Path;

#[cfg(unix)]
use crate::Error;
use crate::Result;

#[cfg(unix)]
use super::super::storage::{map_io, sync_directory};
#[cfg(not(unix))]
use super::atomic_exchange::ensure_exchange_platform;

#[cfg(unix)]
pub(in crate::workspace) fn create_transaction_directory(path: &Path) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};

    let parent = path.parent().expect("transaction has a parent");
    let parent_metadata = map_io(
        fs::symlink_metadata(parent),
        "inspect project instruction transaction parent",
        parent,
    )?;
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    map_io(
        builder.create(path),
        "create project instruction transaction",
        path,
    )?;
    (|| {
        let metadata = map_io(
            fs::symlink_metadata(path),
            "inspect project instruction transaction permissions",
            path,
        )?;
        if !transaction_directory_permissions_are_private(
            parent_metadata.mode(),
            parent_metadata.gid(),
            metadata.mode(),
            metadata.gid(),
            metadata.is_dir(),
        ) {
            return Err(Error::Conflict(format!(
                "project instruction transaction permissions or inherited identity are invalid: {}",
                path.display()
            )));
        }
        sync_directory(parent)
    })()
    .map_err(|error| {
        Error::AmbiguousEffect(format!(
            "project instruction transaction may exist at {}; retained for inspection after post-create failure: {error}",
            path.display()
        ))
    })
}

#[cfg(unix)]
pub(in crate::workspace) fn transaction_directory_permissions_are_private(
    parent_mode: u32,
    parent_gid: u32,
    transaction_mode: u32,
    transaction_gid: u32,
    is_directory: bool,
) -> bool {
    if !is_directory || transaction_mode & 0o077 != 0 {
        return false;
    }
    let special = transaction_mode & 0o7000;
    // Linux mkdir inherits S_ISGID from a setgid parent without granting access.
    #[cfg(target_os = "linux")]
    let inherited_setgid =
        special == 0o2000 && parent_mode & 0o2000 != 0 && transaction_gid == parent_gid;
    #[cfg(not(target_os = "linux"))]
    let inherited_setgid = {
        let _ = (parent_mode, parent_gid, transaction_gid);
        false
    };
    special == 0 || inherited_setgid
}

#[cfg(not(unix))]
pub(in crate::workspace) fn create_transaction_directory(_: &Path) -> Result<()> {
    ensure_exchange_platform()
}
