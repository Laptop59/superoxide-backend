use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use colored::Colorize;
use serde_json::json;
use tokio::net::TcpListener;

use crate::{
    error::{SuperoxideError, SuperoxideResult},
    state::ServerState,
};

/// The Superoxide subsystem that handles HTTP requests
/// for API usage.
pub struct HttpServer {
    server_state: Arc<ServerState>,
}

impl HttpServer {
    /// Creates a new [`HttpServer`] from an instance of global server state.
    pub fn new(server_state: Arc<ServerState>) -> Self {
        Self { server_state }
    }

    pub async fn run(self) -> SuperoxideResult<()> {
        let app = Router::new()
            .route("/", get(Self::status))
            .with_state(self.server_state);

        let http_addr = "localhost:8080";
        let listener = TcpListener::bind(http_addr).await.map_err(|error| {
            tracing::error!("Could not bind HTTP server to address {http_addr}: {error}");
            SuperoxideError::BindingHTTPServerFailed
        })?;

        tracing::info!(
            "Started the HTTP server! It can be connected to with the address: {}",
            format!("http://{http_addr}").underline().bold()
        );

        axum::serve(listener, app)
            .await
            .expect("HTTP server returned an error when it should not");

        Ok(())
    }
}

impl HttpServer {
    async fn status(State(state): State<Arc<ServerState>>) -> Json<serde_json::Value> {
        let uptime = state.uptime_start.elapsed().as_secs_f64();
        Json(json!({
            "ok": true,
            "uptime": uptime
        }))
    }
}
