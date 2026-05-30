//! This file implements the user-related functions on the database.

use crate::database::{Database, DatabaseError};

pub struct UserLogin {
    pub id: u64,
    pub password_hash: String,
}

impl Database {
    /// Returns the user ID from the name of a user.
    pub async fn fetch_user_id(&self, username: &str) -> Result<Option<u64>, DatabaseError> {
        let query_result = sqlx::query!(
            r#"
            SELECT id from users
            WHERE username = ?
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await;

        query_result
            .map(|option| option.map(|record| record.id))
            .map_err(Into::into)
    }

    /// Returns a struct useful for logging in a user from their name.
    pub async fn fetch_user_login_from_username(
        &self,
        username: &str,
    ) -> Result<Option<UserLogin>, DatabaseError> {
        let query_result = sqlx::query!(
            r#"
            SELECT id, password_hash from users
            WHERE username = ?
            "#,
            username
        )
        .fetch_optional(&self.pool)
        .await;

        query_result
            .map(|option| {
                option.map(|record| UserLogin {
                    id: record.id,
                    password_hash: record.password_hash,
                })
            })
            .map_err(Into::into)
    }

    /// Returns whether the user with the given name exists.
    pub async fn user_exists(&self, username: &str) -> Result<bool, DatabaseError> {
        self.fetch_user_id(username).await.map(|id| id.is_some())
    }

    /// Attempts to create an account with the given username and hashed password.
    /// If the user is successfully registered, an ID to that user is returned.
    /// If a user with the given name already exists, `Ok(None)` is returned.
    pub async fn create_account(
        &self,
        username: &str,
        password_hash: &str,
    ) -> Result<Option<u64>, DatabaseError> {
        let query_result = sqlx::query!(
            r#"
            INSERT INTO users (
                username,
                password_hash,
                created_at,
                roles
            ) VALUES (?, ?, UTC_TIMESTAMP(), ?)
            "#,
            username,
            password_hash,
            0
        )
        .execute(&self.pool)
        .await;

        match query_result {
            Ok(insertion) => Ok(Some(insertion.last_insert_id())),
            Err(error) => {
                if let sqlx::Error::Database(ref error) = error
                    && error.is_unique_violation()
                {
                    // This means that a username with the specified username already exists.
                    Ok(None)
                } else {
                    Err(error.into())
                }
            }
        }
    }

    /// Attempts to create a session to the database for a given user.
    pub async fn create_session(&self, user_id: u64, token: &str) -> Result<(), DatabaseError> {
        sqlx::query!(
            r#"
            INSERT INTO sessions (
                token,
                user_id,
                created_at,
                updated_at
            ) VALUES (?, ?, UTC_TIMESTAMP(), UTC_TIMESTAMP())
            "#,
            token,
            user_id
        )
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(Into::into)
    }
}
