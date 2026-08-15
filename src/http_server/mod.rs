mod accounts;
mod me;
mod tests;

use std::{net::SocketAddr, sync::Arc};

use axum::{
    Json, Router,
    extract::{FromRequestParts, Query, State},
    http::{HeaderValue, Method, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use axum_extra::extract::CookieJar;
use colored::Colorize;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use tokio::net::TcpListener;
use tower_governor::GovernorError;
use tower_http::cors::CorsLayer;

use crate::{
    database::{self, DatabaseError},
    error::{SuperoxideError, SuperoxideResult},
    http_server::{accounts::AccountsModule, me::MeModule, tests::TestsModule},
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
            .allow_credentials(true)
            .allow_methods([Method::GET, Method::POST, Method::DELETE])
            .allow_headers([header::CONTENT_TYPE]);

        let app = Router::new()
            .route("/", get(Self::status))
            .register::<AccountsModule>()
            .register::<MeModule>()
            .register::<TestsModule>()
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

        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("HTTP server returned an error when it should not");

        Ok(())
    }

    async fn not_found() -> HttpServerError {
        HttpServerError::NotFound
    }

    fn too_many_requests_handler(_: GovernorError) -> Response {
        HttpServerError::TooManyRequests.into_response()
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
    InternalServerError(SuperoxideError),
    InvalidParameters,
    TooManyRequests,
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
            Self::InternalServerError(error) => {
                tracing::error!(
                    "Internal server error encountered while serving HTTP request: {error}"
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
            Self::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "status": "too_many_requests"
                })),
            )
                .into_response(),
        }
    }
}

impl From<SuperoxideError> for HttpServerError {
    fn from(value: SuperoxideError) -> Self {
        HttpServerError::InternalServerError(value)
    }
}

impl From<DatabaseError> for HttpServerError {
    fn from(value: DatabaseError) -> Self {
        HttpServerError::InternalServerError(SuperoxideError::DatabaseError(value))
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

/// Represents a valid user's session.
/// If this is received in a handler, it can be assumed that
/// the authentication stage has been passed for determining the user.
pub struct Session {
    user_id: u64,
    session_id: u64,
}

impl FromRequestParts<Arc<ServerState>> for Session {
    type Rejection = HttpServerError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(token) = jar.get("session") else {
            return Err(HttpServerError::Unauthorized);
        };
        let token = token.value();

        if token.len() != 64 {
            return Err(HttpServerError::Unauthorized);
        }

        let mut token_bytes: [u8; 32] = [0; 32];
        if hex::decode_to_slice(token, &mut token_bytes).is_err() {
            return Err(HttpServerError::Unauthorized);
        };

        let token_hash: [u8; 32] = ServerState::hash_token_bytes(&token_bytes);

        // Find a session from this token.
        let Some(database::session::Session {
            user_id,
            session_id,
        }) = state.database.find_and_update_session(&token_hash).await?
        else {
            return Err(HttpServerError::Unauthorized);
        };

        Ok(Self {
            user_id,
            session_id,
        })
    }
}

/// The subset of user details sent to the frontend
/// so that it can know who it is representing.
#[derive(Serialize)]
pub struct FrontendUserDetails {
    pub username: String,
}

/// An enum whose only way to express success is `Successful`, without any data.
#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum HttpSuccess {
    Successful,
}
