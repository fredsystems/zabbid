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
    /// Initialization error (e.g., foreign key enforcement not enabled).
    InitializationError(String),
    /// The requested operator was not found.
    OperatorNotFound(String),
    /// The requested session was not found.
    SessionNotFound(String),
    /// Session has expired.
    SessionExpired(String),
    /// Operator cannot be deleted because it is referenced by audit events.
    OperatorReferenced { operator_id: i64 },
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
            Self::InitializationError(msg) => write!(f, "Initialization error: {msg}"),
            Self::OperatorNotFound(msg) => write!(f, "Operator not found: {msg}"),
            Self::SessionNotFound(msg) => write!(f, "Session not found: {msg}"),
            Self::SessionExpired(msg) => write!(f, "Session expired: {msg}"),
            Self::OperatorReferenced { operator_id } => {
                write!(
                    f,
                    "Operator {operator_id} cannot be deleted: referenced by audit events"
                )
            }
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
