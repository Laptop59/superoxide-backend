use std::{sync::Arc, time::Instant};

use crate::{
    database::Database,
    error::{SuperoxideError, SuperoxideResult},
    http_server::HttpServer,
    state::ServerState,
};

pub mod database;
pub mod error;
pub mod http_server;
pub mod logger;
pub mod state;

#[tokio::main]
async fn main() {
    match init().await {
        Ok(()) => {
            tracing::info!("The server has stopped.");
            std::process::exit(0);
        }
        Err(error) => {
            tracing::error!("{error}");
            tracing::info!("The server has stopped due to a fatal error.");
            std::process::exit(error.exit_code());
        }
    };
}

async fn init() -> SuperoxideResult<()> {
    let uptime_start = Instant::now();

    logger::init();

    tracing::info!("Starting Superoxide {}...", env!("CARGO_PKG_VERSION"));

    try_load_dotenv_file()?;
    let database = Database::create().await?;
    database.migrate().await?;

    let server_state = Arc::new(ServerState {
        database,
        uptime_start,
    });

    let http_server = HttpServer::new(server_state.clone());
    http_server.run().await?;

    Ok(())
}

fn try_load_dotenv_file() -> SuperoxideResult<()> {
    match dotenvy::dotenv() {
        Ok(load) => {
            tracing::info!("Loaded environment variables from {}.", load.display());
            Ok(())
        }
        Err(error) => {
            if error.not_found() {
                tracing::debug!("Could not find a .env file to load environment variables from.");
                Ok(())
            } else {
                Err(SuperoxideError::FailedToParseEnv(error))
            }
        }
    }
}
