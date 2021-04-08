use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LaxError {
    #[error("Could not find file or directory: {0}")]
    EntityNotFound(String),

    #[error("{0}")]
    IoError(#[from] io::Error),
}

pub type LaxResult<T = ()> = Result<T, LaxError>;
