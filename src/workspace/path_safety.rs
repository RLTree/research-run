use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

use super::injected_storage_failure;
use super::storage::map_io;

pub(super) fn create_directory_chain(path: &Path) -> Result<()> {
    if path.exists() {
        return reject_symlink_chain(path);
    }
    let parent = path
        .parent()
        .expect("a missing path always has an existing root ancestor");
    create_directory_chain(parent)?;
    reject_symlink_chain(parent)?;
    let created = if injected_storage_failure("directory already exists") {
        #[cfg(any(test, coverage))]
        fs::create_dir(path).expect("create injected concurrent directory");
        Err(io::Error::from(io::ErrorKind::AlreadyExists))
    } else if injected_storage_failure("create project directory") {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    } else {
        fs::create_dir(path)
    };
    match created {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(Error::io("create project directory", path, error)),
    }
    reject_symlink_chain(path)
}

pub(super) fn absolute_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        let current = map_io(
            std::env::current_dir(),
            "read current directory",
            Path::new("."),
        )?;
        current.join(path)
    };
    reject_symlink_chain(&absolute)?;
    if absolute.exists() {
        map_io(
            fs::symlink_metadata(&absolute),
            "inspect workspace root",
            &absolute,
        )?;
        return map_io(
            absolute.canonicalize(),
            "canonicalize workspace root",
            &absolute,
        );
    }
    let existing = absolute
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .expect("an absolute path always has an existing root ancestor");
    map_io(
        fs::symlink_metadata(existing),
        "inspect workspace ancestor",
        existing,
    )?;
    let canonical = map_io(
        existing.canonicalize(),
        "canonicalize workspace ancestor",
        existing,
    )?;
    let suffix = absolute
        .strip_prefix(existing)
        .expect("the selected existing directory is an ancestor");
    Ok(canonical.join(suffix))
}

pub(super) fn reject_symlink_chain(path: &Path) -> Result<()> {
    if injected_storage_failure("inspect workspace path") {
        return Err(Error::io(
            "inspect workspace path",
            path,
            io::Error::other("injected storage failure"),
        ));
    }
    for ancestor in path.ancestors() {
        let inspected = if injected_storage_failure("inspect path component") {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        } else {
            fs::symlink_metadata(ancestor)
        };
        match inspected {
            Ok(metadata)
                if metadata.file_type().is_symlink()
                    && !is_allowed_platform_alias(ancestor, &metadata) =>
            {
                return Err(Error::invalid(
                    "workspace path",
                    format!("symlink is forbidden: {}", ancestor.display()),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(Error::io("inspect workspace path", ancestor, error)),
        }
    }
    Ok(())
}

fn is_allowed_platform_alias(path: &Path, _metadata: &fs::Metadata) -> bool {
    #[cfg(target_os = "macos")]
    {
        path == Path::new("/var")
            && fs::read_link(path).is_ok_and(|target| target == Path::new("private/var"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

// Explicit error arms keep both fail-closed directory boundaries independently
// visible to the authoritative region-coverage gate.
#[allow(clippy::question_mark)]
pub(super) fn ensure_no_pending_effect(target: &Path) -> Result<()> {
    let parent = target
        .parent()
        .expect("canonical publication targets are rooted in the workspace");
    let target_name = target
        .file_name()
        .and_then(OsStr::to_str)
        .expect("validated record identifiers produce UTF-8 filenames");
    let prefix = format!(".{target_name}.");
    let entries = match map_io(fs::read_dir(parent), "inspect pending effects", parent) {
        Ok(entries) => entries,
        Err(error) => return Err(error),
    };
    for entry in entries {
        let entry = match map_io(entry, "inspect pending effect", parent) {
            Ok(entry) => entry,
            Err(error) => return Err(error),
        };
        let name = entry.file_name();
        #[cfg(target_os = "macos")]
        let name = name
            .to_str()
            .expect("macOS rejects non-UTF-8 filesystem names");
        #[cfg(not(target_os = "macos"))]
        let name = pending_entry_name(&name)?;
        if name.starts_with(&prefix) && name.ends_with(".tmp") {
            return Err(Error::AmbiguousEffect(format!(
                "interrupted publication found at {}; run 'research-run recover' before retry",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn pending_entry_name(name: &std::ffi::OsStr) -> Result<&str> {
    name.to_str()
        .ok_or_else(|| Error::invalid("pending effect", "filename is not UTF-8"))
}

pub(super) fn interrupted_target_name(name: &OsStr) -> Option<&str> {
    #[cfg(target_os = "macos")]
    let name = name
        .to_str()
        .expect("macOS rejects non-UTF-8 filesystem names");
    #[cfg(not(target_os = "macos"))]
    let name = name.to_str()?;
    let body = name.strip_prefix('.')?.strip_suffix(".tmp")?;
    let (without_sequence, sequence) = body.rsplit_once('.')?;
    if sequence.parse::<u64>().is_err() {
        return None;
    }
    let (target, process) = without_sequence.rsplit_once('.')?;
    if process.parse::<u32>().is_err() || !target.ends_with(".json") {
        return None;
    }
    Some(target)
}
