use serde::{Deserialize, Serialize};

use crate::{Error, Result};

use super::validation::{validate_header, validate_id};
use super::{CanonicalRecord, FORMAT_VERSION};

const SEQUENCE: [&str; 5] = [
    "retrieve-bounded-context",
    "classify-new-material",
    "append-typed-records",
    "validate-workspace",
    "answer-or-handoff",
];
const TRIGGERS: [&str; 7] = [
    "human-observation",
    "human-correction",
    "human-decision",
    "negative-result",
    "ambiguous-result",
    "blocker",
    "next-action",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContributionProtocol {
    pub schema_version: u32,
    pub kind: String,
    pub id: String,
    pub sequence: Vec<String>,
    pub triggers: Vec<String>,
    pub human_input_authorship: String,
    pub agent_analysis_authorship: String,
    pub no_new_material_effect: String,
    pub identical_retry_effect: String,
    pub claim_promotion: String,
}

impl ContributionProtocol {
    pub fn agent_v1() -> Self {
        Self {
            schema_version: FORMAT_VERSION,
            kind: "contribution-protocol".to_owned(),
            id: "agent-contribution".to_owned(),
            sequence: strings(&SEQUENCE),
            triggers: strings(&TRIGGERS),
            human_input_authorship: "human".to_owned(),
            agent_analysis_authorship: "ai".to_owned(),
            no_new_material_effect: "no-write".to_owned(),
            identical_retry_effect: "no-op".to_owned(),
            claim_promotion: "signed-human-review-only".to_owned(),
        }
    }
}

impl CanonicalRecord for ContributionProtocol {
    const KIND: &'static str = "contribution-protocol";

    fn id(&self) -> &str {
        &self.id
    }

    fn validate(&self) -> Result<()> {
        validate_header(self.schema_version, &self.kind, Self::KIND)?;
        validate_id(&self.id, "contribution protocol id")?;
        if self != &Self::agent_v1() {
            return Err(Error::invalid(
                "contribution protocol",
                "unknown or modified activation contract",
            ));
        }
        Ok(())
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}
