use std::fs::{self, File};
use std::io;
use std::path::{Component, Path};

use crate::{Error, Result};

use super::super::path_safety::reject_symlink_chain;
use super::super::storage::{map_io, same_file_identity};

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
pub(super) fn exchange(target: &Path, transaction: &Path) -> Result<()> {
    use rustix::fs::{RenameFlags, renameat_with};

    let root = target.parent().expect("instructions have a parent");
    let target_name = validated_leaf(target)?;
    reject_symlink_chain(root)?;
    reject_symlink_chain(transaction)?;
    let root_handle = open_directory(root)?;
    let transaction_handle = open_directory(transaction)?;
    verify_anchor(root, &root_handle)?;
    verify_anchor(transaction, &transaction_handle)?;
    ensure_same_device(root, &root_handle, transaction, &transaction_handle)?;
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
        &transaction_handle,
        "exchange",
        &root_handle,
        target_name,
        RenameFlags::EXCHANGE,
    )
    .map_err(|error| {
        exchange_error(
            target,
            &format!("operating-system exchange failed with {error}"),
        )
    })?;
    verify_anchor(root, &root_handle)
        .map_err(|error| exchange_error(target, &format!("post-exchange root check: {error}")))?;
    verify_anchor(transaction, &transaction_handle).map_err(|error| {
        exchange_error(target, &format!("post-exchange transaction check: {error}"))
    })
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) fn exchange(_target: &Path, _transaction: &Path) -> Result<()> {
    ensure_exchange_platform()
}

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

#[cfg(unix)]
fn open_directory(path: &Path) -> Result<File> {
    use rustix::fs::{Mode, OFlags, open};

    let descriptor = open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| {
        Error::io(
            "open project instruction directory without following symlinks",
            path,
            error.into(),
        )
    })?;
    let file = File::from(descriptor);
    let metadata = map_io(
        file.metadata(),
        "inspect project instruction directory",
        path,
    )?;
    if !metadata.is_dir() {
        return Err(Error::invalid(
            "project instruction directory",
            format!("{} is not a directory", path.display()),
        ));
    }
    Ok(file)
}

#[cfg(not(unix))]
fn open_directory(_: &Path) -> Result<File> {
    ensure_exchange_platform()?;
    unreachable!("unsupported platforms fail before opening directories")
}

fn verify_anchor(path: &Path, handle: &File) -> Result<()> {
    let current = map_io(
        fs::symlink_metadata(path),
        "inspect project instruction directory anchor",
        path,
    )?;
    let opened = map_io(
        handle.metadata(),
        "inspect opened project instruction directory",
        path,
    )?;
    if !current.is_dir() || !same_file_identity(&current, &opened) {
        return Err(Error::AmbiguousEffect(format!(
            "project instruction directory identity changed at {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn ensure_same_device(
    root: &Path,
    root_handle: &File,
    transaction: &Path,
    transaction_handle: &File,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;

    let root_metadata = map_io(root_handle.metadata(), "inspect project directory", root)?;
    let transaction_metadata = map_io(
        transaction_handle.metadata(),
        "inspect project instruction transaction",
        transaction,
    )?;
    if root_metadata.dev() != transaction_metadata.dev() {
        return Err(Error::invalid(
            "project instruction atomic exchange",
            "canonical and exchange entries are on different devices",
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn ensure_same_device(_: &Path, _: &File, _: &Path, _: &File) -> Result<()> {
    ensure_exchange_platform()
}

fn exchange_error(target: &Path, reason: &str) -> Error {
    Error::AmbiguousEffect(format!(
        "atomic project instruction exchange at {} is unsupported or has an uncertain effect; retained transaction evidence: {reason}",
        target.display()
    ))
}

#[cfg(any(test, coverage))]
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

#[cfg(not(any(test, coverage)))]
fn inject_path_continuity_probe(_target: &Path) {}

#[cfg(any(test, coverage))]
fn inject_concurrent_target_change(target: &Path) {
    if super::super::take_storage_failure("agent instruction appeared during append publication") {
        fs::write(target, b"concurrent target claimant")
            .expect("write injected concurrent target claimant");
    }
}

#[cfg(not(any(test, coverage)))]
fn inject_concurrent_target_change(_target: &Path) {}

#[cfg(all(coverage, test))]
#[path = "tests/agent_integration_atomic_exchange_coverage.rs"]
mod coverage_tests;
