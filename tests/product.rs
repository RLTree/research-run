#[path = "product/data_contracts.rs"]
mod data_contracts;
#[path = "product/domain_boundaries.rs"]
mod domain_boundaries;
#[path = "product/identifier_validation.rs"]
mod identifier_validation;
#[path = "product/knowledge_history.rs"]
mod knowledge_history;
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
#[path = "product/workspace_defense.rs"]
mod workspace_defense;
