use axum::http::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageProcessingError {
    #[error("Failed to decode image: {0}")]
    DecodeError(String),
    #[error("Failed to encode image: {0}")]
    EncodeError(String),
    #[error("Image not found: {0}")]
    NotFound(String),
    #[error("Internal processing error: {0}")]
    InternalError(String),
}

impl ImageProcessingError {
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        match self {
            ImageProcessingError::NotFound(_) => StatusCode::NOT_FOUND,
            ImageProcessingError::DecodeError(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("IO Error: {0}")]
    IoError(String),
}

impl StorageError {
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        match self {
            StorageError::NotFound(_) => StatusCode::NOT_FOUND,
            StorageError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
