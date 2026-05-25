use std::env::VarError;

use sqlx::{MySqlPool, mysql::MySqlPoolOptions};

use crate::error::{SuperoxideError, SuperoxideResult};

/// Represents an abstraction around the database connection
/// that keeps the internal queries and statements required hidden.
#[derive(Debug, Clone)]
pub struct Database {
    pool: MySqlPool,
}

impl Database {
    /// Attempts to initialize a pool of connections to a database and returns that.
    pub async fn create() -> SuperoxideResult<Self> {
        match Self::fetch_credentials_for_url() {
            Ok(database_url) => {
                let pool = MySqlPoolOptions::new()
                    .max_connections(10)
                    .connect(&database_url)
                    .await;

                match pool {
                    Ok(pool) => {
                        tracing::info!("Successfully connected to the required database.");
                        Ok(Database { pool })
                    }
                    Err(error) => {
                        tracing::error!("Could not connect to the database: {error}");
                        Err(SuperoxideError::FailedDatabaseConnection)
                    }
                }
            },
            Err(VarError::NotPresent) => {
                tracing::error!("The DATABASE_URL environment variable has not been set. It must be a URL pointing to a MySql/MariaDB database.");
                Err(SuperoxideError::ImproperCredentials)
            },
            Err(VarError::NotUnicode(_)) => {
                tracing::error!("The DATABASE_URL environment variable contains invalid Unicode data. Please make it valid UTF-8.");
                Err(SuperoxideError::ImproperCredentials)
            }
        }
    }

    /// Fetches all the required database credentials,
    /// failing if any one of them don't exist.
    ///
    /// Returns a URL of the database to connect to it if successful.
    fn fetch_credentials_for_url() -> Result<String, VarError> {
        std::env::var("DATABASE_URL")
    }

    /// Updates tables so that the schema is the latest.
    pub async fn migrate(&self) -> Result<(), SuperoxideError> {
        // We will initialize some tables for the user.
        // To create new entries, we just need to run `sqlx migrate add <migration_name>`.
        if let Err(error) = sqlx::migrate!().run(&self.pool).await {
            tracing::error!("Could not migrate your database to the latest: {error}");
            Err(SuperoxideError::DatabaseMigrationFailed)
        } else {
            tracing::info!("Database schema is confirmed to be up to date.");
            Ok(())
        }
    }
}
