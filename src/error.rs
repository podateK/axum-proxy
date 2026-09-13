use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("no available backends")]
    NoBackends,

    #[error("backend {0} is unhealthy")]
    BackendUnhealthy(String),

    #[error("request timeout after {0}s")]
    Timeout(u64),

    #[error("upstream error: {0}")]
    Upstream(String),

    #[error("rate limit exceeded")]
    RateLimited,

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("connection error: {0}")]
    Connection(String),

    #[error("request too large")]
    PayloadTooLarge,

    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ProxyError::NoBackends => (StatusCode::BAD_GATEWAY, self.to_string()),
            ProxyError::BackendUnhealthy(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            ProxyError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, self.to_string()),
            ProxyError::Upstream(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            ProxyError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            ProxyError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            ProxyError::Connection(_) => (StatusCode::BAD_GATEWAY, self.to_string()),
            ProxyError::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, self.to_string()),
            ProxyError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        let body = serde_json::json!({
            "error": message,
            "status": status.as_u16(),
        });

        (status, axum::Json(body)).into_response()
    }
}

impl From<reqwest::Error> for ProxyError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ProxyError::Timeout(30)
        } else if err.is_connect() {
            ProxyError::Connection(err.to_string())
        } else {
            ProxyError::Upstream(err.to_string())
        }
    }
}

impl From<axum::http::uri::InvalidUri> for ProxyError {
    fn from(err: axum::http::uri::InvalidUri) -> Self {
        ProxyError::Config(err.to_string())
    }
}
