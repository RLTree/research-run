use crate::Result;

use super::path_safety::create_directory_chain;
use super::write_lock::WorkspaceWriteLock;
use super::{CONTRIBUTION_PROTOCOL_DIRECTORY, RecoveryResult, Workspace};

impl Workspace {
    pub fn recover(&self) -> Result<RecoveryResult> {
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        self.recover_pending_under_lock()
    }

    pub(super) fn recover_pending_under_lock(&self) -> Result<RecoveryResult> {
        let mut recovery = self.preflight_recovery_batch()?;
        if recovery.requires_contribution_protocol_bootstrap() {
            create_directory_chain(&self.state.join(CONTRIBUTION_PROTOCOL_DIRECTORY))?;
            self.stage_record_for_recovery(
                CONTRIBUTION_PROTOCOL_DIRECTORY,
                &crate::domain::ContributionProtocol::agent_v1(),
            )?;
            recovery = self.preflight_recovery_batch()?;
        }
        let mut result = RecoveryResult {
            recovered: Vec::new(),
            discarded_identical: Vec::new(),
        };
        recovery.publish(self, &mut result)?;
        self.load_snapshot()?;
        Ok(result)
    }
}
