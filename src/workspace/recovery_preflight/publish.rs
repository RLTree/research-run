use crate::Result;

use super::RecoveryBatch;
use crate::workspace::recovery_commit::commit_recovery;
use crate::workspace::{CONTRIBUTION_PROTOCOL_DIRECTORY, RecoveryResult, Workspace};

impl RecoveryBatch {
    pub(in crate::workspace) fn authorizes_contribution_protocol_bootstrap(&self) -> bool {
        self.manifest.iter().any(|record| !record.target.exists()) || !self.migrations.is_empty()
    }

    pub(in crate::workspace) fn publish(
        mut self,
        workspace: &Workspace,
        result: &mut RecoveryResult,
    ) -> Result<()> {
        let activates_fresh_workspace = self.manifest.iter().any(|record| !record.target.exists());
        if activates_fresh_workspace {
            commit_recovery(
                &workspace.state.join(CONTRIBUTION_PROTOCOL_DIRECTORY),
                std::mem::take(&mut self.contribution_protocols),
                result,
            )?;
            commit_recovery(
                &workspace.state.join("review-authorities"),
                std::mem::take(&mut self.review_authorities),
                result,
            )?;
        }
        commit_recovery(&workspace.state, self.manifest, result)?;
        for (directory, pending) in [
            ("sources", self.sources),
            ("claims", self.claims),
            ("experiments", self.experiments),
            ("evidence", self.evidence),
            ("review-authorities", self.review_authorities),
            ("reviews", self.reviews),
            ("inventories", self.inventories),
            ("knowledge", self.knowledge),
            ("relationships", self.relationships),
            (CONTRIBUTION_PROTOCOL_DIRECTORY, self.contribution_protocols),
            ("migrations", self.migrations),
        ] {
            commit_recovery(&workspace.state.join(directory), pending, result)?;
        }
        Ok(())
    }
}
