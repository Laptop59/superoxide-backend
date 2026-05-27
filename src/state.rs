use serde::Serialize;
use std::time::Instant;

use crate::database::{Database, DatabaseError};

/// The server's global state.
pub struct ServerState {
    /// The instance to a database.
    pub database: Database,

    /// The instant at which the server is assumed to have started.
    pub uptime_start: Instant,
}

/// Tells how a username is invalid.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UsernameValidationError {
    TooShort,
    TooLong,
    InvalidCharacters,
    ContainsSpaces
}
/// Tells the state of the availability of a username.
#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "reason")]
pub enum UsernameAvailability {
    Available,
    AlreadyTaken,
    Invalid(UsernameValidationError),
}

impl ServerState {
    /// Takes a username and returns if it is valid for a user to exist with the given name.
    /// If invalid, it tells how it is invalid. Note that this does not tell if the username is already taken or not.
    pub fn validate_username(username: &str) -> Result<(), UsernameValidationError> {
        match username.len() {
            0..=2 => Err(UsernameValidationError::TooShort),
            3..=32 => {
                for byte in username.as_bytes() {
                    match byte {
                        b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'-' | b'.' => (),
                        b' ' => return Err(UsernameValidationError::ContainsSpaces),
                        _ => return Err(UsernameValidationError::InvalidCharacters),
                    }
                }
                Ok(())
            }
            33.. => Err(UsernameValidationError::TooLong),
        }
    }

    /// Takes a username and returns if it is available, already taken or invalid.
    /// Do not use this to check if a username is available when creating a user, as that
    /// can lead to data races.
    pub async fn username_availability(
        &self,
        username: &str,
    ) -> Result<UsernameAvailability, DatabaseError> {
        Ok(
            if let Err(validation_error) = Self::validate_username(username) {
                UsernameAvailability::Invalid(validation_error)
            } else if self.database.user_exists(username).await? {
                UsernameAvailability::AlreadyTaken
            } else {
                UsernameAvailability::Available
            },
        )
    }
}
