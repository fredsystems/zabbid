// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Database backend-specific code.
//!
//! This module isolates backend-specific initialization, migration,
//! and helper functions that cannot be expressed in backend-agnostic
//! Diesel DSL.
//!
//! ## Backend Support
//!
//! - `sqlite` — `SQLite` backend (default for development and testing)
//! - `mysql` — MySQL/MariaDB backend (validated via opt-in tests)
//!
//! ## Backend-Agnostic Code
//!
//! Most persistence code should be backend-agnostic and use Diesel DSL.
//! Backend-specific code is limited to:
//!
//! - Connection initialization
//! - Migration execution
//! - Backend-specific configuration (e.g., PRAGMA, engine settings)
//! - Backend-specific workarounds for missing Diesel DSL features
//!
//! All domain queries and mutations live in `queries/` and `mutations/`
//! modules and must work across all supported backends.

pub mod mysql;
pub mod sqlite;

use diesel::{Connection, MysqlConnection, SqliteConnection};

use crate::error::PersistenceError;

/// Trait for backend-specific operations.
///
/// This trait provides a unified interface for operations that cannot be
/// expressed in backend-agnostic Diesel DSL, such as retrieving the last
/// inserted row ID or verifying foreign key enforcement.
///
/// This trait is implemented for both `SqliteConnection` and `MysqlConnection`,
/// allowing query and mutation functions to be generic over backend type
/// while maintaining a single implementation.
pub trait PersistenceBackend: Connection {
    /// Retrieves the last inserted row ID.
    ///
    /// This is needed because Diesel's `RETURNING` clause support varies
    /// across backends, and some operations require the inserted ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    fn get_last_insert_rowid(&mut self) -> Result<i64, PersistenceError>;

    /// Verifies that foreign key enforcement is enabled.
    ///
    /// This is a startup-time check to ensure referential integrity
    /// constraints are enforced by the database backend.
    ///
    /// # Errors
    ///
    /// Returns an error if foreign key enforcement is not enabled.
    fn verify_foreign_key_enforcement(&mut self) -> Result<(), PersistenceError>;
}

impl PersistenceBackend for SqliteConnection {
    fn get_last_insert_rowid(&mut self) -> Result<i64, PersistenceError> {
        sqlite::get_last_insert_rowid(self)
    }

    fn verify_foreign_key_enforcement(&mut self) -> Result<(), PersistenceError> {
        sqlite::verify_foreign_key_enforcement(self)
    }
}

impl PersistenceBackend for MysqlConnection {
    fn get_last_insert_rowid(&mut self) -> Result<i64, PersistenceError> {
        mysql::get_last_insert_rowid(self)
    }

    fn verify_foreign_key_enforcement(&mut self) -> Result<(), PersistenceError> {
        mysql::verify_foreign_key_enforcement(self)
    }
}
