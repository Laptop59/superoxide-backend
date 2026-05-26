//! This file implements the user-related routes on the HTTP server.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    http_server::{HttpServerModule, HttpServerResponse},
    state::ServerState,
};

#[derive(Deserialize)]
struct UsernameAvailabilityQuery {
    username: String,
}

pub struct UserModule;

impl HttpServerModule for UserModule {
    fn configure(router: Router<Arc<ServerState>>) -> Router<Arc<ServerState>> {
        router.route(
            "/users/username-availability",
            get(Self::username_availability),
        )
    }
}

impl UserModule {
    async fn username_availability(
        State(state): State<Arc<ServerState>>,
        Query(query): Query<UsernameAvailabilityQuery>,
    ) -> HttpServerResponse<Json<serde_json::Value>> {
        let exists = state
            .database
            .user_exists(&query.username)
            .await?;
        Ok(Json(json!({
            "available": !exists
        })))
    }
}
