//! Server configuration types, shared state, and error mapping.

use axum::{
    Json,
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
};
use tesela_core::Error;
use tesela_runtime::{ports::ActorResolver, runtime::Runtime};
use std::sync::Arc;
use std::time::Duration;

/// CORS configuration.
#[derive(Default)]
pub struct CorsConfig {
    /// Allowed origins.  An empty list means allow all (`*`).
    pub allowed_origins: Vec<String>,
    /// Allowed HTTP methods.  Defaults to all.
    pub allowed_methods: Vec<Method>,
    /// Whether to allow credentials (cookies / authorization headers).
    pub allow_credentials: bool,
}

/// Configuration for the Tesela HTTP server.
pub struct ServerOptions {
    /// The runtime instance to delegate all operations to.
    pub runtime: Arc<Runtime>,
    /// Optional actor resolver for extracting authenticated actors from requests.
    pub actor_resolver: Option<Arc<dyn ActorResolver>>,
    /// Maximum request body size in bytes (default: 4 MiB).
    pub max_body_bytes: usize,
    /// Request timeout (default: 30 s).
    pub request_timeout: Duration,
    /// CORS configuration.  `None` = allow all origins.
    pub cors: Option<CorsConfig>,
}

impl ServerOptions {
    /// Create options with a runtime and defaults for all other settings.
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self {
            runtime,
            actor_resolver: None,
            max_body_bytes: 4 * 1024 * 1024,
            request_timeout: Duration::from_secs(30),
            cors: None,
        }
    }

    /// Set an actor resolver for authentication.
    pub fn with_actor_resolver(mut self, resolver: Arc<dyn ActorResolver>) -> Self {
        self.actor_resolver = Some(resolver);
        self
    }

    /// Restrict CORS to specific origins, methods, and credential policy.
    pub fn with_cors(mut self, cfg: CorsConfig) -> Self {
        self.cors = Some(cfg);
        self
    }

    /// Override the request body size limit.
    pub fn with_max_body_bytes(mut self, bytes: usize) -> Self {
        self.max_body_bytes = bytes;
        self
    }

    /// Override the per-request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) runtime: Arc<Runtime>,
    pub(crate) actor_resolver: Option<Arc<dyn ActorResolver>>,
}

pub(crate) struct ApiError(pub(crate) Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self.0 {
            Error::Validation { .. } | Error::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Error::NotFound { .. } => StatusCode::NOT_FOUND,
            Error::PolicyDenied { .. } => StatusCode::FORBIDDEN,
            Error::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            Error::Conflict { .. } => StatusCode::CONFLICT,
            Error::Timeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            Error::UnsupportedCapability { .. } => StatusCode::NOT_IMPLEMENTED,
            Error::Adapter { .. } => StatusCode::BAD_GATEWAY,
            Error::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let body = serde_json::json!({ "error": self.0.to_string() });
        (status, Json(body)).into_response()
    }
}

impl From<Error> for ApiError {
    fn from(e: Error) -> Self {
        ApiError(e)
    }
}
