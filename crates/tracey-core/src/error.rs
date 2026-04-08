use thiserror::Error;

#[derive(Error, Debug)]
pub enum TraceyError {
    #[error("config error: {0}")]
    Config(String),

    #[error("llm error: {0}")]
    Llm(String),

    #[error("tool error: {0}")]
    Tool(String),

    #[error("graph error: {0}")]
    Graph(String),

    #[error("session error: {0}")]
    Session(String),

    #[error("sandbox error: {0}")]
    Sandbox(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("interrupted")]
    Interrupted,

    #[error("timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

pub type TraceyResult<T> = Result<T, TraceyError>;
