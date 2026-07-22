use serde_json::Error as SerdeError;
use std::io::Error as IoError;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to read file")]
    ReadError(#[from] IoError),
    #[error("failed to deserialise data")]
    DeserializeError(#[from] SerdeError),
}
