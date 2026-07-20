use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::Path;

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

pub(super) fn print_effect(kind: &str, id: &str, created: bool) -> Result<()> {
    print_text(&format!(
        "{} {kind} {id}",
        if created { "Added" } else { "Already present" }
    ))
}

pub fn terminal_text(value: &str) -> String {
    value.chars().flat_map(char::escape_default).collect()
}

pub(super) fn print_json(value: &impl serde::Serialize) -> Result<()> {
    let output = serde_json::to_string_pretty(value).expect(
        "Research Run projections contain only JSON-representable strings, numbers, lists, and enums",
    );
    print_text(&output)
}

pub(super) fn print_human_status(status: &Status) -> Result<()> {
    print_text(&render_human_status(status))
}

pub(super) fn render_human_status(status: &Status) -> String {
    let mut output = String::new();
    writeln!(
        &mut output,
        "Research Run: {} ({})",
        terminal_text(&status.project.name),
        terminal_text(&status.project.id)
    )
    .expect("String rendering is infallible");
    writeln!(&mut output, "Claim ceiling: {}", status.claim_ceiling)
        .expect("String rendering is infallible");
    render_review_authority(&mut output, status);
    render_claims(&mut output, status);
    render_ai_drafts(&mut output, status);
    render_experiments(&mut output, status);
    output
}

fn render_review_authority(output: &mut String, status: &Status) {
    writeln!(output, "\nReview authority").expect("String rendering is infallible");
    writeln!(
        output,
        "- Mode: {}; promotion capable: {}; repairable: {}",
        status.review_authority.mode,
        status.review_authority.promotion_capable,
        status.review_authority.repairable
    )
    .expect("String rendering is infallible");
    if let Some(id) = &status.review_authority.id {
        writeln!(output, "  ID: {}", terminal_text(id)).expect("String rendering is infallible");
    }
    if let Some(fingerprint) = &status.review_authority.fingerprint {
        writeln!(output, "  Fingerprint: {}", terminal_text(fingerprint))
            .expect("String rendering is infallible");
    }
    if let Some(blocker) = status.review_authority.blocker {
        writeln!(output, "  Blocker: {}", terminal_text(blocker))
            .expect("String rendering is infallible");
    }
    writeln!(
        output,
        "  Next: {}",
        terminal_text(status.review_authority.next_action)
    )
    .expect("String rendering is infallible");
}

fn render_claims(output: &mut String, status: &Status) {
    writeln!(output, "\nClaims").expect("String rendering is infallible");
    if status.claims.is_empty() {
        writeln!(output, "- None. Next action: add a claim.")
            .expect("String rendering is infallible");
    }
    for claim in &status.claims {
        writeln!(
            output,
            "- {}: {:?} — {}",
            terminal_text(&claim.id),
            claim.assessment,
            terminal_text(&claim.text)
        )
        .expect("String rendering is infallible");
        writeln!(output, "  Scope: {}", terminal_text(&claim.scope))
            .expect("String rendering is infallible");
        for blocker in &claim.blockers {
            writeln!(output, "  Blocker: {}", terminal_text(blocker))
                .expect("String rendering is infallible");
        }
        writeln!(output, "  Next: {}", terminal_text(&claim.next_action))
            .expect("String rendering is infallible");
    }
}

fn render_ai_drafts(output: &mut String, status: &Status) {
    writeln!(output, "\nUnreviewed AI drafts").expect("String rendering is infallible");
    if status.unreviewed_ai_drafts.is_empty() {
        writeln!(output, "- None.").expect("String rendering is infallible");
    }
    for draft in &status.unreviewed_ai_drafts {
        writeln!(
            output,
            "- {}/{}: {}",
            terminal_text(draft.kind),
            terminal_text(&draft.id),
            terminal_text(draft.reason)
        )
        .expect("String rendering is infallible");
    }
}

fn render_experiments(output: &mut String, status: &Status) {
    writeln!(output, "\nExperiments").expect("String rendering is infallible");
    if status.experiments.is_empty() {
        writeln!(output, "- None.").expect("String rendering is infallible");
    }
    for experiment in &status.experiments {
        writeln!(
            output,
            "- {}: {:?}",
            terminal_text(&experiment.id),
            experiment.outcome
        )
        .expect("String rendering is infallible");
        for observation in &experiment.observations {
            writeln!(output, "  Observation: {}", terminal_text(observation))
                .expect("String rendering is infallible");
        }
        writeln!(
            output,
            "  Interpretation: {}",
            terminal_text(&experiment.interpretation)
        )
        .expect("String rendering is infallible");
        writeln!(output, "  Next: {}", terminal_text(&experiment.next_move))
            .expect("String rendering is infallible");
    }
}

pub(super) fn print_text(value: &str) -> Result<()> {
    write_stdout(value)
}

pub fn print_error(error: &Error) -> Result<()> {
    let stderr = io::stderr();
    let mut output = stderr.lock();
    writeln!(output, "error: {}", terminal_text(&error.to_string()))
        .and_then(|()| output.flush())
        .map_err(|source| Error::io("write standard error", Path::new("stderr"), source))
}

fn write_stdout(value: &str) -> Result<()> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    writeln!(output, "{value}")
        .and_then(|()| output.flush())
        .map_err(|source| Error::io("write standard output", Path::new("stdout"), source))
}
