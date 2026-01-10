// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

/// Errors that can occur during persistence operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    /// A database error occurred.
    DatabaseError(String),
    /// The requested event was not found.
    EventNotFound(i64),
    /// The requested snapshot was not found.
    SnapshotNotFound { bid_year: u16, area: String },
    /// A state reconstruction error occurred.
    ReconstructionError(String),
    /// Serialization/deserialization error.
    SerializationError(String),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            Self::EventNotFound(id) => write!(f, "Event not found: {id}"),
            Self::SnapshotNotFound { bid_year, area } => {
                write!(f, "Snapshot not found for bid_year={bid_year}, area={area}")
            }
            Self::ReconstructionError(msg) => write!(f, "State reconstruction error: {msg}"),
            Self::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
        }
    }
}

impl std::error::Error for PersistenceError {}

impl From<rusqlite::Error> for PersistenceError {
    fn from(err: rusqlite::Error) -> Self {
        Self::DatabaseError(err.to_string())
    }
}

impl From<serde_json::Error> for PersistenceError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}
