//! Errors for use within this crate.
use std::io;

/// The error type used ubiquitously within this crate.
#[derive(thiserror::Error, Debug)]
pub enum LaxError {
    /// Glob pattern has zero potential matches
    #[error("Could not find file or directory: {0}")]
    EntityNotFound(String),
    /// Selector is malformed or out-of-range.
    #[error("Invalid Selector: {0}")]
    InvalidSelector(String),
    /// Generic IO error.
    #[error("{0}")]
    IoError(#[from] io::Error),
    /// Generic Globset error.
    #[error("{0}")]
    GlobError(#[from] globset::Error),
}

/// The result type used ubiquitously within this crate.
pub type LaxResult<T = ()> = Result<T, LaxError>;
