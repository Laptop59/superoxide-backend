//! This file implements the related routes for the My Tests on the HTTP server.

use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use uuid::Uuid;

use crate::{
    database::test::TestType,
    http_server::{HttpServer, HttpServerModule, HttpServerResponse, Session},
    state::ServerState,
};

#[derive(Serialize, Deserialize)]
pub struct MyTestsResponse {
    pub tests: Vec<TestEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct TestEntry {
    pub id: Uuid,
    pub name: String,
    pub test_type: TestType,
    pub updated_at: DateTime<Utc>,
}

pub struct TestsModule;

impl HttpServerModule for TestsModule {
    fn configure(router: Router<Arc<ServerState>>) -> Router<Arc<ServerState>> {
        // My Tests route
        let get_router = Router::new().route("/my-tests", get(my_tests)).layer(
            GovernorLayer::new(
                GovernorConfigBuilder::default()
                    .per_millisecond(200)
                    .burst_size(10)
                    .finish()
                    .expect("governor should have been built properly"),
            )
            .error_handler(HttpServer::too_many_requests_handler),
        );

        router.nest("/tests", get_router)
    }
}

#[axum::debug_handler]
async fn my_tests(
    State(state): State<Arc<ServerState>>,
    session: Session,
) -> HttpServerResponse<Json<MyTestsResponse>> {
    let tests = state
        .database
        .query_tests(session.user_id)
        .await?
        .into_iter()
        .map(|test| TestEntry {
            id: test.public_id,
            name: test.name,
            test_type: test.test_type,
            updated_at: test.updated_at.and_utc(),
        })
        .collect();

    Ok(Json(MyTestsResponse { tests }))
}
