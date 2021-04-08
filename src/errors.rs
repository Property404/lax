use std::io;
use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum LaxError {
    #[error("Could not find file or directory: {0}")]
    EntityNotFound(String),

    #[error("{0}")]
    IoError(#[from] io::Error),

    #[error("{0}")]
    GlobError(#[from] globset::Error),
}

pub type LaxResult<T = ()> = Result<T, LaxError>;
