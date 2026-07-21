use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::domain::MaterialClass;
use crate::{Error, Result};

use super::super::injected_storage_failure;
use super::super::path_safety::reject_symlink_chain;
use super::super::storage::{map_io, same_file_identity};
#[cfg(all(any(test, coverage), unix))]
use super::super::take_storage_failure;

pub(in crate::workspace) fn hash_file(path: &Path, maximum: u64) -> Result<(u64, String)> {
    reject_material_path(path, "material pre-hash path")?;
    let before = map_io(fs::metadata(path), "inspect material", path)?;
    if !before.is_file() {
        return Err(Error::invalid(
            "material path",
            format!("must be a regular file: {}", path.display()),
        ));
    }
    if before.len() > maximum {
        return Err(Error::Budget(format!(
            "{} exceeds the {maximum} byte material budget",
            path.display()
        )));
    }
    inject_material_open_race(path);
    let mut file = map_io(open_material(path), "open material", path)?;
    let opened = map_io(file.metadata(), "inspect opened material", path)?;
    if !opened.is_file() {
        return Err(Error::invalid(
            "material path",
            format!("must open as a regular file: {}", path.display()),
        ));
    }
    if !same_file_identity(&before, &opened) || injected_storage_failure("material opened identity")
    {
        return Err(Error::AmbiguousEffect(format!(
            "material identity changed while opening {}",
            path.display()
        )));
    }
    let mut hasher = Sha256::new();
    let copied = map_io(
        std::io::copy(
            &mut file.by_ref().take(maximum.saturating_add(1)),
            &mut hasher,
        ),
        "hash material",
        path,
    )?;
    if copied > maximum || injected_storage_failure("material grew") {
        return Err(Error::Budget(format!(
            "{} grew beyond its budget",
            path.display()
        )));
    }
    reject_material_path(path, "material post-hash path")?;
    let after = map_io(fs::metadata(path), "reinspect material", path)?;
    if !same_file_identity(&opened, &after)
        || before.len() != after.len()
        || copied != opened.len()
        || injected_storage_failure("material copied length")
        || injected_storage_failure("material identity")
    {
        return Err(Error::AmbiguousEffect(format!(
            "material identity changed while indexing {}",
            path.display()
        )));
    }
    Ok((copied, format!("{:x}", hasher.finalize())))
}

fn open_material(path: &Path) -> io::Result<File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(path)
    }
    #[cfg(not(unix))]
    OpenOptions::new().read(true).open(path)
}

#[cfg(all(any(test, coverage), unix))]
fn inject_material_open_race(path: &Path) {
    if take_storage_failure("material open fifo race") {
        fs::remove_file(path).expect("remove material for injected FIFO race");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(path)
                .status()
                .expect("create injected FIFO")
                .success()
        );
    }
}

#[cfg(not(all(any(test, coverage), unix)))]
fn inject_material_open_race(_path: &Path) {}

fn reject_material_path(path: &Path, fault: &str) -> Result<()> {
    if injected_storage_failure(fault) {
        return Err(Error::io(
            "inspect material path",
            path,
            std::io::Error::other("injected material path failure"),
        ));
    }
    reject_symlink_chain(path)
}

pub(in crate::workspace) fn classify_material(path: &str) -> MaterialClass {
    let lower = path.to_ascii_lowercase();
    let extension = Path::new(path).extension().and_then(OsStr::to_str);
    if lower.contains("protocol") || lower.contains("method") {
        MaterialClass::Protocol
    } else if lower.contains("source")
        || lower.contains("reference")
        || lower.contains("literature")
    {
        MaterialClass::Source
    } else if lower.contains("experiment") || lower.contains("run-") {
        MaterialClass::Experiment
    } else if lower.contains("observation") || lower.contains("result") {
        MaterialClass::Observation
    } else if lower.contains("analysis") {
        MaterialClass::Analysis
    } else if lower.contains("decision") {
        MaterialClass::Decision
    } else if lower.contains("plan") {
        MaterialClass::Plan
    } else if matches!(extension, Some("ppt" | "pptx" | "key")) || lower.contains("presentation") {
        MaterialClass::Presentation
    } else if matches!(extension, Some("md" | "txt" | "rst")) || lower.contains("note") {
        MaterialClass::Note
    } else if extension.is_some() {
        MaterialClass::Artifact
    } else {
        MaterialClass::Unknown
    }
}
