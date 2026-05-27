//! This file implements the user-related routes on the HTTP server.

use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use serde::Deserialize;

use crate::{
    http_server::{ApiQuery, HttpServerModule, HttpServerResponse},
    state::{ServerState, UsernameAvailability},
};

#[derive(Deserialize)]
struct UsernameAvailabilityQuery {
    username: String,
}

pub struct AccountsModule;

impl HttpServerModule for AccountsModule {
    fn configure(router: Router<Arc<ServerState>>) -> Router<Arc<ServerState>> {
        router.route(
            "/accounts/username-availability",
            get(Self::username_availability),
        )
    }
}

impl AccountsModule {
    async fn username_availability(
        State(state): State<Arc<ServerState>>,
        ApiQuery(query): ApiQuery<UsernameAvailabilityQuery>,
    ) -> HttpServerResponse<Json<UsernameAvailability>> {
        Ok(Json(state.username_availability(&query.username).await?))
    }
}
