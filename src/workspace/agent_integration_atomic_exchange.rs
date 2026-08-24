#[cfg(all(any(test, coverage), any(target_os = "linux", target_os = "macos")))]
use std::fs;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::io;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::Component;
use std::path::Path;

use crate::{Error, Result};

use super::directories::ExchangeHandles;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use super::directories::verify_anchor;
#[cfg(all(coverage, test))]
use super::directories::{ensure_same_device, open_directory};

pub(super) fn ensure_exchange_platform() -> Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        Ok(())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Err(Error::invalid(
            "project instruction atomic exchange",
            "append publication requires Linux renameat2 or macOS renameatx_np",
        ))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn exchange_with_handles(
    target: &Path,
    transaction: &Path,
    handles: &ExchangeHandles,
) -> Result<()> {
    use rustix::fs::{RenameFlags, renameat_with};

    let target_name = validated_leaf(target)?;
    super::directories::verify_relative_directory_anchor(
        &handles.root,
        transaction,
        &handles.transaction,
    )
    .map_err(|error| exchange_error(target, &format!("pre-exchange transaction check: {error}")))?;
    inject_path_continuity_probe(target);
    inject_concurrent_target_change(target);
    if super::super::injected_storage_failure("replace project instructions") {
        return Err(Error::io(
            "atomic exchange project instructions",
            target,
            io::Error::other("injected storage failure before exchange"),
        ));
    }
    if super::super::injected_storage_failure("atomic exchange project instructions") {
        return Err(exchange_error(
            target,
            "injected atomic exchange failure with uncertain effect",
        ));
    }
    renameat_with(
        &handles.transaction,
        "exchange",
        &handles.root,
        target_name,
        RenameFlags::EXCHANGE,
    )
    .map_err(|error| {
        exchange_error(
            target,
            &format!("operating-system exchange failed with {error}"),
        )
    })?;
    let root = target.parent().expect("instructions have a parent");
    verify_anchor(root, &handles.root)
        .map_err(|error| exchange_error(target, &format!("post-exchange root check: {error}")))?;
    verify_anchor(transaction, &handles.transaction).map_err(|error| {
        exchange_error(target, &format!("post-exchange transaction check: {error}"))
    })?;
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) fn exchange_with_handles(
    _target: &Path,
    _transaction: &Path,
    _handles: &ExchangeHandles,
) -> Result<()> {
    ensure_exchange_platform()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn validated_leaf(path: &Path) -> Result<&std::ffi::OsStr> {
    let name = path.file_name().ok_or_else(|| {
        Error::invalid(
            "project instruction atomic exchange",
            "instruction target has no leaf name",
        )
    })?;
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(Error::invalid(
            "project instruction atomic exchange",
            "instruction target must be one relative leaf name",
        ));
    }
    if name != "AGENTS.md" && name != "AGENTS.override.md" {
        return Err(Error::invalid(
            "project instruction atomic exchange",
            "instruction target is outside the supported root instruction surface",
        ));
    }
    Ok(name)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn exchange_error(target: &Path, reason: &str) -> Error {
    Error::AmbiguousEffect(format!(
        "atomic project instruction exchange at {} is unsupported or has an uncertain effect; retained transaction evidence: {reason}",
        target.display()
    ))
}

#[cfg(all(any(test, coverage), any(target_os = "linux", target_os = "macos")))]
fn inject_path_continuity_probe(target: &Path) {
    if super::super::take_storage_failure("probe continuous agent instruction path") {
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
        {
            Ok(file) => drop(file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!("probe project instruction path: {error}"),
        }
    }
}

#[cfg(all(
    not(any(test, coverage)),
    any(target_os = "linux", target_os = "macos")
))]
fn inject_path_continuity_probe(_target: &Path) {}

#[cfg(all(any(test, coverage), any(target_os = "linux", target_os = "macos")))]
fn inject_concurrent_target_change(target: &Path) {
    if super::super::take_storage_failure("agent instruction appeared during append publication") {
        fs::write(target, b"concurrent target claimant")
            .expect("write injected concurrent target claimant");
    }
}

#[cfg(all(
    not(any(test, coverage)),
    any(target_os = "linux", target_os = "macos")
))]
fn inject_concurrent_target_change(_target: &Path) {}

#[cfg(test)]
#[path = "tests/agent_integration_atomic_exchange.rs"]
mod tests;

#[cfg(all(coverage, test))]
#[path = "tests/agent_integration_atomic_exchange_coverage.rs"]
mod coverage_tests;
