#![deny(warnings)]
#![deny(missing_docs)]

//! HTTP REST server for Lattice runtime.
//!
//! Provides an Axum-based HTTP server exposing all Lattice runtime operations
//! through a versioned REST API. Authentication is handled via an optional
//! [`ActorResolver`] middleware.

mod handlers;
mod types;

pub use types::{CorsConfig, ServerOptions, TlsConfig};

use axum::{
    http::{HeaderValue, StatusCode},
    routing::{delete, get, post, put},
    Router,
};
use std::net::SocketAddr;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    timeout::TimeoutLayer,
};
use tracing::info;

use handlers::*;
use types::AppState;

/// The Lattice HTTP server.
pub struct Server {
    opts: ServerOptions,
}

impl Server {
    /// Create a new server with the given options.
    pub fn new(opts: ServerOptions) -> Self {
        Self { opts }
    }

    /// Build the Axum router without binding.
    pub fn router(&self) -> Router {
        let state = AppState {
            runtime: self.opts.runtime.clone(),
            actor_resolver: self.opts.actor_resolver.clone(),
        };

        let cors = self.build_cors_layer();

        Router::new()
            .route("/v1/objects/:type_name/search", post(search_handler))
            .route("/v1/objects/:type_name/:pk", get(get_handler))
            .route("/v1/objects/:type_name/mutate", post(mutate_handler))
            .route("/v1/objects/:type_name/aggregate", post(aggregate_handler))
            .route("/v1/objects/:type_name/upload", post(upload_handler))
            .route("/v1/objects/:type_name/rollback", post(rollback_handler))
            .route("/v1/objects/:type_name/explain", post(explain_handler))
            .route("/v1/objects/:type_name/subscribe", get(subscribe_handler))
            .route(
                "/v1/objects/:type_name/vector-search",
                post(vector_search_handler),
            )
            .route("/v1/objects/:type_name/:pk/lineage", get(lineage_handler))
            .route("/v1/actions/:name", post(action_handler))
            .route("/v1/artifacts/:name/read", post(artifact_read_handler))
            .route("/v1/upload-flows/:name", post(upload_flow_handler))
            .route(
                "/v1/upload-flows/:name/complete",
                post(upload_flow_complete_handler),
            )
            .route(
                "/v1/upload-flows/:name/load",
                post(upload_flow_load_handler),
            )
            .route(
                "/v1/upload-flows/:name/rollback",
                post(upload_flow_rollback_handler),
            )
            .route(
                "/v1/capability-grants/:name/issue",
                post(capability_issue_handler),
            )
            .route("/v1/jobs/:name/start", post(job_start_handler))
            .route("/v1/runs/:run_id", get(run_get_handler))
            .route("/v1/aggregate-views/:name", get(aggregate_view_handler))
            .route("/v1/agents/:name", post(agent_start_handler))
            .route("/v1/agents/:name/:run_id", get(agent_get_run_handler))
            .route("/v1/links/:name/traverse", post(traverse_handler))
            .route("/v1/links/:name/explain", post(explain_traverse_handler))
            .route("/v1/object-sets/:name", get(object_set_resolve_handler))
            .route(
                "/v1/object-sets/:name/compose",
                post(object_set_compose_handler),
            )
            .route(
                "/v1/pipelines/:name/execute",
                post(pipeline_execute_handler),
            )
            .route("/v1/search/federated", post(federated_search_handler))
            .route("/v1/branches", post(branch_create_handler))
            .route("/v1/branches", get(branch_list_handler))
            .route("/v1/branches/:id", put(branch_update_handler))
            .route("/v1/branches/:id/merge", post(branch_merge_handler))
            .route("/v1/branches/:id", delete(branch_discard_handler))
            .route("/v1/spec", get(spec_handler))
            .route("/v1/ontology/apply", post(apply_spec_handler))
            .route("/v1/capabilities", get(capabilities_handler))
            .route("/v1/health", get(health_handler))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::GATEWAY_TIMEOUT,
                self.opts.request_timeout,
            ))
            .layer(RequestBodyLimitLayer::new(self.opts.max_body_bytes))
            .layer(cors)
            .with_state(state)
    }

    fn build_cors_layer(&self) -> CorsLayer {
        match &self.opts.cors {
            None => CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
            Some(cfg) => {
                let mut layer = CorsLayer::new();

                if cfg.allowed_origins.is_empty() {
                    layer = layer.allow_origin(Any);
                } else {
                    let origins: Vec<HeaderValue> = cfg
                        .allowed_origins
                        .iter()
                        .filter_map(|o| o.parse().ok())
                        .collect();
                    layer = layer.allow_origin(origins);
                }

                if cfg.allowed_methods.is_empty() {
                    layer = layer.allow_methods(Any);
                } else {
                    layer = layer.allow_methods(cfg.allowed_methods.clone());
                }

                layer = layer.allow_headers(Any);

                if cfg.allow_credentials {
                    layer = layer.allow_credentials(true);
                }

                layer
            }
        }
    }

    /// Start the server and listen on the given address (e.g. `"0.0.0.0:8080"`).
    pub async fn serve(self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let addr: SocketAddr = addr.parse()?;
        let router = self.router();
        info!("Lattice HTTP server listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await?;
        Ok(())
    }
}
