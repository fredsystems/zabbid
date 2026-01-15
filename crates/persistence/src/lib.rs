// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Persistence layer for the ZAB Bidding System.
//!
//! This crate provides database persistence for audit events, state snapshots,
//! and canonical domain entities. It is built on Diesel and supports multiple
//! database backends.
//!
//! ## Database Backend Support
//!
//! ### Supported Backends
//!
//! - **`SQLite`** (default) — Used for development, unit tests, and integration tests
//! - **`MariaDB`/`MySQL`** — Validated via explicit opt-in tests
//!
//! ### Default Backend: `SQLite`
//!
//! `SQLite` is the primary backend for:
//! - All standard development workflows
//! - Unit and integration tests
//! - Fast, deterministic, in-memory testing
//!
//! `SQLite` support is always available and requires no external infrastructure.
//!
//! ### Additional Backend: `MariaDB`/`MySQL`
//!
//! `MySQL`/`MariaDB` support is compiled by default (no feature flags) but validated
//! only via explicit opt-in tests. See the `backend::mysql` module for details.
//!
//! To run `MySQL` validation tests:
//! ```bash
//! cargo xtask test-mariadb
//! ```
//!
//! This command:
//! 1. Starts a `MariaDB` container via `Docker`
//! 2. Runs migrations
//! 3. Executes backend validation tests marked with `#[ignore]`
//! 4. Cleans up the container
//!
//! ### Compilation Requirements
//!
//! `MySQL` support requires `MySQL` client development libraries at compile time.
//! These are provided by the `Nix` development environment (`flake.nix`).
//!
//! After updating the `Nix` environment:
//! ```bash
//! direnv allow
//! ```
//!
//! ### Migration Strategy
//!
//! Due to `SQL` syntax differences between backends, we maintain separate
//! migration directories:
//!
//! - `migrations/` — `SQLite`-specific (default)
//! - `migrations_mysql/` — `MySQL`/`MariaDB`-specific
//!
//! Both produce identical schema semantics but use backend-appropriate syntax.
//! See the `backend` module for details.
//!
//! ## Testing Philosophy
//!
//! - Standard tests (`cargo test`) run against `SQLite` only
//! - Backend validation tests are explicitly marked `#[ignore]`
//! - External database tests never run automatically
//! - All infrastructure is orchestrated by `xtask`, not embedded in tests
//! - Tests fail fast if required infrastructure is missing

#![deny(
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,
    clippy::style,
    clippy::correctness,
    clippy::all
)]
#![allow(clippy::multiple_crate_versions)]

mod backend;
mod data_models;
mod diesel_schema;
mod error;
mod mutations;
mod queries;

#[cfg(test)]
mod tests;

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use std::path::Path;
use zab_bid::{BootstrapMetadata, BootstrapResult, State, TransitionResult};
use zab_bid_audit::AuditEvent;
use zab_bid_domain::{Area, BidYear, CanonicalBidYear, Initials, User};

pub use data_models::{OperatorData, SessionData};
pub use error::PersistenceError;

/// Internal connection backend enum.
///
/// This enum wraps different database connection types and allows
/// the persistence adapter to dispatch queries to the appropriate backend.
pub(crate) enum ConnectionBackend {
    /// `SQLite` backend connection.
    Sqlite(SqliteConnection),
    /// MySQL/MariaDB backend connection.
    Mysql(MysqlConnection),
}

/// Macro to dispatch method calls to the underlying connection backend.
///
/// This macro eliminates boilerplate by automatically matching on the
/// connection backend and executing the same logic for each backend type.
macro_rules! with_conn {
    ($self:expr, $conn:ident, $body:expr) => {
        match &mut $self.conn {
            ConnectionBackend::Sqlite($conn) => $body,
            ConnectionBackend::Mysql(_conn) => {
                // MySQL backend not yet fully implemented
                // For now, return an error for MySQL-specific calls
                Err(PersistenceError::Other(
                    "MySQL backend query/mutation support not yet implemented".to_string(),
                ))
            }
        }
    };
}

/// Persistence adapter for audit events and state snapshots.
///
/// This adapter provides a backend-agnostic interface for persisting
/// domain events, state snapshots, and operational data. Backend selection
/// happens at construction time via factory functions.
///
/// Supported backends:
/// - `SQLite` (default for development and testing)
/// - MySQL/MariaDB (validated via opt-in tests)
pub struct Persistence {
    pub(crate) conn: ConnectionBackend,
}

impl Persistence {
    /// Creates a new persistence adapter with an in-memory database.
    ///
    /// Uses a shared in-memory database via Diesel.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn new_in_memory() -> Result<Self, PersistenceError> {
        use std::time::{SystemTime, UNIX_EPOCH};

        // Create a unique shared in-memory database name per call so tests are isolated.
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| PersistenceError::InitializationError(e.to_string()))?
            .as_nanos();
        let db_name = format!("memdb_{nanos}");
        let shared_memory_url = format!("file:{db_name}?mode=memory&cache=shared");

        // Initialize database with Diesel migrations
        let mut conn: SqliteConnection = backend::sqlite::initialize_database(&shared_memory_url)?;

        // Verify foreign key enforcement is active
        backend::sqlite::verify_foreign_key_enforcement(&mut conn)?;

        Ok(Self {
            conn: ConnectionBackend::Sqlite(conn),
        })
    }

    /// Creates a new persistence adapter with a file-based database.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the `SQLite` database file
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn new_with_file<P: AsRef<Path>>(path: P) -> Result<Self, PersistenceError> {
        let path_str = path.as_ref().to_str().ok_or_else(|| {
            PersistenceError::InitializationError("Invalid database path".to_string())
        })?;

        // Initialize database with Diesel migrations
        let mut conn: SqliteConnection = backend::sqlite::initialize_database(path_str)?;

        // Enable WAL mode for better read concurrency
        backend::sqlite::enable_wal_mode(&mut conn)?;

        // Verify foreign key enforcement is active
        backend::sqlite::verify_foreign_key_enforcement(&mut conn)?;

        Ok(Self {
            conn: ConnectionBackend::Sqlite(conn),
        })
    }

    /// Creates a new persistence adapter with a MySQL/MariaDB database.
    ///
    /// # Arguments
    ///
    /// * `database_url` - The `MySQL` connection URL (e.g., `mysql://user:pass@host/db`)
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be connected or initialized.
    pub fn new_mysql(database_url: &str) -> Result<Self, PersistenceError> {
        // Initialize database with Diesel migrations
        let mut conn: MysqlConnection = backend::mysql::initialize_database(database_url)?;

        // Verify foreign key enforcement is active
        backend::mysql::verify_foreign_key_enforcement(&mut conn)?;

        Ok(Self {
            conn: ConnectionBackend::Mysql(conn),
        })
    }

    /// Verifies that foreign key enforcement is enabled.
    ///
    /// This is a startup-time check required to ensure
    /// referential integrity constraints are enforced.
    ///
    /// # Errors
    ///
    /// Returns an error if foreign key enforcement is not enabled.
    pub fn verify_foreign_key_enforcement(&mut self) -> Result<(), PersistenceError> {
        match &mut self.conn {
            ConnectionBackend::Sqlite(conn) => {
                backend::sqlite::verify_foreign_key_enforcement(conn)
            }
            ConnectionBackend::Mysql(conn) => backend::mysql::verify_foreign_key_enforcement(conn),
        }
    }

    // ========================================================================
    // Transitions & Bootstrap
    // ========================================================================

    /// Persists a transition result (audit event and optionally a full snapshot).
    ///
    /// # Arguments
    ///
    /// * `result` - The transition result to persist
    ///
    /// # Returns
    ///
    /// The event ID assigned to the persisted audit event.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub fn persist_transition(
        &mut self,
        result: &TransitionResult,
    ) -> Result<i64, PersistenceError> {
        let should_snapshot = queries::state::should_snapshot(&result.audit_event.action.name);
        with_conn!(
            self,
            conn,
            mutations::persist_transition(conn, result, should_snapshot)
        )
    }

    /// Persists an audit event.
    ///
    /// # Arguments
    ///
    /// * `event` - The audit event to persist
    ///
    /// # Returns
    ///
    /// The event ID assigned to the persisted audit event.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub fn persist_audit_event(&mut self, event: &AuditEvent) -> Result<i64, PersistenceError> {
        with_conn!(self, conn, mutations::persist_audit_event(conn, event))
    }

    /// Persists a bootstrap result (audit event for bid year/area creation).
    ///
    /// # Arguments
    ///
    /// * `result` - The bootstrap result to persist
    ///
    /// # Returns
    ///
    /// The event ID assigned to the persisted audit event.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub fn persist_bootstrap(&mut self, result: &BootstrapResult) -> Result<i64, PersistenceError> {
        with_conn!(self, conn, mutations::persist_bootstrap(conn, result))
    }

    // ========================================================================
    // Audit Event Queries
    // ========================================================================

    /// Retrieves an audit event by ID.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The event ID to retrieve
    ///
    /// # Errors
    ///
    /// Returns an error if the event is not found or cannot be deserialized.
    pub fn get_audit_event(&mut self, event_id: i64) -> Result<AuditEvent, PersistenceError> {
        with_conn!(self, conn, queries::get_audit_event(conn, event_id))
    }

    /// Retrieves the most recent state snapshot for a `(bid_year, area)` scope.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    ///
    /// # Errors
    ///
    /// Returns an error if no snapshot exists or cannot be deserialized.
    pub fn get_latest_snapshot(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<(State, i64), PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            let area_id = queries::lookup_area_id(conn, bid_year_id, area.id())?;
            queries::get_latest_snapshot(conn, bid_year_id, area_id)
        })
    }

    /// Retrieves all audit events for a `(bid_year, area)` scope after a given event ID.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    /// * `after_event_id` - Only return events after this ID (exclusive)
    ///
    /// # Errors
    ///
    /// Returns an error if events cannot be retrieved or deserialized.
    pub fn get_events_after(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
        after_event_id: i64,
    ) -> Result<Vec<AuditEvent>, PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            let area_id = queries::lookup_area_id(conn, bid_year_id, area.id())?;
            queries::get_events_after(conn, bid_year_id, area_id, after_event_id)
        })
    }

    /// Retrieves the current effective state for a given `(bid_year, area)` scope.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_current_state(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<State, PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            let area_id = queries::lookup_area_id(conn, bid_year_id, area.id())?;
            queries::get_current_state(conn, bid_year_id, area_id, bid_year, area)
        })
    }

    /// Retrieves the effective state for a given `(bid_year, area)` scope at a specific timestamp.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    /// * `timestamp` - The target timestamp (ISO 8601 format)
    ///
    /// # Errors
    ///
    /// Returns an error if no snapshot exists before the timestamp.
    pub fn get_historical_state(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
        timestamp: &str,
    ) -> Result<State, PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            let area_id = queries::lookup_area_id(conn, bid_year_id, area.id())?;
            queries::get_historical_state(conn, bid_year_id, area_id, timestamp)
        })
    }

    /// Retrieves the ordered audit event timeline for a given `(bid_year, area)` scope.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    ///
    /// # Errors
    ///
    /// Returns an error if events cannot be retrieved or deserialized.
    pub fn get_audit_timeline(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<Vec<AuditEvent>, PersistenceError> {
        with_conn!(self, conn, {
            // Look up the canonical IDs - if they don't exist, return empty timeline
            let bid_year_id = match queries::lookup_bid_year_id(conn, bid_year.year()) {
                Ok(id) => id,
                Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
                Err(e) => return Err(e),
            };
            let area_id = match queries::lookup_area_id(conn, bid_year_id, area.id()) {
                Ok(id) => id,
                Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
                Err(e) => return Err(e),
            };

            queries::get_audit_timeline(conn, bid_year_id, area_id)
        })
    }

    /// Retrieves all global audit events (events with no bid year or area scope).
    ///
    /// # Errors
    ///
    /// Returns an error if events cannot be retrieved or deserialized.
    pub fn get_global_audit_events(&mut self) -> Result<Vec<AuditEvent>, PersistenceError> {
        with_conn!(self, conn, queries::get_global_audit_events(conn))
    }

    // ========================================================================
    // Bootstrap & Canonical Queries
    // ========================================================================

    /// Reconstructs bootstrap metadata from canonical tables.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_bootstrap_metadata(&mut self) -> Result<BootstrapMetadata, PersistenceError> {
        with_conn!(self, conn, queries::get_bootstrap_metadata(conn))
    }

    /// Lists all bid years that have been created.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn list_bid_years(&mut self) -> Result<Vec<CanonicalBidYear>, PersistenceError> {
        with_conn!(self, conn, queries::list_bid_years(conn))
    }

    /// Lists all areas for a given bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year to list areas for
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn list_areas(&mut self, bid_year: &BidYear) -> Result<Vec<Area>, PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = match bid_year.bid_year_id() {
                Some(id) => id,
                None => match queries::lookup_bid_year_id(conn, bid_year.year()) {
                    Ok(id) => id,
                    Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
                    Err(e) => return Err(e),
                },
            };
            queries::list_areas(conn, bid_year_id)
        })
    }

    /// Lists all users for a given `(bid_year, area)` scope.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn list_users(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<Vec<User>, PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            let area_id = queries::lookup_area_id(conn, bid_year_id, area.id())?;
            queries::list_users(conn, bid_year_id, area_id, bid_year, area)
        })
    }

    // ========================================================================
    // Completeness Queries
    // ========================================================================

    /// Counts users per area for a given bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year to count users for
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_users_by_area(
        &mut self,
        bid_year: &BidYear,
    ) -> Result<Vec<(String, usize)>, PersistenceError> {
        let bid_year_id = bid_year.bid_year_id().ok_or_else(|| {
            PersistenceError::ReconstructionError(
                "BidYear must have a bid_year_id to count users".to_string(),
            )
        })?;
        with_conn!(self, conn, queries::count_users_by_area(conn, bid_year_id))
    }

    /// Counts areas per bid year.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_areas_by_bid_year(&mut self) -> Result<Vec<(u16, usize)>, PersistenceError> {
        with_conn!(self, conn, queries::count_areas_by_bid_year(conn))
    }

    /// Counts total users per bid year across all areas.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_users_by_bid_year(&mut self) -> Result<Vec<(u16, usize)>, PersistenceError> {
        with_conn!(self, conn, queries::count_users_by_bid_year(conn))
    }

    /// Counts users per (`bid_year`, `area_id`) combination.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_users_by_bid_year_and_area(
        &mut self,
    ) -> Result<Vec<(u16, String, usize)>, PersistenceError> {
        with_conn!(self, conn, queries::count_users_by_bid_year_and_area(conn))
    }

    /// Determines if a given action requires a full snapshot.
    ///
    /// # Arguments
    ///
    /// * `action_name` - The action name to check
    ///
    /// # Returns
    ///
    /// `true` if the action requires a snapshot, `false` otherwise.
    #[must_use]
    pub fn should_snapshot(&mut self, action_name: &str) -> bool {
        queries::state::should_snapshot(action_name)
    }

    // ========================================================================
    // Operator Queries
    // ========================================================================

    /// Creates a new operator.
    ///
    /// # Arguments
    ///
    /// * `login_name` - The login name (will be normalized)
    /// * `display_name` - The display name
    /// * `password` - The plain-text password (will be hashed)
    /// * `role` - The role (Admin or Bidder)
    ///
    /// # Errors
    ///
    /// Returns an error if the operator cannot be created.
    pub fn create_operator(
        &mut self,
        login_name: &str,
        display_name: &str,
        password: &str,
        role: &str,
    ) -> Result<i64, PersistenceError> {
        with_conn!(
            self,
            conn,
            mutations::create_operator(conn, login_name, display_name, password, role)
        )
    }

    /// Retrieves an operator by login name.
    ///
    /// # Arguments
    ///
    /// * `login_name` - The login name to search for
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn get_operator_by_login(
        &mut self,
        login_name: &str,
    ) -> Result<Option<OperatorData>, PersistenceError> {
        with_conn!(self, conn, queries::get_operator_by_login(conn, login_name))
    }

    /// Retrieves an operator by ID.
    ///
    /// # Arguments
    ///
    /// * `operator_id` - The operator ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn get_operator_by_id(
        &mut self,
        operator_id: i64,
    ) -> Result<Option<OperatorData>, PersistenceError> {
        with_conn!(self, conn, queries::get_operator_by_id(conn, operator_id))
    }

    /// Updates the last login timestamp for an operator.
    ///
    /// # Arguments
    ///
    /// * `operator_id` - The operator ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails.
    pub fn update_last_login(&mut self, operator_id: i64) -> Result<(), PersistenceError> {
        with_conn!(self, conn, mutations::update_last_login(conn, operator_id))
    }

    /// Disables an operator.
    ///
    /// # Arguments
    ///
    /// * `operator_id` - The operator ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails.
    pub fn disable_operator(&mut self, operator_id: i64) -> Result<(), PersistenceError> {
        with_conn!(self, conn, mutations::disable_operator(conn, operator_id))
    }

    /// Re-enables a disabled operator.
    ///
    /// # Arguments
    ///
    /// * `operator_id` - The operator ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails.
    pub fn enable_operator(&mut self, operator_id: i64) -> Result<(), PersistenceError> {
        with_conn!(self, conn, mutations::enable_operator(conn, operator_id))
    }

    /// Deletes an operator if they are not referenced by any audit events.
    ///
    /// # Arguments
    ///
    /// * `operator_id` - The operator ID
    ///
    /// # Errors
    ///
    /// Returns an error if the operator is referenced or doesn't exist.
    pub fn delete_operator(&mut self, operator_id: i64) -> Result<(), PersistenceError> {
        with_conn!(self, conn, mutations::delete_operator(conn, operator_id))
    }

    /// Lists all operators.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn list_operators(&mut self) -> Result<Vec<OperatorData>, PersistenceError> {
        with_conn!(self, conn, queries::list_operators(conn))
    }

    /// Checks if an operator is referenced by any audit events.
    ///
    /// # Arguments
    ///
    /// * `operator_id` - The operator ID to check
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn is_operator_referenced(&mut self, operator_id: i64) -> Result<bool, PersistenceError> {
        with_conn!(
            self,
            conn,
            queries::is_operator_referenced(conn, operator_id)
        )
    }

    /// Counts the total number of operators.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn count_operators(&mut self) -> Result<i64, PersistenceError> {
        with_conn!(self, conn, queries::count_operators(conn))
    }

    /// Counts the number of active admin operators.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn count_active_admin_operators(&mut self) -> Result<i64, PersistenceError> {
        with_conn!(self, conn, queries::count_active_admin_operators(conn))
    }

    /// Verifies a password against a stored hash.
    ///
    /// # Arguments
    ///
    /// * `password` - The plain text password to verify
    /// * `password_hash` - The stored bcrypt hash
    ///
    /// # Errors
    ///
    /// Returns an error if password verification fails.
    pub fn verify_password(
        &self,
        password: &str,
        password_hash: &str,
    ) -> Result<bool, PersistenceError> {
        queries::verify_password(password, password_hash)
    }

    /// Updates an operator's password.
    ///
    /// # Arguments
    ///
    /// * `operator_id` - The operator ID
    /// * `new_password` - The new password (will be hashed)
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    pub fn update_password(
        &mut self,
        operator_id: i64,
        new_password: &str,
    ) -> Result<(), PersistenceError> {
        with_conn!(
            self,
            conn,
            mutations::update_password(conn, operator_id, new_password)
        )
    }

    /// Deletes all sessions for a specific operator.
    ///
    /// # Arguments
    ///
    /// * `operator_id` - The operator ID whose sessions should be deleted
    ///
    /// # Errors
    ///
    /// Returns an error if the database delete fails.
    pub fn delete_sessions_for_operator(
        &mut self,
        operator_id: i64,
    ) -> Result<usize, PersistenceError> {
        with_conn!(
            self,
            conn,
            mutations::delete_sessions_for_operator(conn, operator_id)
        )
    }

    // ========================================================================
    // Session Management
    // ========================================================================

    /// Creates a new session for an operator.
    ///
    /// # Arguments
    ///
    /// * `session_token` - The unique session token
    /// * `operator_id` - The operator ID
    /// * `expires_at` - The expiration timestamp (ISO 8601 format)
    ///
    /// # Errors
    ///
    /// Returns an error if the session cannot be created.
    pub fn create_session(
        &mut self,
        session_token: &str,
        operator_id: i64,
        expires_at: &str,
    ) -> Result<i64, PersistenceError> {
        with_conn!(
            self,
            conn,
            mutations::create_session(conn, session_token, operator_id, expires_at)
        )
    }

    /// Retrieves a session by token.
    ///
    /// # Arguments
    ///
    /// * `session_token` - The session token
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn get_session_by_token(
        &mut self,
        session_token: &str,
    ) -> Result<Option<SessionData>, PersistenceError> {
        with_conn!(
            self,
            conn,
            queries::get_session_by_token(conn, session_token)
        )
    }

    /// Updates the last activity timestamp for a session.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails.
    pub fn update_session_activity(&mut self, session_id: i64) -> Result<(), PersistenceError> {
        with_conn!(
            self,
            conn,
            mutations::update_session_activity(conn, session_id)
        )
    }

    /// Deletes a session by token.
    ///
    /// # Arguments
    ///
    /// * `session_token` - The session token to delete
    ///
    /// # Errors
    ///
    /// Returns an error if the database delete fails.
    pub fn delete_session(&mut self, session_token: &str) -> Result<(), PersistenceError> {
        with_conn!(self, conn, mutations::delete_session(conn, session_token))
    }

    /// Deletes all expired sessions.
    ///
    /// # Errors
    ///
    /// Returns an error if the database delete fails.
    pub fn delete_expired_sessions(&mut self) -> Result<usize, PersistenceError> {
        with_conn!(self, conn, mutations::delete_expired_sessions(conn))
    }

    // ========================================================================
    // Bootstrap Configuration
    // ========================================================================

    /// Sets a bid year as active.
    ///
    /// # Arguments
    ///
    /// * `year` - The year to mark as active
    ///
    /// # Errors
    ///
    /// Returns an error if the bid year doesn't exist or update fails.
    pub fn set_active_bid_year(&mut self, year: u16) -> Result<(), PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, year)?;
            mutations::set_active_bid_year(conn, bid_year_id)
        })
    }

    /// Gets the active bid year.
    ///
    /// # Errors
    ///
    /// Returns an error if no active bid year exists.
    pub fn get_active_bid_year(&mut self) -> Result<u16, PersistenceError> {
        with_conn!(self, conn, queries::get_active_bid_year(conn))
    }

    /// Sets the expected area count for a bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `count` - The expected area count
    ///
    /// # Errors
    ///
    /// Returns an error if the bid year doesn't exist or update fails.
    pub fn set_expected_area_count(
        &mut self,
        bid_year: &BidYear,
        count: usize,
    ) -> Result<(), PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            mutations::set_expected_area_count(conn, bid_year_id, count)
        })
    }

    /// Gets the expected area count for a bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    ///
    /// # Errors
    ///
    /// Returns an error if the bid year doesn't exist.
    pub fn get_expected_area_count(
        &mut self,
        bid_year: &BidYear,
    ) -> Result<Option<usize>, PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            queries::get_expected_area_count(conn, bid_year_id)
        })
    }

    /// Sets the expected user count for an area.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    /// * `count` - The expected user count
    ///
    /// # Errors
    ///
    /// Returns an error if the area doesn't exist or update fails.
    pub fn set_expected_user_count(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
        count: usize,
    ) -> Result<(), PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            let area_id = queries::lookup_area_id(conn, bid_year_id, area.id())?;
            mutations::set_expected_user_count(conn, bid_year_id, area_id, count)
        })
    }

    /// Gets the expected user count for an area.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    ///
    /// # Errors
    ///
    /// Returns an error if the area doesn't exist.
    pub fn get_expected_user_count(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<Option<usize>, PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            let area_id = queries::lookup_area_id(conn, bid_year_id, area.id())?;
            queries::get_expected_user_count(conn, bid_year_id, area_id)
        })
    }

    /// Gets the actual area count for a bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_actual_area_count(&mut self, bid_year: &BidYear) -> Result<usize, PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            queries::get_actual_area_count(conn, bid_year_id)
        })
    }

    /// Gets the actual user count for an area.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_actual_user_count(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<usize, PersistenceError> {
        with_conn!(self, conn, {
            let bid_year_id = queries::lookup_bid_year_id(conn, bid_year.year())?;
            let area_id = queries::lookup_area_id(conn, bid_year_id, area.id())?;
            queries::get_actual_user_count(conn, bid_year_id, area_id)
        })
    }

    /// Updates an existing user's information.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user's canonical internal identifier
    /// * `initials` - The user's initials
    /// * `name` - The user's name
    /// * `area` - The user's area
    /// * `user_type` - The user's type classification
    /// * `crew` - The user's crew (optional)
    /// * `cumulative_natca_bu_date` - Cumulative NATCA bargaining unit date
    /// * `natca_bu_date` - NATCA bargaining unit date
    /// * `eod_faa_date` - Entry on Duty / FAA date
    /// * `service_computation_date` - Service Computation Date
    /// * `lottery_value` - Optional lottery value
    ///
    /// # Errors
    ///
    /// Returns an error if the user doesn't exist or update fails.
    #[allow(clippy::too_many_arguments)]
    pub fn update_user(
        &mut self,
        user_id: i64,
        initials: &Initials,
        name: &str,
        area: &Area,
        user_type: &str,
        crew: Option<u8>,
        cumulative_natca_bu_date: &str,
        natca_bu_date: &str,
        eod_faa_date: &str,
        service_computation_date: &str,
        lottery_value: Option<u32>,
    ) -> Result<(), PersistenceError> {
        with_conn!(
            self,
            conn,
            mutations::update_user(
                conn,
                user_id,
                initials,
                name,
                area,
                user_type,
                crew,
                cumulative_natca_bu_date,
                natca_bu_date,
                eod_faa_date,
                service_computation_date,
                lottery_value,
            )
        )
    }

    // ========================================================================
    // Canonical ID Lookups (Test Support)
    // ========================================================================

    /// Queries the canonical `bid_year_id` for a given year.
    ///
    /// # Arguments
    ///
    /// * `year` - The year to query
    ///
    /// # Errors
    ///
    /// Returns an error if the bid year is not found or the query fails.
    pub fn get_bid_year_id(&mut self, year: u16) -> Result<i64, PersistenceError> {
        use crate::diesel_schema::bid_years;

        with_conn!(self, conn, {
            let result: Result<i64, diesel::result::Error> = bid_years::table
                .select(bid_years::bid_year_id)
                .filter(bid_years::year.eq(i32::from(year)))
                .first::<i64>(conn);

            match result {
                Ok(bid_year_id) => Ok(bid_year_id),
                Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(format!(
                    "Bid year {year} not found"
                ))),
                Err(e) => Err(PersistenceError::from(e)),
            }
        })
    }

    /// Queries the canonical `area_id` for a given bid year and area code.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year identifier
    /// * `area_code` - The area code
    ///
    /// # Errors
    ///
    /// Returns an error if the area is not found or the query fails.
    pub fn get_area_id(
        &mut self,
        bid_year_id: i64,
        area_code: &str,
    ) -> Result<i64, PersistenceError> {
        use crate::diesel_schema::areas;

        with_conn!(self, conn, {
            let normalized_code: String = area_code.to_uppercase();
            let result: Result<i64, diesel::result::Error> = areas::table
                .select(areas::area_id)
                .filter(areas::bid_year_id.eq(bid_year_id))
                .filter(areas::area_code.eq(&normalized_code))
                .first::<i64>(conn);

            match result {
                Ok(area_id) => Ok(area_id),
                Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(format!(
                    "Area {area_code} not found"
                ))),
                Err(e) => Err(PersistenceError::from(e)),
            }
        })
    }

    /// Queries the canonical `user_id` for a given bid year, area, and initials.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year identifier
    /// * `area_id` - The canonical area identifier
    /// * `initials` - The user initials
    ///
    /// # Errors
    ///
    /// Returns an error if the user is not found or the query fails.
    pub fn get_user_id(
        &mut self,
        bid_year_id: i64,
        area_id: i64,
        initials: &str,
    ) -> Result<i64, PersistenceError> {
        use crate::diesel_schema::users;

        with_conn!(self, conn, {
            let normalized_initials: String = initials.to_uppercase();
            let result: Result<i64, diesel::result::Error> = users::table
                .select(users::user_id)
                .filter(users::bid_year_id.eq(bid_year_id))
                .filter(users::area_id.eq(area_id))
                .filter(users::initials.eq(&normalized_initials))
                .first::<i64>(conn);

            match result {
                Ok(user_id) => Ok(user_id),
                Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(format!(
                    "User {initials} not found"
                ))),
                Err(e) => Err(PersistenceError::from(e)),
            }
        })
    }
}
