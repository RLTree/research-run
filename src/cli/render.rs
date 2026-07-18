use crate::domain::{ArtifactLocatorType, ArtifactPointer, required_text};
use crate::workspace::Status;
use crate::{Error, Result};

pub(super) fn parse_artifact(value: &str) -> Result<ArtifactPointer> {
    let (locator_type, rest) = value
        .split_once(':')
        .ok_or_else(|| Error::invalid("artifact pointer", "expected TYPE:LOCATOR:DESCRIPTION"))?;
    let (locator, description) = rest
        .rsplit_once(':')
        .ok_or_else(|| Error::invalid("artifact pointer", "expected TYPE:LOCATOR:DESCRIPTION"))?;
    let locator_type = match locator_type {
        "workspace" => ArtifactLocatorType::Workspace,
        "external" => ArtifactLocatorType::External,
        _ => {
            return Err(Error::invalid(
                "artifact pointer type",
                "expected workspace or external",
            ));
        }
    };
    Ok(ArtifactPointer {
        locator_type,
        locator: required_text(locator, "artifact locator")?,
        description: required_text(description, "artifact description")?,
        digest: None,
    })
}

pub(super) fn print_effect(kind: &str, id: &str, created: bool) {
    println!(
        "{} {kind} {id}",
        if created { "Added" } else { "Already present" }
    );
}

pub fn terminal_text(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

pub(super) fn print_json(value: &impl serde::Serialize) {
    let output = serde_json::to_string_pretty(value).expect(
        "Research Run projections contain only JSON-representable strings, numbers, lists, and enums",
    );
    println!("{output}");
}

pub(super) fn print_human_status(status: &Status) {
    println!(
        "Research Run: {} ({})",
        terminal_text(&status.project.name),
        terminal_text(&status.project.id)
    );
    println!("Claim ceiling: {}", status.claim_ceiling);
    println!("\nClaims");
    if status.claims.is_empty() {
        println!("- None. Next action: add a claim.");
    }
    for claim in &status.claims {
        println!(
            "- {}: {:?} — {}",
            terminal_text(&claim.id),
            claim.assessment,
            terminal_text(&claim.text)
        );
        println!("  Scope: {}", terminal_text(&claim.scope));
        for blocker in &claim.blockers {
            println!("  Blocker: {}", terminal_text(blocker));
        }
        println!("  Next: {}", terminal_text(&claim.next_action));
    }
    println!("\nUnreviewed AI drafts");
    if status.unreviewed_ai_drafts.is_empty() {
        println!("- None.");
    }
    for draft in &status.unreviewed_ai_drafts {
        println!(
            "- {}/{}: {}",
            terminal_text(draft.kind),
            terminal_text(&draft.id),
            terminal_text(draft.reason)
        );
    }
    println!("\nExperiments");
    if status.experiments.is_empty() {
        println!("- None.");
    }
    for experiment in &status.experiments {
        println!(
            "- {}: {:?}",
            terminal_text(&experiment.id),
            experiment.outcome
        );
        for observation in &experiment.observations {
            println!("  Observation: {}", terminal_text(observation));
        }
        println!(
            "  Interpretation: {}",
            terminal_text(&experiment.interpretation)
        );
        println!("  Next: {}", terminal_text(&experiment.next_move));
    }
}
