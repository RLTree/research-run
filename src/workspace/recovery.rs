use crate::Result;

use super::write_lock::WorkspaceWriteLock;
use super::{RecoveryResult, Workspace};

impl Workspace {
    pub fn recover(&self) -> Result<RecoveryResult> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        let recovery = self.preflight_recovery_batch()?;
        let mut result = RecoveryResult {
            recovered: Vec::new(),
            discarded_identical: Vec::new(),
        };
        recovery.publish(self, &mut result)?;
        self.load_snapshot()?;
        Ok(result)
    }
}
