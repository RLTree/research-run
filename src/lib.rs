#![forbid(unsafe_code)]

pub mod cli;
pub mod domain;
pub mod workspace;

use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Invalid {
        context: String,
        reason: String,
    },
    MalformedJson {
        path: PathBuf,
    },
    NotFound(String),
    Conflict(String),
    AmbiguousEffect(String),
    Budget(String),
}

impl Error {
    pub fn io(action: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            action,
            path: path.into(),
            source,
        }
    }

    pub fn invalid(context: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Invalid {
            context: context.into(),
            reason: reason.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                action,
                path,
                source,
            } => write!(formatter, "cannot {action} {}: {source}", path.display()),
            Self::Invalid { context, reason } => write!(formatter, "invalid {context}: {reason}"),
            Self::MalformedJson { path } => {
                write!(formatter, "malformed JSON in {}", path.display())
            }
            Self::NotFound(message) => formatter.write_str(message),
            Self::Conflict(message) => formatter.write_str(message),
            Self::AmbiguousEffect(message) => formatter.write_str(message),
            Self::Budget(message) => formatter.write_str(message),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
