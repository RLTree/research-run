use serde::{Deserialize, Serialize};

use crate::{Error, Result};

use super::CanonicalRecord;
use super::validation::{validate_header, validate_hex_digest, validate_id, validate_timestamp};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MigrationPlan {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub project_id: String,
    pub from_format: String,
    pub to_format: String,
    pub migrated_at: String,
    pub authority_files: usize,
    pub authority_bytes: u64,
    pub authority_sha256: String,
}

impl MigrationPlan {
    pub fn validate(&self) -> Result<()> {
        validate_header(self.schema_version, &self.kind, "migration-plan")?;
        validate_fields(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MigrationRecord {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub project_id: String,
    pub from_format: String,
    pub to_format: String,
    pub migrated_at: String,
    pub authority_files: usize,
    pub authority_bytes: u64,
    pub authority_sha256: String,
}

impl From<MigrationPlan> for MigrationRecord {
    fn from(plan: MigrationPlan) -> Self {
        Self {
            schema_version: plan.schema_version,
            kind: "migration".to_owned(),
            id: plan.id,
            project_id: plan.project_id,
            from_format: plan.from_format,
            to_format: plan.to_format,
            migrated_at: plan.migrated_at,
            authority_files: plan.authority_files,
            authority_bytes: plan.authority_bytes,
            authority_sha256: plan.authority_sha256,
        }
    }
}

impl CanonicalRecord for MigrationRecord {
    const KIND: &'static str = "migration";

    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self) -> Result<()> {
        validate_header(self.schema_version, &self.kind, Self::KIND)?;
        validate_fields(self)
    }
}

trait MigrationFields {
    fn id(&self) -> &str;
    fn project_id(&self) -> &str;
    fn source_format(&self) -> &str;
    fn target_format(&self) -> &str;
    fn migrated_at(&self) -> &str;
    fn authority_files(&self) -> usize;
    fn authority_bytes(&self) -> u64;
    fn authority_sha256(&self) -> &str;
}

macro_rules! migration_fields {
    ($type:ty) => {
        impl MigrationFields for $type {
            fn id(&self) -> &str {
                &self.id
            }
            fn project_id(&self) -> &str {
                &self.project_id
            }
            fn source_format(&self) -> &str {
                &self.from_format
            }
            fn target_format(&self) -> &str {
                &self.to_format
            }
            fn migrated_at(&self) -> &str {
                &self.migrated_at
            }
            fn authority_files(&self) -> usize {
                self.authority_files
            }
            fn authority_bytes(&self) -> u64 {
                self.authority_bytes
            }
            fn authority_sha256(&self) -> &str {
                &self.authority_sha256
            }
        }
    };
}

migration_fields!(MigrationPlan);
migration_fields!(MigrationRecord);

fn validate_fields(value: &impl MigrationFields) -> Result<()> {
    validate_id(value.id(), "migration id")?;
    validate_id(value.project_id(), "migration project_id")?;
    if value.source_format() != "research-run-v0.1"
        || value.target_format() != "research-run-v0.1-extended"
    {
        return Err(Error::invalid(
            "migration format",
            "unsupported migration boundary",
        ));
    }
    validate_timestamp(value.migrated_at())?;
    if value.authority_files() == 0 || value.authority_bytes() == 0 {
        return Err(Error::invalid(
            "migration authority",
            "must contain at least one canonical file and byte",
        ));
    }
    validate_hex_digest(value.authority_sha256(), "migration digest")
}
