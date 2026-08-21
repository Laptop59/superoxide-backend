//! This file implements the related routes for the My Tests on the HTTP server.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    routing::{delete, get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use uuid::Uuid;

use crate::{
    database::test::TestType,
    http_server::{
        HttpServer, HttpServerError, HttpServerModule, HttpServerResponse, HttpSuccess, Session,
    },
    state::ServerState,
};

#[derive(Serialize, Deserialize)]
pub(crate) struct MyTestsResponse {
    pub tests: Vec<TestEntry>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct TestEntry {
    pub id: Uuid,

    pub name: String,

    #[serde(rename = "type")]
    pub test_type: TestType,

    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CreateTestRequest {
    pub name: String,

    #[serde(rename = "type")]
    pub test_type: TestType,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct CreateTestResponse {
    pub id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct DeleteTestRequest {
    pub id: Uuid,
}

pub struct TestsModule;

impl HttpServerModule for TestsModule {
    fn configure(router: Router<Arc<ServerState>>) -> Router<Arc<ServerState>> {
        let my_tests_router = Router::new().route("/my-tests", get(my_tests)).layer(
            GovernorLayer::new(
                GovernorConfigBuilder::default()
                    .per_millisecond(200)
                    .burst_size(10)
                    .finish()
                    .expect("governor should have been built properly"),
            )
            .error_handler(HttpServer::too_many_requests_handler),
        );

        let create_test_router = Router::new().route("/create", post(create_test)).layer(
            GovernorLayer::new(
                GovernorConfigBuilder::default()
                    .per_millisecond(1000)
                    .burst_size(10)
                    .finish()
                    .expect("governor should have been built properly"),
            )
            .error_handler(HttpServer::too_many_requests_handler),
        );

        let delete_test_router = Router::new().route("/delete", delete(delete_test)).layer(
            GovernorLayer::new(
                GovernorConfigBuilder::default()
                    .per_millisecond(1000)
                    .burst_size(10)
                    .finish()
                    .expect("governor should have been built properly"),
            )
            .error_handler(HttpServer::too_many_requests_handler),
        );

        router.nest(
            "/tests",
            my_tests_router
                .merge(create_test_router)
                .merge(delete_test_router),
        )
    }
}

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

async fn create_test(
    State(state): State<Arc<ServerState>>,
    session: Session,
    Json(query): Json<CreateTestRequest>,
) -> HttpServerResponse<Json<CreateTestResponse>> {
    let id = Uuid::new_v4();
    state
        .database
        .create_test(session.user_id, &query.name, query.test_type, id)
        .await?;

    Ok(Json(CreateTestResponse { id }))
}

async fn delete_test(
    State(state): State<Arc<ServerState>>,
    session: Session,
    Json(query): Json<DeleteTestRequest>,
) -> HttpServerResponse<Json<HttpSuccess>> {
    let result = state
        .database
        .delete_test(session.user_id, query.id)
        .await?;

    if result {
        Ok(Json(HttpSuccess::Successful))
    } else {
        Err(HttpServerError::NotFound)
    }
}
