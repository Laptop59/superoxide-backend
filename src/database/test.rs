//! This file implements the test-related functions on the database.
//! This includes, but is not limited to, tests, sections, parts, and questions.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::database::{Database, DatabaseError};

/// Represents the type of a test.
#[derive(Debug, Serialize, Deserialize)]
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
}
