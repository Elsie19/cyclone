use thiserror::Error;

#[derive(Debug, Error)]
pub enum GetOrParseError {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
}
