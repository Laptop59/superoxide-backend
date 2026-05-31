//! This file implements the session-related functions on the database.

use crate::database::{Database, DatabaseError};

impl Database {
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

    /// Attempts to get a user from a given session token. Returns their internal ID.
    /// If a session is found, the session will be updated.
    pub async fn find_session(&self, token: &str) -> Result<Option<u64>, DatabaseError> {
        let query_result = sqlx::query!(
            r#"
            SELECT user_id from sessions
            WHERE token = ?
            "#,
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        if query_result.is_some() {
            sqlx::query!(
                r#"
                UPDATE sessions
                SET updated_at = UTC_TIMESTAMP()
                WHERE token = ?
                "#,
                token
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(query_result.map(|record| record.user_id))
    }
}
