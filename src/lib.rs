#![forbid(unsafe_code)]

pub mod cli;
pub mod domain;
mod error;
pub mod workspace;

pub use error::{Error, Result};

pub(crate) const MAX_STRUCTURED_INPUT_BYTES: u64 = 1_048_576;
