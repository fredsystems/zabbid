// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Diesel-based schema migration management.
//!
//! This module handles database schema initialization and migration
//! via Diesel. It is used at server startup and in test infrastructure
//! to bootstrap databases.
//!
//! Runtime queries continue to use the existing persistence layer.

use diesel::{Connection, RunQueryDsl, SqliteConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tracing::info;

use crate::error::PersistenceError;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Run pending migrations on the provided connection.
///
/// This function applies all pending migrations to bring the database
/// schema up to date.
///
/// # Arguments
///
/// * `conn` - A mutable reference to a Diesel `SqliteConnection`
///
/// # Errors
///
/// Returns an error if migration execution fails.
pub fn run_migrations(
    conn: &mut SqliteConnection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Running database migrations");
    conn.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

/// Initialize a `SQLite` database at the given path and run migrations.
///
/// # Arguments
///
/// * `database_url` - The `SQLite` database URL (e.g., `":memory:"` or `file path`)
///
/// # Errors
///
/// Returns an error if connection or migration fails.
pub fn initialize_database(database_url: &str) -> Result<SqliteConnection, PersistenceError> {
    info!("Initializing database at: {}", database_url);

    let mut conn: SqliteConnection = SqliteConnection::establish(database_url)
        .map_err(|e| PersistenceError::DatabaseConnectionFailed(e.to_string()))?;

    // Enable foreign key enforcement
    diesel::sql_query("PRAGMA foreign_keys = ON")
        .execute(&mut conn)
        .map_err(|e| PersistenceError::QueryFailed(e.to_string()))?;

    run_migrations(&mut conn).map_err(|e| PersistenceError::MigrationFailed(e.to_string()))?;

    Ok(conn)
}
