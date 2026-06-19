use thiserror::Error;

pub type Result<T, E = TossError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum TossError {
    #[error("config error: {0}")]
    Config(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("auth error: {0}")]
    Auth(String),
    #[error("api error: {message}")]
    Api {
        status: Option<u16>,
        code: Option<String>,
        message: String,
        request_id: Option<String>,
    },
    #[error("rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after: Option<String>,
        request_id: Option<String>,
    },
    #[error("runtime error: {0}")]
    Runtime(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
}
