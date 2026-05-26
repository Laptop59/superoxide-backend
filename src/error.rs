use std::borrow::Cow;

use thiserror::Error;

/// Represents an overall result of the Superoxide server.
pub type SuperoxideResult<T> = Result<T, SuperoxideError>;

/// An error that can arise during startup or during normal operation.
/// It can be fatal, severe, or mostly harmless.
///
/// Each error type is assigned a unique error code.
#[derive(Debug, Error)]
#[repr(i32)]
pub enum SuperoxideError {
    // Zero is not assigned because that should be returned for a success.
    /// Generic error that is not helpful.
    #[error("Generic error")]
    Generic = 1,

    /// A startup error when the credentials were not set.
    #[error(
        "The DATABASE_URL environment variable has not been set. It must be a URL pointing to a MySql/MariaDB database."
    )]
    UnsetCredentials = 2,

    /// Could not create a connection to the database.
    #[error("Could not connect to the database: {0}")]
    FailedDatabaseConnection(sqlx::Error) = 3,

    /// Failed to parse the .env file successfully.
    #[error("Could not load the .env file: {0}")]
    FailedToParseEnv(dotenvy::Error) = 4,

    /// Could not bind the HTTP server successfully.
    #[error("Could not bind HTTP server to address {0}: {1}")]
    BindingHTTPServerFailed(Cow<'static, str>, std::io::Error) = 5,

    /// Database migration failed.
    #[error("Could not migrate your database to the latest: {0}")]
    DatabaseMigrationFailed(sqlx::migrate::MigrateError) = 6,

    /// A startup error when the credentials contained invalid Unicode.
    #[error(
        "The DATABASE_URL environment variable contains invalid Unicode data. Please make it valid."
    )]
    InvalidUnicodeCredentials = 7,
}

impl SuperoxideError {
    pub fn exit_code(&self) -> i32 {
        // SAFETY: SuperoxideError is marked `repr(i32)`, so it should be totally
        // safe to cast the discriminant to an integer equivalent.
        unsafe { *std::ptr::from_ref::<Self>(self).cast::<i32>() }
    }
}
