use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::domain::{AgentIntegrationPlan, digest};
use crate::{Error, Result};

use super::agent_integration_recovery::plan_transition_matches;
use super::agent_integration_transaction::witnesses::{
    PREVIOUS_TRANSACTION_VERSION, TRANSACTION_VERSION, WitnessPaths, verify_witnesses,
};
use super::agent_integration_transaction::{exchange_and_finish, finish_verified_exchange};
use super::agent_integration_types::MAX_INSTRUCTION_BYTES;
use super::path_safety::reject_symlink_chain;
use super::storage::{map_io, read_bounded_with_limit};

const VERSIONED_NAMES: [&str; 4] = ["exchange", "original", "reviewed", "version"];
const COMPLETION_NAME: &str = "completion";
const LEGACY_NAMES: [&str; 2] = ["planned", "source"];
const ALL_TRANSACTION_NAMES: [&str; 7] = [
    "exchange",
    "original",
    "reviewed",
    "version",
    COMPLETION_NAME,
    "planned",
    "source",
];
const MAX_TRANSACTION_CHILDREN: usize = VERSIONED_NAMES.len() + 1;

enum TransactionLayout {
    Versioned { has_completion: bool },
    Legacy,
}

pub(super) fn recover_transaction(
    transaction: &Path,
    target: &Path,
    plan: &AgentIntegrationPlan,
) -> Result<()> {
    let layout = inspect_layout(transaction).map_err(|error| match error {
        error @ Error::AmbiguousEffect(_) => error,
        error => retained_state(transaction, &error.to_string()),
    })?;
    match layout {
        TransactionLayout::Versioned { has_completion } => {
            recover_versioned(transaction, target, plan, has_completion)
        }
        TransactionLayout::Legacy => Err(Error::AmbiguousEffect(format!(
            "legacy project instruction transaction requires explicit inspection and is retained at {}",
            transaction.display()
        ))),
    }
}

fn recover_versioned(
    transaction: &Path,
    target: &Path,
    plan: &AgentIntegrationPlan,
    has_completion: bool,
) -> Result<()> {
    let paths = WitnessPaths::new(transaction);
    let version = read_witness(transaction, &paths.version)?;
    if version != PREVIOUS_TRANSACTION_VERSION && version != TRANSACTION_VERSION {
        return Err(retained_state(
            transaction,
            "project instruction transaction version is unknown",
        ));
    }
    let original = read_witness(transaction, &paths.original)?;
    let reviewed = read_witness(transaction, &paths.reviewed)?;
    let exchanged = read_witness(transaction, &paths.exchange)?;
    let reviewed_matches = plan_transition_matches(plan, &reviewed)
        .map_err(|error| retained_state(transaction, &error.to_string()))?;
    if !source_matches_plan(plan, &original) || !reviewed_matches {
        return Err(Error::AmbiguousEffect(format!(
            "project instruction transaction does not match the reviewed plan at {}",
            transaction.display()
        )));
    }
    let Some(current) =
        read_optional(target).map_err(|error| retained_state(transaction, &error.to_string()))?
    else {
        return Err(retained_state(
            transaction,
            "canonical instruction path is missing",
        ));
    };
    if current == original && exchanged == reviewed {
        if has_completion {
            return Err(retained_state(
                transaction,
                "a staged completion receipt cannot precede atomic exchange",
            ));
        }
        verify_witnesses(target, &paths, &original, &reviewed, &reviewed, &version)
            .map_err(|error| retained_state(transaction, &error.to_string()))?;
        return exchange_and_finish(
            target,
            transaction,
            &original,
            &reviewed,
            &plan.plan_sha256,
            &version,
        );
    }
    if current == reviewed && exchanged == original {
        verify_witnesses(target, &paths, &original, &reviewed, &original, &version)
            .map_err(|error| retained_state(transaction, &error.to_string()))?;
        return finish_verified_exchange(
            target,
            transaction,
            &original,
            &reviewed,
            &plan.plan_sha256,
            &version,
        );
    }
    Err(retained_state(
        transaction,
        "canonical and exchange bytes are not an exact pre- or post-exchange state",
    ))
}

fn read_witness(transaction: &Path, path: &Path) -> Result<Vec<u8>> {
    read_bounded_with_limit(path, MAX_INSTRUCTION_BYTES)
        .map_err(|error| retained_state(transaction, &error.to_string()))
}

fn inspect_layout(transaction: &Path) -> Result<TransactionLayout> {
    reject_symlink_chain(transaction)?;
    let metadata = map_io(
        fs::symlink_metadata(transaction),
        "inspect project instruction transaction",
        transaction,
    )?;
    if !metadata.is_dir() {
        return Err(Error::invalid(
            "project instruction transaction",
            format!("{} is not a directory", transaction.display()),
        ));
    }
    let mut names = BTreeSet::new();
    for entry in map_io(
        fs::read_dir(transaction),
        "inspect project instruction transaction",
        transaction,
    )? {
        let entry = map_io(
            entry,
            "inspect project instruction transaction",
            transaction,
        )?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(retained_state(transaction, "non-UTF-8 transaction child"));
        };
        if !ALL_TRANSACTION_NAMES.contains(&name) {
            return Err(retained_state(
                transaction,
                &format!("unexpected transaction child: {name}"),
            ));
        }
        if names.len() >= MAX_TRANSACTION_CHILDREN {
            return Err(retained_state(
                transaction,
                "transaction exceeds its closed five-child budget",
            ));
        }
        inspect_child(transaction, &entry.path())?;
        names.insert(name.to_owned());
    }
    classify_names(transaction, &names)
}

fn inspect_child(transaction: &Path, path: &Path) -> Result<()> {
    reject_symlink_chain(path)?;
    let metadata = map_io(
        fs::symlink_metadata(path),
        "inspect transaction witness",
        path,
    )?;
    if !metadata.is_file() {
        return Err(retained_state(
            transaction,
            &format!(
                "transaction child is not a regular file: {}",
                path.display()
            ),
        ));
    }
    Ok(())
}

fn classify_names(transaction: &Path, names: &BTreeSet<String>) -> Result<TransactionLayout> {
    let versioned: BTreeSet<String> = VERSIONED_NAMES.iter().map(ToString::to_string).collect();
    let legacy: BTreeSet<String> = LEGACY_NAMES.iter().map(ToString::to_string).collect();
    if names == &versioned {
        return Ok(TransactionLayout::Versioned {
            has_completion: false,
        });
    }
    let mut completing = versioned.clone();
    completing.insert(COMPLETION_NAME.to_owned());
    if names == &completing {
        return Ok(TransactionLayout::Versioned {
            has_completion: true,
        });
    }
    if !names.is_empty() && names.is_subset(&legacy) {
        return Ok(TransactionLayout::Legacy);
    }
    Err(retained_state(
        transaction,
        "transaction children are missing, mixed, or unexpected",
    ))
}

fn read_optional(target: &Path) -> Result<Option<Vec<u8>>> {
    reject_symlink_chain(target)?;
    match fs::symlink_metadata(target) {
        Ok(metadata) if metadata.is_file() => {
            read_bounded_with_limit(target, MAX_INSTRUCTION_BYTES).map(Some)
        }
        Ok(_) => Err(Error::invalid(
            "project instructions",
            format!("{} is not a regular file", target.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Error::io("inspect project instructions", target, error)),
    }
}

fn source_matches_plan(plan: &AgentIntegrationPlan, bytes: &[u8]) -> bool {
    bytes.len() as u64 == plan.instruction_bytes
        && plan.instruction_sha256.as_deref() == Some(digest(bytes).as_str())
}

fn retained_state(transaction: &Path, reason: &str) -> Error {
    Error::AmbiguousEffect(format!(
        "project instruction transaction is retained at {}: {reason}",
        transaction.display()
    ))
}

#[cfg(all(coverage, test))]
#[path = "tests/agent_integration_transaction_recovery_coverage.rs"]
mod coverage_tests;
