mod accounts;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{FromRequestParts, Query, State},
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use colored::Colorize;
use serde::de::DeserializeOwned;
use serde_json::json;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::{
    database::DatabaseError,
    error::{SuperoxideError, SuperoxideResult},
    http_server::accounts::AccountsModule,
    state::ServerState,
};

pub const FRONTEND_ORIGIN: HeaderValue = HeaderValue::from_static("http://localhost:5173");

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
        let cors = CorsLayer::new()
            .allow_origin(FRONTEND_ORIGIN)
            .allow_methods([
                Method::GET,
                Method::POST
            ]);

        let app = Router::new()
            .route("/", get(Self::status))
            .register::<AccountsModule>()
            .fallback(Self::not_found)
            .layer(cors)
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

    async fn not_found() -> HttpServerError {
        HttpServerError::NotFound
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
    InvalidParameters,
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
            Self::InvalidParameters => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "invalid_parameters"
                })),
            )
                .into_response(),
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

/// Represent query parameters used in HTTP requests.
/// This will give its own kind of query error, so that's
/// why this one is preferred over [`Query`]; it does not
/// expose internal server details.
pub struct ApiQuery<T>(pub T);

impl<S, T> FromRequestParts<S> for ApiQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = HttpServerError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        match Query::<T>::from_request_parts(parts, state).await {
            Ok(Query(value)) => Ok(ApiQuery(value)),
            Err(_) => Err(HttpServerError::InvalidParameters),
        }
    }
}
