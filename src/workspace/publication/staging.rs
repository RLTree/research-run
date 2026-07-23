use std::path::Path;

use serde::Serialize;

use crate::domain::CanonicalRecord;
use crate::{Error, Result};

use super::{canonical_json_bytes, pending_path, publication_needed, write_pending};
use crate::workspace::pending_cleanup::PendingCleanup;
use crate::workspace::storage::sync_directory;
use crate::workspace::{MAX_RECORD_BYTES, Workspace, record_byte_limit};

impl Workspace {
    pub(in crate::workspace) fn stage_record_for_recovery<T: CanonicalRecord>(
        &self,
        directory: &str,
        record: &T,
    ) -> Result<bool> {
        let target = self
            .state
            .join(directory)
            .join(format!("{}.json", record.id()));
        let content = canonical_json_bytes(record);
        self.stage_bytes_for_recovery(&target, &content, record_byte_limit(directory))
    }

    pub(in crate::workspace) fn stage_value_for_recovery(
        &self,
        target: &Path,
        value: &impl Serialize,
    ) -> Result<bool> {
        let content = canonical_json_bytes(value);
        self.stage_bytes_for_recovery(target, &content, MAX_RECORD_BYTES)
    }

    fn stage_bytes_for_recovery(
        &self,
        target: &Path,
        content: &[u8],
        maximum: u64,
    ) -> Result<bool> {
        if !publication_needed(target, content, maximum)? {
            return Ok(false);
        }
        let temporary = pending_path(target);
        let mut cleanup = PendingCleanup::new(temporary.clone());
        if let Err(error) = write_pending(&temporary, content) {
            return Err(cleanup.after_failure(error));
        }
        let parent = target
            .parent()
            .expect("canonical publication targets are rooted in the workspace");
        if let Err(error) = sync_directory(parent) {
            return Err(cleanup.after_failure(Error::AmbiguousEffect(format!(
                "pending recovery record may not be durable at {}: {error}",
                temporary.display()
            ))));
        }
        Ok(true)
    }
}
