use std::path::Path;

use crate::domain::ReviewAuthority;
use crate::workspace::Workspace;
use crate::{Error, Result};

use super::input::read_text_input;
use super::output_arguments::RecoveryArgs;
use super::render::{print_json, print_text, terminal_text};

pub(super) fn initialize(
    path: &Path,
    name: &str,
    review_authority_id: Option<String>,
    review_authority_public_key: Option<String>,
) -> Result<()> {
    let authority = match (review_authority_id, review_authority_public_key) {
        (Some(id), Some(public_key)) => Some(ReviewAuthority::from_openssh(
            id,
            &read_text_input(&public_key)?,
        )?),
        (None, None) => None,
        _ => {
            return Err(Error::invalid(
                "review authority",
                "id and public key must be supplied together",
            ));
        }
    };
    let workspace = match authority {
        Some(ref authority) => Workspace::initialize_with_review_authority(path, name, authority)?,
        None => Workspace::initialize(path, name)?,
    };
    print_text(&format!(
        "Initialized Research Run workspace at {}",
        terminal_text(&workspace.root().display().to_string())
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
