use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::domain::{MAX_INVENTORY_ENTRIES, MaterialClass, MaterialEntry};
use crate::{Error, Result};

use super::injected_storage_failure;
use super::path_safety::{absolute_path, reject_symlink_chain};
use super::storage::{map_io, same_file_identity};
#[cfg(all(any(test, coverage), unix))]
use super::take_storage_failure;

const MAX_INDEXED_FILE_BYTES: u64 = 64 * 1_048_576;
const MAX_INVENTORY_BYTES: u64 = 512 * 1_048_576;

pub(super) fn scan_materials(root: &Path) -> Result<(PathBuf, Vec<MaterialEntry>)> {
    let root = absolute_path(root)?;
    reject_symlink_chain(&root)?;
    if !root.is_dir() {
        return Err(Error::invalid("retrofit target", "must be a directory"));
    }
    let mut paths = Vec::new();
    collect_paths(&root, &root, &mut paths)?;
    paths.sort();
    let mut total = 0_u64;
    let mut entries = Vec::with_capacity(paths.len());
    for path in paths {
        let (bytes, sha256) = hash_file(&path)?;
        total = add_inventory_bytes(total, bytes)?;
        let relative = path.strip_prefix(&root).expect("collected path is rooted");
        let relative = (!injected_storage_failure("material path UTF-8"))
            .then(|| relative.to_str())
            .flatten()
            .ok_or_else(|| Error::invalid("material path", "must be UTF-8"))?
            .replace(std::path::MAIN_SEPARATOR, "/");
        entries.push(MaterialEntry {
            class: classify_material(&relative),
            path: relative,
            bytes,
            sha256,
        });
    }
    Ok((root, entries))
}

pub(super) fn enforce_file_count(count: usize) -> Result<()> {
    if count > MAX_INVENTORY_ENTRIES {
        return Err(Error::Budget(format!(
            "material inventory exceeds the {MAX_INVENTORY_ENTRIES} file budget"
        )));
    }
    Ok(())
}

pub(super) fn add_inventory_bytes(total: u64, bytes: u64) -> Result<u64> {
    let total = total
        .checked_add(bytes)
        .ok_or_else(|| Error::Budget("material inventory byte total overflowed".to_owned()))?;
    if total > MAX_INVENTORY_BYTES || injected_storage_failure("inventory byte budget") {
        return Err(Error::Budget(format!(
            "material inventory exceeds the {MAX_INVENTORY_BYTES} byte scan budget"
        )));
    }
    Ok(total)
}

fn collect_paths(root: &Path, directory: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    reject_symlink_chain(directory)?;
    let entries = map_io(
        fs::read_dir(directory),
        "read retrofit directory",
        directory,
    )?;
    for entry in entries {
        let entry = map_io(entry, "read retrofit entry", directory)?;
        let path = entry.path();
        let relative = path.strip_prefix(root).expect("entry is rooted");
        let metadata = map_io(fs::symlink_metadata(&path), "inspect retrofit entry", &path)?;
        if metadata.file_type().is_symlink() {
            return Err(Error::invalid(
                "retrofit target",
                format!("symlink is forbidden: {}", path.display()),
            ));
        }
        if relative.components().count() == 1
            && matches!(entry.file_name().to_str(), Some(".git" | ".research-run"))
        {
            continue;
        }
        if metadata.is_dir() {
            collect_paths(root, &path, paths)?;
        } else if metadata.is_file() {
            paths.push(path);
            enforce_file_count(paths.len())?;
        } else {
            return Err(Error::invalid(
                "retrofit target",
                format!("unsupported filesystem entry: {}", path.display()),
            ));
        }
    }
    Ok(())
}

pub(super) fn hash_file(path: &Path) -> Result<(u64, String)> {
    reject_material_path(path, "material pre-hash path")?;
    let before = map_io(fs::metadata(path), "inspect material", path)?;
    if !before.is_file() {
        return Err(Error::invalid(
            "material path",
            format!("must be a regular file: {}", path.display()),
        ));
    }
    if before.len() > MAX_INDEXED_FILE_BYTES {
        return Err(Error::Budget(format!(
            "{} exceeds the {MAX_INDEXED_FILE_BYTES} byte material budget",
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
            &mut file.by_ref().take(MAX_INDEXED_FILE_BYTES + 1),
            &mut hasher,
        ),
        "hash material",
        path,
    )?;
    if copied > MAX_INDEXED_FILE_BYTES || injected_storage_failure("material grew") {
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

pub(super) fn classify_material(path: &str) -> MaterialClass {
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
