// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! MySQL/MariaDB-specific persistence utilities.
//!
//! ## Purpose
//!
//! This module provides connection initialization and validation for MySQL/MariaDB
//! database backends. It exists solely to support **explicit, opt-in backend validation**,
//! not for production runtime use.
//!
//! ## Usage
//!
//! This module is used exclusively by backend validation tests marked with `#[ignore]`.
//! These tests are executed only via `cargo xtask test-mariadb`, which:
//!
//! 1. Starts a `MariaDB` container via Docker
//! 2. Sets required environment variables (`DATABASE_URL`, `ZABBID_TEST_BACKEND`)
//! 3. Runs ignored tests explicitly
//! 4. Stops and removes the container
//!
//! ## Compilation Requirements
//!
//! `MySQL` support is compiled by default (no feature flags).
//! Compilation requires:
//!
//! - `MySQL` client development libraries (`libmysqlclient-dev` or equivalent)
//! - `pkg-config` for library detection
//!
//! These are provided by the Nix development environment (`flake.nix`).
//!
//! ## Backend Compatibility
//!
//! All Diesel migrations and queries must work correctly on both `SQLite` and `MySQL`.
//! This module does NOT introduce MySQL-specific schema or behavior.
//! If a query or migration cannot be expressed in backend-agnostic Diesel DSL,
//! stop and ask for guidance.
//!
//! ## Testing Philosophy
//!
//! - `SQLite` remains the default backend for all standard tests
//! - `MySQL` validation is intentional and explicit, never automatic
//! - Tests fail fast if required infrastructure is missing
//! - No test silently skips due to missing services
//!
//! See `tests/backend_validation_tests.rs` for validation test examples.

use diesel::sql_types::Integer;
use diesel::{Connection, MysqlConnection, QueryableByName, RunQueryDsl};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tracing::info;

use crate::error::PersistenceError;

/// Result type for foreign key check query.
#[derive(QueryableByName)]
#[allow(dead_code)]
struct ForeignKeyCheck {
    #[diesel(sql_type = Integer)]
    fk_checks: i32,
}

/// MySQL-specific migrations.
///
/// These migrations are functionally equivalent to the `SQLite` migrations
/// but use MySQL-compatible syntax (e.g., `AUTO_INCREMENT` instead of `AUTOINCREMENT`,
/// `BIGINT` instead of `INTEGER`, `VARCHAR` instead of `TEXT` where appropriate).
#[allow(dead_code)]
pub const MYSQL_MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations_mysql");

/// Initialize a `MySQL` database at the given URL and run migrations.
///
/// This function:
/// - Establishes a connection to MySQL/MariaDB
/// - Runs all pending migrations
/// - Returns the initialized connection
///
/// # Arguments
///
/// * `database_url` - The `MySQL` connection URL (e.g., `mysql://user:pass@host/db`)
///
/// # Errors
///
/// Returns an error if connection or migration fails.
#[allow(dead_code)]
pub fn initialize_database(database_url: &str) -> Result<MysqlConnection, PersistenceError> {
    info!("Initializing MySQL database at: {}", database_url);

    let mut conn: MysqlConnection = MysqlConnection::establish(database_url)
        .map_err(|e| PersistenceError::DatabaseConnectionFailed(e.to_string()))?;

    run_migrations(&mut conn).map_err(|e| PersistenceError::MigrationFailed(e.to_string()))?;

    Ok(conn)
}

/// Run pending migrations on the provided `MySQL` connection.
///
/// This function applies all pending migrations to bring the database
/// schema up to date.
///
/// # Arguments
///
/// * `conn` - A mutable reference to a Diesel `MysqlConnection`
///
/// # Errors
///
/// Returns an error if migration execution fails.
#[allow(dead_code)]
pub fn run_migrations(
    conn: &mut MysqlConnection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Running MySQL database migrations");
    conn.run_pending_migrations(MYSQL_MIGRATIONS)?;
    Ok(())
}

/// Verify that foreign key enforcement is enabled on `MySQL`.
///
/// `MySQL` enforces foreign keys by default when using `InnoDB` engine.
/// This function validates the engine and foreign key support.
///
/// # Errors
///
/// Returns an error if verification fails.
#[allow(dead_code)]
pub fn verify_foreign_key_enforcement(conn: &mut MysqlConnection) -> Result<(), PersistenceError> {
    // Query foreign_key_checks system variable
    // NOTE: This is raw SQL (justified - Diesel has no system variable query DSL)
    let result: Result<ForeignKeyCheck, _> =
        diesel::sql_query("SELECT @@foreign_key_checks AS fk_checks").get_result(conn);

    match result {
        Ok(check) => {
            if check.fk_checks == 1 {
                info!("MySQL foreign key enforcement is enabled");
                Ok(())
            } else {
                Err(PersistenceError::ForeignKeyEnforcementNotEnabled)
            }
        }
        Err(e) => Err(PersistenceError::QueryFailed(format!(
            "Failed to verify foreign key enforcement: {e}"
        ))),
    }
}
