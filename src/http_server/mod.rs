mod user;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use colored::Colorize;
use serde_json::json;
use tokio::net::TcpListener;

use crate::{
    database::DatabaseError,
    error::{SuperoxideError, SuperoxideResult},
    http_server::user::UserModule,
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
            .register::<UserModule>()
            .with_state(self.server_state);

        let http_addr = "localhost:8080";
        let listener = TcpListener::bind(http_addr)
            .await
            .map_err(|error| SuperoxideError::BindingHTTPServerFailed(http_addr.into(), error))?;

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

pub type HttpServerResponse<T> = Result<T, HttpServerError>;

pub enum HttpServerError {
    NotFound,
    Unauthorized,
    InternalServerError,
    DatabaseError(DatabaseError),
}

impl IntoResponse for HttpServerError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "status": "not_found"
                })),
            )
                .into_response(),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "status": "unauthorized"
                })),
            )
                .into_response(),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "internal_server_error"
                })),
            )
                .into_response(),
            Self::DatabaseError(database_error) => {
                tracing::error!(
                    "Database error encountered while serving HTTP request: {database_error}"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "status": "internal_server_error"
                    })),
                )
                    .into_response()
            }
        }
    }
}

impl From<DatabaseError> for HttpServerError {
    fn from(value: DatabaseError) -> Self {
        HttpServerError::DatabaseError(value)
    }
}

/// Represents one part of the HTTP server that can configure its own routes.
pub trait HttpServerModule {
    /// Adds new routes from this module.
    fn configure(router: Router<Arc<ServerState>>) -> Router<Arc<ServerState>>;
}

trait HttpServerModuleRegister {
    fn register<M: HttpServerModule>(self) -> Self;
}

impl HttpServerModuleRegister for Router<Arc<ServerState>> {
    fn register<M: HttpServerModule>(self) -> Self {
        M::configure(self)
    }
}
