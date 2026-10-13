use thiserror::Error;

#[derive(Error, Debug)]
pub enum HarnessError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Required component missing: {0}")]
    ComponentMissing(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Verification failed with exit code: {0}")]
    VerificationFailed(i32),
}
