//! This file implements the user-related routes on the HTTP server.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    http_server::{
        ApiQuery, FrontendUserDetails, HttpServer, HttpServerModule, HttpServerResponse,
        HttpSuccess, Session,
    },
    state::{AccountLoginResult, AccountRegistrationResult, ServerState, UsernameAvailability},
};

#[derive(Deserialize)]
struct UsernameAvailabilityQuery {
    username: String,
}

#[derive(Deserialize)]
struct LoginRequestBody {
    username: String,
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
enum RegisterResponse {
    Successful { user: FrontendUserDetails },
    UsernameAlreadyTaken,
    InvalidUsername,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
enum LoginResponse {
    Successful { user: FrontendUserDetails },
    IncorrectUsernameOrPassword,
}

pub struct AccountsModule;

impl HttpServerModule for AccountsModule {
    fn configure(router: Router<Arc<ServerState>>) -> Router<Arc<ServerState>> {
        // GET routes
        let get_router = Router::new()
            .route("/username-availability", get(username_availability))
            .layer(
                GovernorLayer::new(
                    GovernorConfigBuilder::default()
                        .per_millisecond(200)
                        .burst_size(20)
                        .finish()
                        .expect("governor should have been built properly"),
                )
                .error_handler(HttpServer::too_many_requests_handler),
            );

        // Register route
        let register_router = Router::new().route("/register", post(register)).layer(
            GovernorLayer::new(
                GovernorConfigBuilder::default()
                    .per_second(1)
                    .burst_size(3)
                    .finish()
                    .expect("governor should have been built properly"),
            )
            .error_handler(HttpServer::too_many_requests_handler),
        );

        // Login route
        let login_router = Router::new().route("/login", post(login)).layer(
            GovernorLayer::new(
                GovernorConfigBuilder::default()
                    .per_millisecond(600)
                    .burst_size(5)
                    .finish()
                    .expect("governor should have been built properly"),
            )
            .error_handler(HttpServer::too_many_requests_handler),
        );

        // DELETE routes
        let delete_router = Router::new().route("/sign-out", delete(sign_out)).layer(
            GovernorLayer::new(
                GovernorConfigBuilder::default()
                    .per_millisecond(100)
                    .burst_size(20)
                    .finish()
                    .expect("governor should have been built properly"),
            )
            .error_handler(HttpServer::too_many_requests_handler),
        );

        router.nest(
            "/accounts",
            get_router
                .merge(register_router)
                .merge(login_router)
                .merge(delete_router),
        )
    }
}

async fn username_availability(
    State(state): State<Arc<ServerState>>,
    ApiQuery(query): ApiQuery<UsernameAvailabilityQuery>,
) -> HttpServerResponse<Json<UsernameAvailability>> {
    Ok(Json(state.username_availability(&query.username).await?))
}

async fn register(
    State(state): State<Arc<ServerState>>,
    Json(query): Json<LoginRequestBody>,
) -> HttpServerResponse<Response> {
    Ok(
        match state
            .register_account(&query.username, query.password)
            .await?
        {
            AccountRegistrationResult::Successful(user_id) => {
                send_new_session(
                    &state,
                    user_id,
                    RegisterResponse::Successful {
                        user: state.get_frontend_user_details(user_id).await?,
                    },
                )
                .await?
            }
            AccountRegistrationResult::UsernameAlreadyTaken => (
                StatusCode::CONFLICT,
                Json(RegisterResponse::UsernameAlreadyTaken),
            )
                .into_response(),
            AccountRegistrationResult::InvalidUsername => (
                StatusCode::BAD_REQUEST,
                Json(RegisterResponse::InvalidUsername),
            )
                .into_response(),
        },
    )
}

async fn login(
    State(state): State<Arc<ServerState>>,
    Json(query): Json<LoginRequestBody>,
) -> HttpServerResponse<Response> {
    Ok(
        match state.login_account(&query.username, query.password).await? {
            AccountLoginResult::Successful(user_id) => {
                send_new_session(
                    &state,
                    user_id,
                    LoginResponse::Successful {
                        user: state.get_frontend_user_details(user_id).await?,
                    },
                )
                .await?
            }
            AccountLoginResult::IncorrectUsernameOrPassword => (
                StatusCode::UNAUTHORIZED,
                Json(LoginResponse::IncorrectUsernameOrPassword),
            )
                .into_response(),
        },
    )
}

async fn sign_out(
    State(state): State<Arc<ServerState>>,
    session: Session,
) -> HttpServerResponse<Json<HttpSuccess>> {
    state.revoke_session(session.session_id).await?;
    Ok(Json(HttpSuccess::Successful))
}

async fn send_new_session<T>(
    state: &ServerState,
    user_id: u64,
    success: T,
) -> HttpServerResponse<Response>
where
    Json<T>: IntoResponse,
{
    let token = state.create_session(user_id).await?;
    let mut response = Json(success).into_response();

    // Tell the browser to set the session token.
    let cookie_value: HeaderValue = format!("session={token}; HttpOnly; Path=/; SameSite=Lax")
        .parse()
        .expect("somehow created an invalid header value");

    response
        .headers_mut()
        .insert(header::SET_COOKIE, cookie_value);

    Ok(response)
}
