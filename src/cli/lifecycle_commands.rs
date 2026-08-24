use std::path::Path;

use crate::workspace::Workspace;
use crate::{Error, Result};

use super::input::read_review_authority_pair;
use super::output_arguments::RecoveryArgs;
use super::render::{print_json, print_text, terminal_text};

pub(super) fn initialize(
    path: &Path,
    name: &str,
    review_authority_id: Option<String>,
    review_authority_public_key: Option<String>,
    without_review_authority: bool,
) -> Result<()> {
    let authority =
        match read_review_authority_pair(review_authority_id, review_authority_public_key)? {
            Some(authority) => Some(authority),
            None if without_review_authority => None,
            None => {
                return Err(Error::invalid(
                    "review authority",
                    "supply a review authority or explicitly use --without-review-authority",
                ));
            }
        };
    let workspace = match authority {
        Some(ref authority) => Workspace::initialize_with_review_authority(path, name, authority)?,
        None => Workspace::initialize(path, name)?,
    };
    let mode = match authority {
        Some(ref authority) => format!(
            "anchored {} ({})",
            terminal_text(&authority.id),
            terminal_text(&authority.fingerprint)
        ),
        None => "unanchored; irreversible; claim promotion disabled".to_owned(),
    };
    let integration = workspace.agent_integration_onboarding_status();
    print_text(&format!(
        "Initialized Research Run workspace at {}\nReview authority: {mode}\nAgent integration ready: {}\nNext: run 'research-run agent-integration plan {}', review the plan, apply it, then start a fresh agent run/session.",
        terminal_text(&workspace.root().display().to_string()),
        integration.agent_integration_ready,
        terminal_text(&workspace.root().display().to_string()),
    ))
}

pub(super) fn recover(output: RecoveryArgs) -> Result<()> {
    let workspace = Workspace::for_recovery(&output.path)?;
    let recovery = workspace.recover()?;
    if output.json {
        print_json(&recovery)
    } else {
        print_text(&format!(
            "Recovered publications: {}\nDiscarded identical pending files: {}",
            recovery.recovered.len(),
            recovery.discarded_identical.len()
        ))
    }
}
