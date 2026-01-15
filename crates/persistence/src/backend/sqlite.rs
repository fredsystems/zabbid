// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! SQLite-specific backend utilities.
//!
//! This module contains SQLite-specific initialization, migration,
//! and helper functions that cannot be expressed in backend-agnostic
//! Diesel DSL.
//!
//! ## Backend-Specific Code
//!
//! This module is limited to:
//! - Connection initialization
//! - Migration execution
//! - SQLite-specific configuration (PRAGMA statements)
//! - SQLite-specific workarounds (e.g., `last_insert_rowid()`)
//!
//! All domain queries and mutations must remain backend-agnostic
//! and live in `queries/` or `mutations/` modules.

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer};
use diesel::{Connection, RunQueryDsl, SqliteConnection};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tracing::info;

use crate::error::PersistenceError;

/// SQLite-specific migrations.
///
/// These migrations use `SQLite` syntax and are the default for development
/// and standard testing.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Helper row struct for PRAGMA queries.
///
/// This is a justified use of raw SQL as Diesel has no PRAGMA DSL.
#[derive(QueryableByName)]
struct PragmaRow {
    #[diesel(sql_type = Integer)]
    foreign_keys: i32,
}

/// Helper function to get the last inserted row ID.
///
/// `SQLite` doesn't support `RETURNING` clauses in all contexts,
/// so we must query `last_insert_rowid()`.
///
/// This is a justified use of raw SQL as Diesel has no direct API for this.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn get_last_insert_rowid(conn: &mut SqliteConnection) -> Result<i64, PersistenceError> {
    Ok(diesel::select(sql::<BigInt>("last_insert_rowid()")).get_result(conn)?)
}

/// Verifies that foreign key enforcement is enabled.
///
/// This function checks whether `SQLite` has foreign key enforcement active.
/// If foreign keys are not enabled, the database cannot guarantee referential
/// integrity constraints required by the system.
///
/// # Arguments
///
/// * `conn` - The database connection to check
///
/// # Errors
///
/// Returns an error if foreign key enforcement is not enabled.
pub fn verify_foreign_key_enforcement(conn: &mut SqliteConnection) -> Result<(), PersistenceError> {
    // NOTE: PRAGMA is raw SQL (justified - Diesel has no PRAGMA DSL)
    let foreign_keys_enabled: i32 = diesel::sql_query("PRAGMA foreign_keys")
        .get_result::<PragmaRow>(conn)?
        .foreign_keys;

    if foreign_keys_enabled == 0 {
        return Err(PersistenceError::ForeignKeyEnforcementNotEnabled);
    }

    info!("SQLite foreign key enforcement is enabled");
    Ok(())
}

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
    info!("Running SQLite database migrations");
    conn.run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

/// Initialize a `SQLite` database at the given URL and run migrations.
///
/// # Arguments
///
/// * `database_url` - The `SQLite` database URL (e.g., `":memory:"` or file path)
///
/// # Errors
///
/// Returns an error if connection or migration fails.
pub fn initialize_database(database_url: &str) -> Result<SqliteConnection, PersistenceError> {
    info!("Initializing SQLite database at: {}", database_url);

    let mut conn: SqliteConnection = SqliteConnection::establish(database_url)
        .map_err(|e| PersistenceError::DatabaseConnectionFailed(e.to_string()))?;

    // Enable foreign key enforcement
    // NOTE: PRAGMA is raw SQL (justified - Diesel has no PRAGMA DSL)
    diesel::sql_query("PRAGMA foreign_keys = ON")
        .execute(&mut conn)
        .map_err(|e| PersistenceError::QueryFailed(e.to_string()))?;

    run_migrations(&mut conn).map_err(|e| PersistenceError::MigrationFailed(e.to_string()))?;

    Ok(conn)
}

/// Enable WAL mode for file-based `SQLite` databases.
///
/// WAL (Write-Ahead Logging) mode provides better read concurrency
/// for file-based databases.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the PRAGMA statement fails.
pub fn enable_wal_mode(conn: &mut SqliteConnection) -> Result<(), PersistenceError> {
    // NOTE: PRAGMA is raw SQL (justified - Diesel has no PRAGMA DSL)
    diesel::sql_query("PRAGMA journal_mode = WAL")
        .execute(conn)
        .map_err(|e| PersistenceError::QueryFailed(e.to_string()))?;
    Ok(())
}
