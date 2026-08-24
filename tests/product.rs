#[path = "product/agent_integration_activation_defense.rs"]
mod agent_integration_activation_defense;
#[path = "product/agent_integration_defense.rs"]
mod agent_integration_defense;
#[path = "product/agent_integration_journey.rs"]
mod agent_integration_journey;
#[cfg(target_os = "linux")]
#[path = "product/agent_integration_setgid.rs"]
mod agent_integration_setgid;
#[path = "product/contribution_protocol_journey.rs"]
mod contribution_protocol_journey;
#[path = "product/data_contracts.rs"]
mod data_contracts;
#[path = "product/data_contracts_agent_integration.rs"]
mod data_contracts_agent_integration;
#[path = "product/data_contracts_handoff_schema.rs"]
mod data_contracts_handoff_schema;
#[path = "product/data_contracts_validation_schema.rs"]
mod data_contracts_validation_schema;
#[path = "product/domain_boundaries.rs"]
mod domain_boundaries;
#[path = "product/handoff_defense.rs"]
mod handoff_defense;
#[path = "product/handoff_human.rs"]
mod handoff_human;
#[path = "product/handoff_journey.rs"]
mod handoff_journey;
#[path = "product/identifier_validation.rs"]
mod identifier_validation;
#[path = "product/knowledge_history.rs"]
mod knowledge_history;
#[path = "product/migration_journey.rs"]
mod migration_journey;
#[path = "product/repository_authority.rs"]
mod repository_authority;
#[path = "product/researcher_journey.rs"]
mod researcher_journey;
#[cfg(coverage)]
#[path = "product/resilience.rs"]
mod resilience;
#[path = "product/retrieval_journey.rs"]
mod retrieval_journey;
#[path = "product/retrofit_journey.rs"]
mod retrofit_journey;
#[path = "product/retry_behavior.rs"]
mod retry_behavior;
#[path = "product/review_authority.rs"]
mod review_authority;
#[path = "product/review_test_signing.rs"]
mod review_test_signing;
#[path = "product/workspace_defense.rs"]
mod workspace_defense;
