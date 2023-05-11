use thiserror::Error;
use tonic::Status;

#[derive(Error, Debug)]
pub enum MeeseeksError {
    #[error("gRPC Error")]
    GrpcError(#[from] Status),

    #[error("failed to connect to agent")]
    ConnectionError(#[from] tonic::transport::Error),

    #[error("failed to execute task")]
    TaskExecutorError(String),
}

pub type Result<T> = std::result::Result<T, MeeseeksError>;
