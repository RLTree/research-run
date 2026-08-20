use std::fs;
use std::path::Path;

use crate::domain::AgentIntegrationOperation;
use crate::{Error, Result};

use super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::path_safety::{ensure_no_pending_effect, reject_symlink_chain};
use super::pending_cleanup::PendingCleanup;
use super::publication::{pending_path, write_pending};
use super::storage::{map_io, read_bounded_with_limit, sync_directory};

pub(super) fn publish_instruction(
    target: &Path,
    content: &[u8],
    operation: AgentIntegrationOperation,
) -> Result<bool> {
    if operation == AgentIntegrationOperation::NoOp {
        return Ok(false);
    }
    reject_symlink_chain(target)?;
    ensure_no_pending_effect(target)?;
    let temporary = pending_path(target);
    let mut cleanup = PendingCleanup::new(temporary.clone());
    if let Err(error) = write_pending(&temporary, content) {
        return Err(cleanup.after_failure(error));
    }
    if let Err(error) = copy_permissions(target, &temporary, operation) {
        return Err(cleanup.after_failure(error));
    }
    let publication = if operation == AgentIntegrationOperation::Create {
        create(target, &temporary, content)
    } else {
        replace(target, &temporary).map(|()| true)
    };
    let changed = match publication {
        Ok(changed) => changed,
        Err(error) => return Err(cleanup.after_failure(error)),
    };
    let parent = target.parent().expect("root instruction has a parent");
    sync_directory(parent).map_err(|error| {
        Error::AmbiguousEffect(format!(
            "project instructions may be installed at {}; directory sync failed: {error}",
            target.display()
        ))
    })?;
    cleanup.remove()?;
    Ok(changed)
}

fn copy_permissions(
    target: &Path,
    temporary: &Path,
    operation: AgentIntegrationOperation,
) -> Result<()> {
    if operation != AgentIntegrationOperation::Append {
        return Ok(());
    }
    let metadata = map_io(
        fs::metadata(target),
        "inspect project instruction permissions",
        target,
    )?;
    map_io(
        fs::set_permissions(temporary, metadata.permissions()),
        "preserve project instruction permissions",
        temporary,
    )
}

fn create(target: &Path, temporary: &Path, content: &[u8]) -> Result<bool> {
    let publication = if super::injected_storage_failure("create project instructions") {
        Err(std::io::Error::other("injected storage failure"))
    } else {
        fs::hard_link(temporary, target)
    };
    match publication {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_bounded_with_limit(target, MAX_INSTRUCTION_BYTES)? == content {
                Ok(false)
            } else {
                Err(Error::Conflict(format!(
                    "project instruction file appeared during apply: {}",
                    target.display()
                )))
            }
        }
        Err(error) => Err(Error::io("create project instructions", target, error)),
    }
}

fn replace(target: &Path, temporary: &Path) -> Result<()> {
    map_io(
        fs::rename(temporary, target),
        "replace project instructions",
        target,
    )
}
