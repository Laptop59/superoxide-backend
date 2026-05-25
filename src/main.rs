use std::{sync::Arc, time::Instant};

use colored::Colorize;

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
    let exit_code = match init().await {
        Ok(()) => 0,
        Err(error) => error as i32,
    };
    tracing::info!("The server has stopped.");
    std::process::exit(exit_code);
}

async fn init() -> SuperoxideResult<()> {
    let uptime_start = Instant::now();

    logger::init();

    tracing::info!(
        "Starting Superoxide {}...",
        env!("CARGO_PKG_VERSION").bold()
    );

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
        Ok(load) => tracing::info!("Loaded environment variables from {}.", load.display()),
        Err(error) => {
            if error.not_found() {
                tracing::debug!("Could not find a .env file to load environment variables from.");
            } else {
                tracing::error!(
                    "Could not load the .env file: {error}. The server will stop now to prevent unintended errors later down the line."
                );
                return Err(SuperoxideError::FailedToParseEnv);
            }
        }
    }
    Ok(())
}
