//! This file implements the user-related functions on the database.

use crate::database::{Database, DatabaseError};

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

    /// Returns whether the user with the given name exists.
    pub async fn user_exists(&self, username: &str) -> Result<bool, DatabaseError> {
        self.fetch_user_id(username).await.map(|id| id.is_some())
    }
}
