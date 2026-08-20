use crate::Result;

use super::retrieval::validate_limit;
use super::retrieval_context::context_from_snapshot;
use super::write_lock::WorkspaceWriteLock;
use super::{HandoffBundle, HandoffValidationReceipt, Workspace};

impl Workspace {
    pub fn handoff(
        &self,
        id: &str,
        generated_at: &str,
        query: Option<&str>,
        limit: usize,
    ) -> Result<HandoffBundle> {
        let limit = validate_limit(limit)?;
        let _write_lock = WorkspaceWriteLock::acquire(&self.state)?;
        let snapshot = self.load_snapshot()?;
        let validation = self.validation_from_snapshot(&snapshot);
        let context = context_from_snapshot(self, snapshot, query, limit)?;
        let validation = HandoffValidationReceipt::new(validation, &context);
        let bundle = HandoffBundle {
            schema_version: 2,
            kind: "handoff".to_owned(),
            id: id.to_owned(),
            generated_at: generated_at.to_owned(),
            context,
            validation: Some(validation),
        };
        bundle.validate()?;
        Ok(bundle)
    }
}
