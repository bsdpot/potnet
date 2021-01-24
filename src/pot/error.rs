use thiserror::Error;

#[derive(Debug, Error)]
pub enum PotError {
    #[error("Command {0} not found")]
    WhichError(String),
    #[error("Invalid UTF-8 string")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Invalid Path {0} - no parent")]
    PathError(String),
    #[error("Error during a file operation")]
    FileError(#[from] std::io::Error),
    #[error("jls failed")]
    JlsError,
    #[error("Invalid bridge configuration")]
    BridgeConfError,
}
