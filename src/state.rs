use std::time::Instant;

use crate::database::Database;

/// The server's global state.
pub struct ServerState {
    /// The instance to a database.
    pub database: Database,

    /// The instant at which the server is assumed to have started.
    pub uptime_start: Instant,
}
