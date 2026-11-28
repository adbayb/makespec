use std::{io, path::PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MakespecError {
    #[error("{0}")]
    Invalid(String),
    #[error("project is not initialized; run `makespec init` first")]
    NotInitialized,
    #[error("document already exists: {0}")]
    AlreadyExists(PathBuf),
    #[error("document changed while it was being edited: {0}")]
    ConcurrentModification(PathBuf),
    #[error("document not found: {0}")]
    NotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid YAML front matter: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),
}

pub type Result<T> = std::result::Result<T, MakespecError>;
