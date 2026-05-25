/// Represents an overall result of the Superoxide server.
pub type SuperoxideResult<T> = Result<T, SuperoxideError>;

/// An error that can arise during startup or during normal operation.
/// It can be fatal, severe, or mostly harmless.
///
/// Each error type is assigned a unique error code.
#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum SuperoxideError {
    // Zero is not assigned because that should be returned for a success.
    /// Generic error that is not helpful.
    Generic = 1,

    /// A startup error when the credentials could not be fetched properly.
    ImproperCredentials = 2,

    /// Could not create a connection to the database.
    FailedDatabaseConnection = 3,

    /// Failed to parse the .env file successfully.
    FailedToParseEnv = 4,

    /// Could not bind the HTTP server successfully.
    BindingHTTPServerFailed = 5,

    /// Database migration failed.
    DatabaseMigrationFailed = 6,
}
