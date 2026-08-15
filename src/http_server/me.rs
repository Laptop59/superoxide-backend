//! This file implements the `/me` route on the HTTP server.
//! This lets the server tell the client who it is.

use std::sync::Arc;

use axum::{Json, Router, extract::State, routing::get};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};

use crate::{
    http_server::{FrontendUserDetails, HttpServer, HttpServerModule, HttpServerResponse, Session},
    state::ServerState,
};

pub struct MeModule;

impl HttpServerModule for MeModule {
    fn configure(router: Router<Arc<ServerState>>) -> Router<Arc<ServerState>> {
        let me_router = Router::new().route("/me", get(Self::me)).layer(
            GovernorLayer::new(
                GovernorConfigBuilder::default()
                    .per_millisecond(150)
                    .burst_size(20)
                    .finish()
                    .expect("governor should have been built properly"),
            )
            .error_handler(HttpServer::too_many_requests_handler),
        );

        router.merge(me_router)
    }
}

impl MeModule {
    async fn me(
        State(state): State<Arc<ServerState>>,
        session: Session,
    ) -> HttpServerResponse<Json<FrontendUserDetails>> {
        let user_details = state.get_frontend_user_details(session.user_id).await?;
        Ok(Json(FrontendUserDetails {
            username: user_details.username,
        }))
    }
}
