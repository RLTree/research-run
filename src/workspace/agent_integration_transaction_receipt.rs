use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::domain::{AgentIntegrationPlan, digest, is_lowercase_sha256};
use crate::{Error, Result};

use super::super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::super::storage::parse_json;

pub(super) const MAX_COMPLETION_RECEIPT_BYTES: u64 = 4_096;
const RECEIPT_KIND: &str = "agent-integration-publication-completion";
const RECEIPT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompletionReceipt {
    schema_version: u32,
    kind: String,
    pub(super) target: String,
    pub(super) transaction: String,
    pub(super) transaction_version: u32,
    pub(super) plan_sha256: String,
    pub(super) original_sha256: String,
    pub(super) original_bytes: u64,
    pub(super) reviewed_sha256: String,
    pub(super) reviewed_bytes: u64,
    pub(super) unix_mode: u32,
}

impl CompletionReceipt {
    pub(super) fn new(
        target: &Path,
        transaction: &Path,
        transaction_version: u32,
        plan_sha256: &str,
        original: &[u8],
        reviewed: &[u8],
        unix_mode: u32,
    ) -> Result<Self> {
        let target = leaf_text(target, "completion target")?;
        let transaction = leaf_text(transaction, "completion transaction")?;
        let receipt = Self {
            schema_version: RECEIPT_VERSION,
            kind: RECEIPT_KIND.to_owned(),
            target,
            transaction,
            transaction_version,
            plan_sha256: plan_sha256.to_owned(),
            original_sha256: digest(original),
            original_bytes: original.len() as u64,
            reviewed_sha256: digest(reviewed),
            reviewed_bytes: reviewed.len() as u64,
            unix_mode,
        };
        receipt.validate_shape()?;
        Ok(receipt)
    }

    pub(super) fn parse(bytes: &[u8], path: &Path) -> Result<Self> {
        let receipt: Self = parse_json(bytes, path)?;
        receipt.validate_shape()?;
        Ok(receipt)
    }

    pub(super) fn bytes(&self) -> Vec<u8> {
        super::super::publication::canonical_json_bytes(self)
    }

    pub(super) fn validate_for_plan(&self, plan: &AgentIntegrationPlan) -> Result<()> {
        self.validate_shape()?;
        if self.plan_sha256 != plan.plan_sha256
            || self.target != plan.instruction_path
            || self.original_bytes != plan.instruction_bytes
            || plan.instruction_sha256.as_deref() != Some(self.original_sha256.as_str())
            || self.reviewed_sha256 != plan.prospective_sha256
        {
            return Err(Error::AmbiguousEffect(
                "project instruction completion receipt does not match the reviewed plan"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    pub(super) fn validate_pair(&self, receipt: &Path, transaction: Option<&Path>) -> Result<()> {
        if completion_path_for_name(receipt, &self.transaction)? != receipt {
            return Err(Error::AmbiguousEffect(
                "project instruction completion receipt path does not match its transaction"
                    .to_owned(),
            ));
        }
        if transaction
            .is_some_and(|path| path.file_name() != Some(std::ffi::OsStr::new(&self.transaction)))
        {
            return Err(Error::AmbiguousEffect(
                "project instruction completion receipt names a different transaction".to_owned(),
            ));
        }
        Ok(())
    }

    fn validate_shape(&self) -> Result<()> {
        if self.schema_version != RECEIPT_VERSION
            || self.kind != RECEIPT_KIND
            || !matches!(self.target.as_str(), "AGENTS.md" | "AGENTS.override.md")
            || !is_transaction_name(&self.transaction, &self.target)
            || !matches!(self.transaction_version, 2 | 3)
            || !is_lowercase_sha256(&self.plan_sha256)
            || !is_lowercase_sha256(&self.original_sha256)
            || !is_lowercase_sha256(&self.reviewed_sha256)
            || self.original_bytes > MAX_INSTRUCTION_BYTES
            || self.reviewed_bytes > MAX_INSTRUCTION_BYTES
            || self.unix_mode == 0
        {
            return Err(Error::AmbiguousEffect(
                "project instruction completion receipt is malformed or unsupported".to_owned(),
            ));
        }
        Ok(())
    }
}

pub(super) fn completion_path(transaction: &Path) -> Result<PathBuf> {
    let name = leaf_text(transaction, "completion transaction")?;
    let stem = name.strip_suffix(".txn").ok_or_else(|| {
        Error::invalid(
            "project instruction completion receipt",
            "transaction name must end in .txn",
        )
    })?;
    Ok(transaction.with_file_name(format!("{stem}.done")))
}

fn completion_path_for_name(receipt: &Path, transaction: &str) -> Result<PathBuf> {
    let transaction = receipt.with_file_name(transaction);
    completion_path(&transaction)
}

fn leaf_text(path: &Path, context: &str) -> Result<String> {
    let mut components = path
        .file_name()
        .map(Path::new)
        .into_iter()
        .flat_map(Path::components);
    let Some(Component::Normal(name)) = components.next() else {
        return Err(Error::invalid(
            context,
            "path must have one normal leaf name",
        ));
    };
    if components.next().is_some() {
        return Err(Error::invalid(
            context,
            "path must have one normal leaf name",
        ));
    }
    name.to_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::invalid(context, "path leaf must be UTF-8"))
}

fn is_transaction_name(name: &str, target: &str) -> bool {
    let Some(body) = name
        .strip_prefix(&format!(".{target}."))
        .and_then(|name| name.strip_suffix(".txn"))
    else {
        return false;
    };
    let Some((process, sequence)) = body.split_once('.') else {
        return false;
    };
    process.parse::<u32>().is_ok() && sequence.parse::<u64>().is_ok() && !sequence.contains('.')
}
