//! This file implements the session-related functions on the database.

use crate::database::{Database, DatabaseError};

pub struct Session {
    pub session_id: u64,
    pub user_id: u64,
}

impl Database {
    /// Attempts to create a session to the database for a given user.
    /// Returns the ID of the session created.
    pub async fn create_session(
        &self,
        user_id: u64,
        token_hash: &[u8],
    ) -> Result<u64, DatabaseError> {
        let query_result = sqlx::query!(
            r#"
            INSERT INTO sessions (
                token_hash,
                user_id,
                created_at,
                updated_at
            ) VALUES (?, ?, UTC_TIMESTAMP(), UTC_TIMESTAMP())
            "#,
            token_hash,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(query_result.last_insert_id())
    }

    /// Attempts to revoke a session from the database from its token.
    /// If successful, returns `Ok(true)`. If such a token was not found, returns `Ok(false)`.
    pub async fn revoke_session(&self, session_id: u64) -> Result<bool, DatabaseError> {
        let query_result = sqlx::query!(
            r#"
            DELETE FROM sessions
            WHERE id = ?
            "#,
            session_id
        )
        .execute(&self.pool)
        .await?;

        Ok(query_result.rows_affected() > 0)
    }

    /// Attempts to get a user from a given session token. Returns the user id and session id.
    /// If a session is found, the session will be updated.
    pub async fn find_and_update_session(
        &self,
        token_hash: &[u8],
    ) -> Result<Option<Session>, DatabaseError> {
        let Some(query_result) = sqlx::query!(
            r#"
            SELECT id, user_id from sessions
            WHERE token_hash = ?
            "#,
            token_hash
        )
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        sqlx::query!(
            r#"
            UPDATE sessions
            SET updated_at = UTC_TIMESTAMP()
            WHERE token_hash = ?
            "#,
            token_hash
        )
        .execute(&self.pool)
        .await?;

        Ok(Some(Session {
            user_id: query_result.user_id,
            session_id: query_result.id,
        }))
    }
}
