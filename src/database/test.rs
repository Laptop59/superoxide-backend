//! This file implements the test-related functions on the database.
//! This includes, but is not limited to, tests, sections, parts, and questions.

use std::fmt::Display;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::database::{Database, DatabaseError};

/// Represents the type of a test.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestType {
    Objective,
    Subjective,
}

/// Represents an error due to specifying an invalid test type.
#[derive(Debug, Error)]
#[error("Invalid test type: {0}")]
pub struct InvalidTestType(String);

impl TryFrom<String> for TestType {
    type Error = InvalidTestType;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "objective" => Ok(Self::Objective),
            "subjective" => Ok(Self::Subjective),
            _ => Err(InvalidTestType(value)),
        }
    }
}

impl TestType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Objective => "objective",
            Self::Subjective => "subjective",
        }
    }
}

impl Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Represents a test on the surface. It contains the test's:
/// - public ID,
/// - name,
/// - type, and
/// - last updated date
#[derive(Debug)]
pub struct TestEntry {
    pub public_id: Uuid,
    pub name: String,
    pub test_type: TestType,
    pub updated_at: NaiveDateTime,
}

impl Database {
    /// Fetches a list of tests of a user.
    pub async fn query_tests(&self, user_id: u64) -> Result<Vec<TestEntry>, DatabaseError> {
        let entries = sqlx::query!(
            r#"
                SELECT public_id, name, type AS test_type, updated_at FROM tests
                WHERE created_by = ?
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| TestEntry {
            public_id: Uuid::from_slice(&row.public_id)
                .expect("database contains bytes that cannot be converted into a UUID"),
            name: row.name,
            test_type: row
                .test_type
                .try_into()
                .expect("database contains invalid test type"),
            updated_at: row.updated_at,
        })
        .collect();

        Ok(entries)
    }

    /// Creates a test with some initial data. If successful, this function returns the newly created test's internal ID.
    pub async fn create_test(
        &self,
        user_id: u64,
        name: &str,
        test_type: TestType,
        public_id: Uuid,
    ) -> Result<u64, DatabaseError> {
        Ok(sqlx::query!(
            r#"
                    INSERT INTO tests (
                        name,
                        type,
                        public_id,
                        created_by,
                        created_at,
                        updated_at
                    ) VALUES (?, ?, ?, ?, UTC_TIMESTAMP(), UTC_TIMESTAMP())
                "#,
            name,
            test_type.as_str(),
            public_id.as_bytes().as_slice(),
            user_id,
        )
        .execute(&self.pool)
        .await?
        .last_insert_id())
    }
}
