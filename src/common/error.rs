use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
