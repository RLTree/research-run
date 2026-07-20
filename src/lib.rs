#![forbid(unsafe_code)]

pub mod cli;
pub mod domain;
mod error;
pub mod workspace;

pub use error::{Error, Result};
