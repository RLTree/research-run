#[cfg(test)]
use std::path::Path;

use crate::workspace::Workspace;
use crate::{Error, Result};

pub(super) fn discover_current() -> Result<Workspace> {
    #[cfg(test)]
    let current = if let Some(path) = take_current_directory_override() {
        Ok(path)
    } else if take_current_directory_failure() {
        Err(std::io::Error::other("injected current directory failure"))
    } else {
        std::env::current_dir()
    };
    #[cfg(all(coverage, not(test)))]
    let current = if take_current_directory_failure() {
        Err(std::io::Error::other("injected current directory failure"))
    } else {
        std::env::current_dir()
    };
    #[cfg(not(any(test, coverage)))]
    let current = std::env::current_dir();
    match current {
        Ok(current) => Workspace::discover(&current),
        Err(error) => Err(Error::io("read current directory", ".", error)),
    }
}

#[cfg(test)]
thread_local! {
    static CURRENT_DIRECTORY_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CURRENT_DIRECTORY_OVERRIDE: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn inject_current_directory_failure() {
    CURRENT_DIRECTORY_FAILURE.set(true);
}

#[cfg(test)]
pub(super) fn inject_current_directory(path: &Path) {
    CURRENT_DIRECTORY_OVERRIDE.with_borrow_mut(|current| *current = Some(path.to_path_buf()));
}

#[cfg(test)]
fn take_current_directory_override() -> Option<std::path::PathBuf> {
    CURRENT_DIRECTORY_OVERRIDE.with_borrow_mut(Option::take)
}

#[cfg(test)]
fn take_current_directory_failure() -> bool {
    CURRENT_DIRECTORY_FAILURE.replace(false)
}

#[cfg(all(coverage, not(test)))]
fn take_current_directory_failure() -> bool {
    std::env::var("RESEARCH_RUN_COVERAGE_FAULT").as_deref() == Ok("current directory")
}
