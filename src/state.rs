use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{
        SaltString,
        rand_core::{OsRng, RngCore},
    },
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{borrow::Cow, time::Instant};

use crate::{
    database::Database,
    error::{SuperoxideError, SuperoxideResult},
    http_server::{FrontendUserDetails, HttpServerError},
};

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
    ContainsSpaces,
}

/// Tells the state of the availability of a username.
#[derive(Serialize)]
#[serde(rename_all = "snake_case", tag = "status", content = "reason")]
pub enum UsernameAvailability {
    Available,
    AlreadyTaken,
    Invalid(UsernameValidationError),
}

/// Tells the result of an attempt to create an account.
/// Should not be given to the front-end as it contains
/// the internal user ID.
pub enum AccountRegistrationResult {
    Successful(u64),
    UsernameAlreadyTaken,
    InvalidUsername,
}

/// Tells the result of an attempt to log into an account.
pub enum AccountLoginResult {
    Successful(u64),
    IncorrectUsernameOrPassword,
}

/// Tells that the password provided was invalid.
pub struct InvalidPassword;

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

    pub fn validate_password(password: &str) -> Result<(), InvalidPassword> {
        const MIN_LENGTH: usize = 12;
        const MAX_LENGTH: usize = 256;

        // Length restriction.
        if !(MIN_LENGTH..=MAX_LENGTH).contains(&password.len()) {
            return Err(InvalidPassword);
        }

        Ok(())
    }

    /// Takes a username and returns if it is available, already taken or invalid.
    /// Do not use this to check if a username is available when creating a user, as that
    /// can lead to data races.
    pub async fn username_availability(
        &self,
        username: &str,
    ) -> SuperoxideResult<UsernameAvailability> {
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

    pub async fn hash_password(password: impl Into<Cow<'_, str>>) -> SuperoxideResult<String> {
        let password = password.into().into_owned();

        tokio::task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();

            argon2
                .hash_password(password.as_bytes(), &salt)
                .map(|hash| hash.to_string())
                .map_err(SuperoxideError::HashingError)
        })
        .await
        .expect("hashing thread shouldn't have panicked")
    }

    pub async fn verify_password(
        password: impl Into<Cow<'_, str>>,
        password_hash: impl Into<Cow<'_, str>>,
    ) -> SuperoxideResult<bool> {
        let password = password.into().into_owned();
        let password_hash = password_hash.into().into_owned();

        tokio::task::spawn_blocking(move || {
            let parsed_hash = PasswordHash::new(&password_hash)?;
            let argon2 = Argon2::default();

            match argon2.verify_password(password.as_bytes(), &parsed_hash) {
                Ok(()) => Ok(true),
                Err(argon2::password_hash::Error::Password) => Ok(false),
                Err(error) => Err(SuperoxideError::HashingError(error)),
            }
        })
        .await
        .expect("hashing thread shouldn't have panicked")
    }

    /// Takes a username and password and attempts to create an account with them.
    /// Checks for username validity.
    /// **Does not create a new session.**
    pub async fn register_account(
        &self,
        username: &str,
        password: impl Into<Cow<'_, str>>,
    ) -> SuperoxideResult<AccountRegistrationResult> {
        if Self::validate_username(username).is_err() {
            return Ok(AccountRegistrationResult::InvalidUsername);
        }

        let password_hash = Self::hash_password(password).await?;
        let Some(user_id) = self
            .database
            .create_account(username, &password_hash)
            .await?
        else {
            return Ok(AccountRegistrationResult::UsernameAlreadyTaken);
        };

        Ok(AccountRegistrationResult::Successful(user_id))
    }

    /// Takes a username and password and attempts to log in an account from the given credentials.
    /// **Does not create a new session.**
    pub async fn login_account(
        &self,
        username: &str,
        password: impl Into<Cow<'_, str>>,
    ) -> SuperoxideResult<AccountLoginResult> {
        let Some(user_login) = self
            .database
            .fetch_user_login_from_username(username)
            .await?
        else {
            return Ok(AccountLoginResult::IncorrectUsernameOrPassword);
        };

        if Self::verify_password(password, user_login.password_hash).await? {
            Ok(AccountLoginResult::Successful(user_login.id))
        } else {
            Ok(AccountLoginResult::IncorrectUsernameOrPassword)
        }
    }

    /// Creates a session for the user.
    pub async fn create_session(&self, user_id: u64) -> SuperoxideResult<Box<str>> {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);

        let token_hash = Self::hash_token_bytes(&bytes);

        let token = hex::encode(bytes);
        self.database.create_session(user_id, &token_hash).await?;
        Ok(token.into_boxed_str())
    }

    /// Revokes a session from a user. Does not actually tell if the token got revoked or not.
    pub async fn revoke_session(&self, session_id: u64) -> SuperoxideResult<()> {
        self.database.revoke_session(session_id).await?;
        Ok(())
    }

    pub async fn get_frontend_user_details(
        &self,
        user_id: u64,
    ) -> Result<FrontendUserDetails, HttpServerError> {
        let user_details = self
            .database
            .fetch_user_details(user_id)
            .await?
            .ok_or(HttpServerError::Unauthorized)?;

        Ok(FrontendUserDetails {
            username: user_details.username,
        })
    }

    /// Uses SHA-256 hashing on token bytes to get a hash that is one-way.
    pub fn hash_token_bytes(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }
}
