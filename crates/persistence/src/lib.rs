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
    clippy::all,
    clippy::suspicious,
    clippy::complexity,
    clippy::perf,
    clippy::unwrap_used,
    clippy::expect_used
)]
#![allow(clippy::multiple_crate_versions)]

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use zab_bid::{BootstrapMetadata, BootstrapResult, State, TransitionResult};
use zab_bid_audit::AuditEvent;
use zab_bid_domain::{Area, BidYear, CanonicalBidYear, Initials, Round, RoundGroup, User};

/// Atomic counter for generating unique in-memory database names.
///
/// This ensures deterministic test isolation by eliminating time-based collisions.
/// Each call to `new_in_memory()` receives a unique sequential ID.
static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Macro to generate monomorphic backend-specific query/mutation functions.
///
/// This macro generates two separate functions from a single function body:
/// - One suffixed with `_sqlite` taking `&mut SqliteConnection`
/// - One suffixed with `_mysql` taking `&mut MysqlConnection`
///
/// This approach is required because Diesel's type system requires concrete
/// backend types at compile time and cannot handle generic backend functions.
///
/// # Constraints
///
/// - The macro ONLY duplicates function bodies and substitutes connection types
/// - No logic, branching, or dispatch occurs within the macro
/// - Backend dispatch happens exclusively in the Persistence adapter
/// - The generated functions are completely monomorphic
///
/// # Usage
///
/// ```ignore
/// backend_fn! {
///     pub fn my_query(conn: &mut _, param: i64) -> Result<String, PersistenceError> {
///         // Function body using conn - same for both backends
///         diesel_schema::table::table
///             .filter(diesel_schema::table::id.eq(param))
///             .first::<String>(conn)
///             .map_err(Into::into)
///     }
/// }
/// ```
///
/// This generates:
/// - `my_query_sqlite(&mut SqliteConnection, i64) -> Result<String, PersistenceError>`
/// - `my_query_mysql(&mut MysqlConnection, i64) -> Result<String, PersistenceError>`
macro_rules! backend_fn {
    (
        $(#[$meta:meta])*
        $vis:vis fn $name:ident (
            $conn:ident : &mut _
            $(, $param:ident : $param_ty:ty)* $(,)?
        ) -> $ret:ty
        $body:block
    ) => {
        pastey::paste! {
            // Generate SQLite version
            $(#[$meta])*
            $vis fn [<$name _sqlite>] (
                $conn: &mut SqliteConnection
                $(, $param : $param_ty)*
            ) -> $ret
            $body

            // Generate MySQL version
            $(#[$meta])*
            $vis fn [<$name _mysql>] (
                $conn: &mut MysqlConnection
                $(, $param : $param_ty)*
            ) -> $ret
            $body
        }
    };
}

mod backend;
mod data_models;
mod diesel_schema;
mod error;
mod mutations;
mod queries;

#[cfg(test)]
mod tests;

pub use data_models::{OperatorData, SessionData};
pub use error::PersistenceError;
pub use mutations::PersistTransitionResult;

use backend::PersistenceBackend;

/// Type alias for backward compatibility.
/// All new code should use `Persistence` directly.
pub type SqlitePersistence = Persistence;

/// Internal enum for backend-specific database connections.
///
/// This enum allows the persistence adapter to work with either `SQLite` or `MySQL`
/// backends while maintaining a single public API.
pub enum BackendConnection {
    Sqlite(SqliteConnection),
    Mysql(MysqlConnection),
}

/// Persistence adapter for audit events and state snapshots.
///
/// This adapter is backend-agnostic and works with both `SQLite` and `MySQL`/`MariaDB`.
/// Backend selection happens once at construction time and is transparent to callers.
pub struct Persistence {
    pub(crate) conn: BackendConnection,
}

impl Persistence {
    /// Creates a new persistence adapter with an in-memory `SQLite` database.
    ///
    /// Uses a shared in-memory database via `Diesel`.
    ///
    /// Each call receives a unique database instance via atomic counter,
    /// ensuring deterministic test isolation without time-based collisions.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn new_in_memory() -> Result<Self, PersistenceError> {
        // Create a unique shared in-memory database name per call so tests are isolated.
        // Use atomic counter instead of timestamp to eliminate race conditions.
        let db_id = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_name = format!("memdb_test_{db_id}");
        let shared_memory_url = format!("file:{db_name}?mode=memory&cache=shared");

        // Initialize database with Diesel migrations
        let mut conn: SqliteConnection = backend::sqlite::initialize_database(&shared_memory_url)?;

        // Verify foreign key enforcement is active
        backend::sqlite::verify_foreign_key_enforcement(&mut conn)?;

        Ok(Self {
            conn: BackendConnection::Sqlite(conn),
        })
    }

    /// Creates a new persistence adapter with a file-based `SQLite` database.
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
            conn: BackendConnection::Sqlite(conn),
        })
    }

    /// Creates a new persistence adapter with a `MySQL`/`MariaDB` database.
    ///
    /// # Arguments
    ///
    /// * `database_url` - The `MySQL` connection URL (e.g., `mysql://user:pass@host/db`)
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn new_with_mysql(database_url: &str) -> Result<Self, PersistenceError> {
        // Initialize database with Diesel migrations
        let mut conn: MysqlConnection = backend::mysql::initialize_database(database_url)?;

        // Verify foreign key enforcement is active
        backend::mysql::verify_foreign_key_enforcement(&mut conn)?;

        Ok(Self {
            conn: BackendConnection::Mysql(conn),
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
            BackendConnection::Sqlite(conn) => conn.verify_foreign_key_enforcement(),
            BackendConnection::Mysql(conn) => conn.verify_foreign_key_enforcement(),
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
    /// A `PersistTransitionResult` containing the event ID and optionally the user ID
    /// (for `RegisterUser` transitions).
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails.
    pub fn persist_transition(
        &mut self,
        result: &TransitionResult,
    ) -> Result<mutations::PersistTransitionResult, PersistenceError> {
        let should_snapshot = queries::state::should_snapshot(&result.audit_event.action.name);
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::persist_transition_sqlite(conn, result, should_snapshot)
            }
            BackendConnection::Mysql(conn) => {
                mutations::persist_transition_mysql(conn, result, should_snapshot)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => mutations::persist_audit_event_sqlite(conn, event),
            BackendConnection::Mysql(conn) => mutations::persist_audit_event_mysql(conn, event),
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => mutations::persist_bootstrap_sqlite(conn, result),
            BackendConnection::Mysql(conn) => mutations::persist_bootstrap_mysql(conn, result),
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::audit::get_audit_event_sqlite(conn, event_id)
            }
            BackendConnection::Mysql(conn) => queries::audit::get_audit_event_mysql(conn, event_id),
        }
    }

    /// Retrieves the most recent state snapshot for a `(BidYear, Area)` scope.
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id = queries::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                let area_id = queries::lookup_area_id_sqlite(conn, bid_year_id, area.id())?;
                queries::get_latest_snapshot_sqlite(conn, bid_year_id, area_id)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id = queries::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                let area_id = queries::lookup_area_id_mysql(conn, bid_year_id, area.id())?;
                queries::get_latest_snapshot_mysql(conn, bid_year_id, area_id)
            }
        }
    }

    /// Retrieves all audit events for a `(BidYear, Area)` scope after a given event ID.
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id = queries::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                let area_id = queries::lookup_area_id_sqlite(conn, bid_year_id, area.id())?;
                queries::get_events_after_sqlite(conn, bid_year_id, area_id, after_event_id)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id = queries::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                let area_id = queries::lookup_area_id_mysql(conn, bid_year_id, area.id())?;
                queries::get_events_after_mysql(conn, bid_year_id, area_id, after_event_id)
            }
        }
    }

    /// Retrieves the current effective state for a given `(BidYear, Area)` scope.
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id = queries::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                let area_id = queries::lookup_area_id_sqlite(conn, bid_year_id, area.id())?;
                queries::get_current_state_sqlite(conn, bid_year_id, area_id, bid_year, area)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id = queries::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                let area_id = queries::lookup_area_id_mysql(conn, bid_year_id, area.id())?;
                queries::get_current_state_mysql(conn, bid_year_id, area_id, bid_year, area)
            }
        }
    }

    /// Retrieves the effective state for a given `(BidYear, Area)` scope at a specific timestamp.
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id = queries::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                let area_id = queries::lookup_area_id_sqlite(conn, bid_year_id, area.id())?;
                queries::get_historical_state_sqlite(conn, bid_year_id, area_id, timestamp)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id = queries::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                let area_id = queries::lookup_area_id_mysql(conn, bid_year_id, area.id())?;
                queries::get_historical_state_mysql(conn, bid_year_id, area_id, timestamp)
            }
        }
    }

    /// Retrieves the ordered audit event timeline for a given `(BidYear, Area)` scope.
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                // Look up the canonical IDs - if they don't exist, return empty timeline
                let bid_year_id = match queries::lookup_bid_year_id_sqlite(conn, bid_year.year()) {
                    Ok(id) => id,
                    Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
                    Err(e) => return Err(e),
                };
                let area_id = match queries::lookup_area_id_sqlite(conn, bid_year_id, area.id()) {
                    Ok(id) => id,
                    Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
                    Err(e) => return Err(e),
                };

                queries::get_audit_timeline_sqlite(conn, bid_year_id, area_id)
            }
            BackendConnection::Mysql(conn) => {
                // Look up the canonical IDs - if they don't exist, return empty timeline
                let bid_year_id = match queries::lookup_bid_year_id_mysql(conn, bid_year.year()) {
                    Ok(id) => id,
                    Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
                    Err(e) => return Err(e),
                };
                let area_id = match queries::lookup_area_id_mysql(conn, bid_year_id, area.id()) {
                    Ok(id) => id,
                    Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
                    Err(e) => return Err(e),
                };

                queries::get_audit_timeline_mysql(conn, bid_year_id, area_id)
            }
        }
    }

    /// Retrieves all global audit events (events with no bid year or area scope).
    ///
    /// # Errors
    ///
    /// Returns an error if events cannot be retrieved or deserialized.
    pub fn get_global_audit_events(&mut self) -> Result<Vec<AuditEvent>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::get_global_audit_events_sqlite(conn),
            BackendConnection::Mysql(conn) => queries::get_global_audit_events_mysql(conn),
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::get_bootstrap_metadata_sqlite(conn),
            BackendConnection::Mysql(conn) => queries::get_bootstrap_metadata_mysql(conn),
        }
    }

    /// Lists all bid years that have been created.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn list_bid_years(&mut self) -> Result<Vec<CanonicalBidYear>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::list_bid_years_sqlite(conn),
            BackendConnection::Mysql(conn) => queries::list_bid_years_mysql(conn),
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id = match bid_year.bid_year_id() {
                    Some(id) => id,
                    None => match queries::lookup_bid_year_id_sqlite(conn, bid_year.year()) {
                        Ok(id) => id,
                        Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
                        Err(e) => return Err(e),
                    },
                };
                queries::list_areas_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id = match bid_year.bid_year_id() {
                    Some(id) => id,
                    None => match queries::lookup_bid_year_id_mysql(conn, bid_year.year()) {
                        Ok(id) => id,
                        Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
                        Err(e) => return Err(e),
                    },
                };
                queries::list_areas_mysql(conn, bid_year_id)
            }
        }
    }

    /// Lists all users for a given `(BidYear, Area)` scope.
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id = queries::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                let area_id = queries::lookup_area_id_sqlite(conn, bid_year_id, area.id())?;
                queries::list_users_sqlite(conn, bid_year_id, area_id, bid_year, area)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id = queries::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                let area_id = queries::lookup_area_id_mysql(conn, bid_year_id, area.id())?;
                queries::list_users_mysql(conn, bid_year_id, area_id, bid_year, area)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::count_users_by_area_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => queries::count_users_by_area_mysql(conn, bid_year_id),
        }
    }

    /// Counts areas per bid year.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_areas_by_bid_year(&mut self) -> Result<Vec<(u16, usize)>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::count_areas_by_bid_year_sqlite(conn),
            BackendConnection::Mysql(conn) => queries::count_areas_by_bid_year_mysql(conn),
        }
    }

    /// Counts total users per bid year across all areas.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_users_by_bid_year(&mut self) -> Result<Vec<(u16, usize)>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::count_users_by_bid_year_sqlite(conn),
            BackendConnection::Mysql(conn) => queries::count_users_by_bid_year_mysql(conn),
        }
    }

    /// Counts users per (`bid_year`, `area_id`) combination.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_users_by_bid_year_and_area(
        &mut self,
    ) -> Result<Vec<(u16, String, usize)>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::count_users_by_bid_year_and_area_sqlite(conn)
            }
            BackendConnection::Mysql(conn) => queries::count_users_by_bid_year_and_area_mysql(conn),
        }
    }

    /// Finds the system area (No Bid) for a given bid year.
    ///
    /// Phase 25B: Returns the area ID and area code of the system area.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    ///
    /// # Returns
    ///
    /// * `Ok(Some((area_id, area_code)))` if a system area exists
    /// * `Ok(None)` if no system area exists
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn find_system_area(
        &mut self,
        bid_year_id: i64,
    ) -> Result<Option<(i64, String)>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::find_system_area_sqlite(conn, bid_year_id),
            BackendConnection::Mysql(conn) => queries::find_system_area_mysql(conn, bid_year_id),
        }
    }

    /// Counts users in the system area (No Bid) for a given bid year.
    ///
    /// Phase 25B: Used to check if bootstrap can be completed.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    ///
    /// # Returns
    ///
    /// The number of users in the No Bid area (0 if no system area exists).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_users_in_system_area(
        &mut self,
        bid_year_id: i64,
    ) -> Result<usize, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::count_users_in_system_area_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::count_users_in_system_area_mysql(conn, bid_year_id)
            }
        }
    }

    /// Lists users in the system area (No Bid) for a given bid year.
    ///
    /// Phase 25B: Returns up to `limit` user initials for error reporting.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `limit` - Maximum number of initials to return
    ///
    /// # Returns
    ///
    /// A vector of user initials (empty if no system area or no users).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn list_users_in_system_area(
        &mut self,
        bid_year_id: i64,
        limit: i64,
    ) -> Result<Vec<String>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::list_users_in_system_area_sqlite(conn, bid_year_id, limit)
            }
            BackendConnection::Mysql(conn) => {
                queries::list_users_in_system_area_mysql(conn, bid_year_id, limit)
            }
        }
    }

    /// Checks if an area is a system area.
    ///
    /// Phase 25B: Used to prevent deletion/renaming of system areas.
    ///
    /// # Arguments
    ///
    /// * `area_id` - The canonical area ID to check
    ///
    /// # Returns
    ///
    /// `true` if the area is a system area, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried or the area doesn't exist.
    pub fn is_system_area(&mut self, area_id: i64) -> Result<bool, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::is_system_area_sqlite(conn, area_id),
            BackendConnection::Mysql(conn) => queries::is_system_area_mysql(conn, area_id),
        }
    }

    /// Updates an area's display name.
    ///
    /// Phase 26C: Used to edit area metadata (display name only, not area code).
    ///
    /// # Arguments
    ///
    /// * `area_id` - The canonical area ID
    /// * `area_name` - The new display name (or `None` to clear)
    ///
    /// # Errors
    ///
    /// Returns an error if the area doesn't exist or the database operation fails.
    pub fn update_area_name(
        &mut self,
        area_id: i64,
        area_name: Option<&str>,
    ) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::update_area_name_sqlite(conn, area_id, area_name)
            }
            BackendConnection::Mysql(conn) => {
                mutations::update_area_name_mysql(conn, area_id, area_name)
            }
        }
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
    pub fn should_snapshot(&self, action_name: &str) -> bool {
        queries::should_snapshot(action_name)
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::create_operator_sqlite(conn, login_name, display_name, password, role)
            }
            BackendConnection::Mysql(conn) => {
                mutations::create_operator_mysql(conn, login_name, display_name, password, role)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::operators::get_operator_by_login_sqlite(conn, login_name)
            }
            BackendConnection::Mysql(conn) => {
                queries::operators::get_operator_by_login_mysql(conn, login_name)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::operators::get_operator_by_id_sqlite(conn, operator_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::operators::get_operator_by_id_mysql(conn, operator_id)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::update_last_login_sqlite(conn, operator_id)
            }
            BackendConnection::Mysql(conn) => mutations::update_last_login_mysql(conn, operator_id),
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::disable_operator_sqlite(conn, operator_id)
            }
            BackendConnection::Mysql(conn) => mutations::disable_operator_mysql(conn, operator_id),
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => mutations::enable_operator_sqlite(conn, operator_id),
            BackendConnection::Mysql(conn) => mutations::enable_operator_mysql(conn, operator_id),
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => mutations::delete_operator_sqlite(conn, operator_id),
            BackendConnection::Mysql(conn) => mutations::delete_operator_mysql(conn, operator_id),
        }
    }

    /// Lists all operators.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn list_operators(&mut self) -> Result<Vec<OperatorData>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::operators::list_operators_sqlite(conn),
            BackendConnection::Mysql(conn) => queries::operators::list_operators_mysql(conn),
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::operators::is_operator_referenced_sqlite(conn, operator_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::operators::is_operator_referenced_mysql(conn, operator_id)
            }
        }
    }

    /// Counts the total number of operators.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn count_operators(&mut self) -> Result<i64, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::operators::count_operators_sqlite(conn),
            BackendConnection::Mysql(conn) => queries::operators::count_operators_mysql(conn),
        }
    }

    /// Counts the number of active admin operators.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn count_active_admin_operators(&mut self) -> Result<i64, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::operators::count_active_admin_operators_sqlite(conn)
            }
            BackendConnection::Mysql(conn) => {
                queries::operators::count_active_admin_operators_mysql(conn)
            }
        }
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
        queries::operators::verify_password(password, password_hash)
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::update_password_sqlite(conn, operator_id, new_password)
            }
            BackendConnection::Mysql(conn) => {
                mutations::update_password_mysql(conn, operator_id, new_password)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::delete_sessions_for_operator_sqlite(conn, operator_id)
            }
            BackendConnection::Mysql(conn) => {
                mutations::delete_sessions_for_operator_mysql(conn, operator_id)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::create_session_sqlite(conn, session_token, operator_id, expires_at)
            }
            BackendConnection::Mysql(conn) => {
                mutations::create_session_mysql(conn, session_token, operator_id, expires_at)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::operators::get_session_by_token_sqlite(conn, session_token)
            }
            BackendConnection::Mysql(conn) => {
                queries::operators::get_session_by_token_mysql(conn, session_token)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::update_session_activity_sqlite(conn, session_id)
            }
            BackendConnection::Mysql(conn) => {
                mutations::update_session_activity_mysql(conn, session_id)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::delete_session_sqlite(conn, session_token)
            }
            BackendConnection::Mysql(conn) => mutations::delete_session_mysql(conn, session_token),
        }
    }

    /// Deletes all expired sessions.
    ///
    /// # Errors
    ///
    /// Returns an error if the database delete fails.
    pub fn delete_expired_sessions(&mut self) -> Result<usize, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => mutations::delete_expired_sessions_sqlite(conn),
            BackendConnection::Mysql(conn) => mutations::delete_expired_sessions_mysql(conn),
        }
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
    pub fn set_active_bid_year(&mut self, bid_year: &BidYear) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                mutations::set_active_bid_year_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                mutations::set_active_bid_year_mysql(conn, bid_year_id)
            }
        }
    }

    /// Gets the active bid year.
    ///
    /// # Errors
    ///
    /// Returns an error if no active bid year exists.
    pub fn get_active_bid_year(&mut self) -> Result<u16, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::canonical::get_active_bid_year_sqlite(conn),
            BackendConnection::Mysql(conn) => queries::canonical::get_active_bid_year_mysql(conn),
        }
    }

    /// Sets the expected area count for a bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `count` - The expected number of areas
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be updated or the bid year doesn't exist.
    pub fn set_expected_area_count(
        &mut self,
        bid_year: &BidYear,
        count: usize,
    ) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                mutations::set_expected_area_count_sqlite(conn, bid_year_id, count)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                mutations::set_expected_area_count_mysql(conn, bid_year_id, count)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                queries::canonical::get_expected_area_count_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                queries::canonical::get_expected_area_count_mysql(conn, bid_year_id)
            }
        }
    }

    /// Sets the expected user count for an area.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    /// * `count` - The expected number of users
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be updated or the area doesn't exist.
    pub fn set_expected_user_count(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
        count: usize,
    ) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                let area_id =
                    queries::canonical::lookup_area_id_sqlite(conn, bid_year_id, area.id())?;
                mutations::set_expected_user_count_sqlite(conn, bid_year_id, area_id, count)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                let area_id =
                    queries::canonical::lookup_area_id_mysql(conn, bid_year_id, area.id())?;
                mutations::set_expected_user_count_mysql(conn, bid_year_id, area_id, count)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                let area_id =
                    queries::canonical::lookup_area_id_sqlite(conn, bid_year_id, area.id())?;
                queries::canonical::get_expected_user_count_sqlite(conn, bid_year_id, area_id)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                let area_id =
                    queries::canonical::lookup_area_id_mysql(conn, bid_year_id, area.id())?;
                queries::canonical::get_expected_user_count_mysql(conn, bid_year_id, area_id)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                queries::canonical::get_actual_area_count_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                queries::canonical::get_actual_area_count_mysql(conn, bid_year_id)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_sqlite(conn, bid_year.year())?;
                let area_id =
                    queries::canonical::lookup_area_id_sqlite(conn, bid_year_id, area.id())?;
                queries::canonical::get_actual_user_count_sqlite(conn, bid_year_id, area_id)
            }
            BackendConnection::Mysql(conn) => {
                let bid_year_id =
                    queries::canonical::lookup_bid_year_id_mysql(conn, bid_year.year())?;
                let area_id =
                    queries::canonical::lookup_area_id_mysql(conn, bid_year_id, area.id())?;
                queries::canonical::get_actual_user_count_mysql(conn, bid_year_id, area_id)
            }
        }
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
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => mutations::update_user_sqlite(
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
            ),
            BackendConnection::Mysql(conn) => mutations::update_user_mysql(
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
            ),
        }
    }

    /// Creates a system area (e.g., "No Bid") for a bid year.
    ///
    /// Phase 25B: System areas are auto-created and cannot be deleted or renamed.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `area_code` - The area code (e.g., "NO BID")
    ///
    /// # Returns
    ///
    /// The generated `area_id` for the new system area.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn create_system_area(
        &mut self,
        bid_year_id: i64,
        area_code: &str,
    ) -> Result<i64, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::create_system_area_sqlite(conn, bid_year_id, area_code)
            }
            BackendConnection::Mysql(conn) => {
                mutations::create_system_area_mysql(conn, bid_year_id, area_code)
            }
        }
    }

    /// Gets the lifecycle state for a bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    ///
    /// # Errors
    ///
    /// Returns an error if the bid year doesn't exist or the database cannot be queried.
    pub fn get_lifecycle_state(&mut self, bid_year_id: i64) -> Result<String, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::canonical::get_lifecycle_state_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::canonical::get_lifecycle_state_mysql(conn, bid_year_id)
            }
        }
    }

    /// Updates the lifecycle state for a bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `new_state` - The new lifecycle state as a string
    ///
    /// # Errors
    ///
    /// Returns an error if the bid year doesn't exist or the database cannot be updated.
    pub fn update_lifecycle_state(
        &mut self,
        bid_year_id: i64,
        new_state: &str,
    ) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::canonical::update_lifecycle_state_sqlite(conn, bid_year_id, new_state)
            }
            BackendConnection::Mysql(conn) => {
                queries::canonical::update_lifecycle_state_mysql(conn, bid_year_id, new_state)
            }
        }
    }

    /// Retrieves the metadata (label and notes) for a bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    ///
    /// # Errors
    ///
    /// Returns an error if the bid year doesn't exist or the database cannot be queried.
    pub fn get_bid_year_metadata(
        &mut self,
        bid_year_id: i64,
    ) -> Result<(Option<String>, Option<String>), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::canonical::get_bid_year_metadata_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::canonical::get_bid_year_metadata_mysql(conn, bid_year_id)
            }
        }
    }

    /// Updates the metadata fields (label and notes) for a bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `label` - Optional display label (max 100 characters)
    /// * `notes` - Optional operational notes (max 2000 characters)
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be updated or the bid year doesn't exist.
    pub fn update_bid_year_metadata(
        &mut self,
        bid_year_id: i64,
        label: Option<&str>,
        notes: Option<&str>,
    ) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::bootstrap::update_bid_year_metadata_sqlite(
                    conn,
                    bid_year_id,
                    label,
                    notes,
                )
            }
            BackendConnection::Mysql(conn) => mutations::bootstrap::update_bid_year_metadata_mysql(
                conn,
                bid_year_id,
                label,
                notes,
            ),
        }
    }

    /// Retrieves the bid schedule for a bid year.
    ///
    /// Phase 29C: Returns bid schedule fields if set, or None values if not configured.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    ///
    /// # Errors
    ///
    /// Returns an error if the bid year doesn't exist or the database cannot be queried.
    pub fn get_bid_schedule(
        &mut self,
        bid_year_id: i64,
    ) -> Result<mutations::bootstrap::BidScheduleFields, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::bootstrap::get_bid_schedule_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                mutations::bootstrap::get_bid_schedule_mysql(conn, bid_year_id)
            }
        }
    }

    /// Updates the bid schedule for a bid year.
    ///
    /// Phase 29C: Sets all bid schedule fields atomically.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `timezone` - IANA timezone identifier
    /// * `start_date` - Bid start date (ISO 8601 format)
    /// * `window_start_time` - Daily window start time (HH:MM:SS format)
    /// * `window_end_time` - Daily window end time (HH:MM:SS format)
    /// * `bidders_per_day` - Number of bidders per area per day
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be updated or the bid year doesn't exist.
    pub fn update_bid_schedule(
        &mut self,
        bid_year_id: i64,
        timezone: Option<&str>,
        start_date: Option<&str>,
        window_start_time: Option<&str>,
        window_end_time: Option<&str>,
        bidders_per_day: Option<i32>,
    ) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => mutations::bootstrap::update_bid_schedule_sqlite(
                conn,
                bid_year_id,
                timezone,
                start_date,
                window_start_time,
                window_end_time,
                bidders_per_day,
            ),
            BackendConnection::Mysql(conn) => mutations::bootstrap::update_bid_schedule_mysql(
                conn,
                bid_year_id,
                timezone,
                start_date,
                window_start_time,
                window_end_time,
                bidders_per_day,
            ),
        }
    }

    /// Queries whether any bid year is in the `BiddingActive` lifecycle state.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(year))` if a bid year is `BiddingActive`
    /// * `Ok(None)` if no bid year is `BiddingActive`
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_bidding_active_year(&mut self) -> Result<Option<u16>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::canonical::get_bidding_active_year_sqlite(conn)
            }
            BackendConnection::Mysql(conn) => {
                queries::canonical::get_bidding_active_year_mysql(conn)
            }
        }
    }

    // ========================================================================
    // Canonical ID Lookups (Test Support)
    // ========================================================================

    /// Queries the canonical `bid_year_id` for a given year.
    /// Get the year value for a given canonical bid year ID.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID to query
    ///
    /// # Errors
    ///
    /// Returns an error if the bid year is not found or the query fails.
    pub fn get_bid_year_from_id(&mut self, bid_year_id: i64) -> Result<u16, PersistenceError> {
        use diesel_schema::bid_years;

        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let result: Result<i32, diesel::result::Error> = bid_years::table
                    .select(bid_years::year)
                    .filter(bid_years::bid_year_id.eq(bid_year_id))
                    .first::<i32>(conn);

                match result {
                    Ok(year) => Ok(u16::try_from(year).map_err(|e| {
                        PersistenceError::Other(format!("Invalid year value: {e}"))
                    })?),
                    Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(
                        format!("Bid year with ID {bid_year_id} does not exist"),
                    )),
                    Err(e) => Err(PersistenceError::from(e)),
                }
            }
            BackendConnection::Mysql(conn) => {
                let result: Result<i32, diesel::result::Error> = bid_years::table
                    .select(bid_years::year)
                    .filter(bid_years::bid_year_id.eq(bid_year_id))
                    .first::<i32>(conn);

                match result {
                    Ok(year) => Ok(u16::try_from(year).map_err(|e| {
                        PersistenceError::Other(format!("Invalid year value: {e}"))
                    })?),
                    Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(
                        format!("Bid year with ID {bid_year_id} does not exist"),
                    )),
                    Err(e) => Err(PersistenceError::from(e)),
                }
            }
        }
    }

    /// Get the canonical bid year ID for a given year.
    ///
    /// # Arguments
    ///
    /// * `year` - The year to query
    ///
    /// # Errors
    ///
    /// Returns an error if the bid year is not found or the query fails.
    pub fn get_bid_year_id(&mut self, year: u16) -> Result<i64, PersistenceError> {
        use diesel_schema::bid_years;

        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let result: Result<i64, diesel::result::Error> = bid_years::table
                    .select(bid_years::bid_year_id)
                    .filter(bid_years::year.eq(i32::from(year)))
                    .first::<i64>(conn);

                match result {
                    Ok(id) => Ok(id),
                    Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(
                        format!("Bid year {year} does not exist"),
                    )),
                    Err(e) => Err(PersistenceError::from(e)),
                }
            }
            BackendConnection::Mysql(conn) => {
                let result: Result<i64, diesel::result::Error> = bid_years::table
                    .select(bid_years::bid_year_id)
                    .filter(bid_years::year.eq(i32::from(year)))
                    .first::<i64>(conn);

                match result {
                    Ok(id) => Ok(id),
                    Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(
                        format!("Bid year {year} does not exist"),
                    )),
                    Err(e) => Err(PersistenceError::from(e)),
                }
            }
        }
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
        use diesel_schema::areas;

        let normalized_code: String = area_code.to_uppercase();

        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                let result: Result<i64, diesel::result::Error> = areas::table
                    .select(areas::area_id)
                    .filter(areas::bid_year_id.eq(bid_year_id))
                    .filter(areas::area_code.eq(&normalized_code))
                    .first::<i64>(conn);

                match result {
                    Ok(id) => Ok(id),
                    Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(
                        format!("Area {area_code} does not exist"),
                    )),
                    Err(e) => Err(PersistenceError::from(e)),
                }
            }
            BackendConnection::Mysql(conn) => {
                let result: Result<i64, diesel::result::Error> = areas::table
                    .select(areas::area_id)
                    .filter(areas::bid_year_id.eq(bid_year_id))
                    .filter(areas::area_code.eq(&normalized_code))
                    .first::<i64>(conn);

                match result {
                    Ok(id) => Ok(id),
                    Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(
                        format!("Area {area_code} does not exist"),
                    )),
                    Err(e) => Err(PersistenceError::from(e)),
                }
            }
        }
    }

    /// Canonicalizes a bid year by populating canonical data tables.
    ///
    /// This function persists the audit event and creates canonical rows for:
    /// - Area membership
    /// - Eligibility
    /// - Bid order (NULL until computed)
    /// - Bid windows (NULL until computed)
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The bid year to canonicalize
    /// * `audit_event` - The audit event recording canonicalization
    ///
    /// # Returns
    ///
    /// The `event_id` of the persisted audit event.
    ///
    /// # Errors
    ///
    /// Returns an error if any database operation fails.
    pub fn canonicalize_bid_year(
        &mut self,
        bid_year_id: i64,
        audit_event: &zab_bid_audit::AuditEvent,
    ) -> Result<i64, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::bootstrap::canonicalize_bid_year_sqlite(conn, bid_year_id, audit_event)
            }
            BackendConnection::Mysql(conn) => {
                mutations::bootstrap::canonicalize_bid_year_mysql(conn, bid_year_id, audit_event)
            }
        }
    }

    /// Lists users with lifecycle-aware routing.
    ///
    /// Phase 25C: Routes reads to canonical or derived tables based on lifecycle state.
    ///
    /// When `lifecycle_state >= Canonicalized`, reads come from canonical tables.
    /// When `lifecycle_state < Canonicalized`, reads come from the users table.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `area_id` - The canonical area ID
    /// * `bid_year` - The `BidYear` domain object
    /// * `area` - The Area domain object
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The database cannot be queried
    /// - Canonical data is missing when lifecycle >= Canonicalized
    pub fn list_users_with_routing(
        &mut self,
        bid_year_id: i64,
        area_id: i64,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<Vec<User>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::canonical::list_users_with_routing_sqlite(
                conn,
                bid_year_id,
                area_id,
                bid_year,
                area,
            ),
            BackendConnection::Mysql(conn) => queries::canonical::list_users_with_routing_mysql(
                conn,
                bid_year_id,
                area_id,
                bid_year,
                area,
            ),
        }
    }

    /// Override a user's area assignment after canonicalization.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `user_id` - The canonical user ID
    /// * `new_area_id` - The new area ID to assign
    /// * `reason` - The reason for the override
    ///
    /// # Returns
    ///
    /// Returns a tuple of (`previous_area_id`, `was_already_overridden`).
    ///
    /// # Errors
    ///
    /// Returns an error if the canonical record does not exist or the database operation fails.
    pub fn override_area_assignment(
        &mut self,
        bid_year_id: i64,
        user_id: i64,
        new_area_id: i64,
        reason: &str,
    ) -> Result<(i64, bool), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                mutations::canonical::override_area_assignment_sqlite(
                    conn,
                    bid_year_id,
                    user_id,
                    new_area_id,
                    reason,
                )
            }
            BackendConnection::Mysql(conn) => mutations::canonical::override_area_assignment_mysql(
                conn,
                bid_year_id,
                user_id,
                new_area_id,
                reason,
            ),
        }
    }

    /// Override a user's eligibility after canonicalization.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `user_id` - The canonical user ID
    /// * `can_bid` - The new eligibility status
    /// * `reason` - The reason for the override
    ///
    /// # Returns
    ///
    /// Returns a tuple of (`previous_can_bid`, `was_already_overridden`).
    ///
    /// # Errors
    ///
    /// Returns an error if the canonical record does not exist or the database operation fails.
    pub fn override_eligibility(
        &mut self,
        bid_year_id: i64,
        user_id: i64,
        can_bid: bool,
        reason: &str,
    ) -> Result<(bool, bool), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => mutations::canonical::override_eligibility_sqlite(
                conn,
                bid_year_id,
                user_id,
                can_bid,
                reason,
            ),
            BackendConnection::Mysql(conn) => mutations::canonical::override_eligibility_mysql(
                conn,
                bid_year_id,
                user_id,
                can_bid,
                reason,
            ),
        }
    }

    /// Override a user's bid order after canonicalization.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `user_id` - The canonical user ID
    /// * `bid_order` - The new bid order (or `None` to clear)
    /// * `reason` - The reason for the override
    ///
    /// # Returns
    ///
    /// Returns a tuple of (`previous_bid_order`, `was_already_overridden`).
    ///
    /// # Errors
    ///
    /// Returns an error if the canonical record does not exist or the database operation fails.
    pub fn override_bid_order(
        &mut self,
        bid_year_id: i64,
        user_id: i64,
        bid_order: Option<i32>,
        reason: &str,
    ) -> Result<(Option<i32>, bool), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => mutations::canonical::override_bid_order_sqlite(
                conn,
                bid_year_id,
                user_id,
                bid_order,
                reason,
            ),
            BackendConnection::Mysql(conn) => mutations::canonical::override_bid_order_mysql(
                conn,
                bid_year_id,
                user_id,
                bid_order,
                reason,
            ),
        }
    }

    /// Override a user's bid window after canonicalization.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `user_id` - The canonical user ID
    /// * `window_start` - The new window start date (or `None` to clear)
    /// * `window_end` - The new window end date (or `None` to clear)
    /// * `reason` - The reason for the override
    ///
    /// # Returns
    ///
    /// Returns a tuple of (`previous_window_start`, `previous_window_end`, `was_already_overridden`).
    ///
    /// # Errors
    ///
    /// Returns an error if the canonical record does not exist or the database operation fails.
    pub fn override_bid_window(
        &mut self,
        bid_year_id: i64,
        user_id: i64,
        window_start: Option<&String>,
        window_end: Option<&String>,
        reason: &str,
    ) -> Result<(Option<String>, Option<String>, bool), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => mutations::canonical::override_bid_window_sqlite(
                conn,
                bid_year_id,
                user_id,
                window_start,
                window_end,
                reason,
            ),
            BackendConnection::Mysql(conn) => mutations::canonical::override_bid_window_mysql(
                conn,
                bid_year_id,
                user_id,
                window_start,
                window_end,
                reason,
            ),
        }
    }

    /// Get user details for override operations.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The canonical user ID
    ///
    /// # Returns
    ///
    /// Returns a tuple of (`bid_year_id`, `user_initials`).
    ///
    /// # Errors
    ///
    /// Returns an error if the user does not exist or the database operation fails.
    pub fn get_user_details(&mut self, user_id: i64) -> Result<(i64, String), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::canonical::get_user_details_for_override_sqlite(conn, user_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::canonical::get_user_details_for_override_mysql(conn, user_id)
            }
        }
    }

    /// Get the area ID for a user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The canonical user ID
    ///
    /// # Returns
    ///
    /// Returns the `area_id` where the user is currently assigned.
    ///
    /// # Errors
    ///
    /// Returns an error if the user does not exist or the database operation fails.
    pub fn get_user_area_id(&mut self, user_id: i64) -> Result<i64, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::canonical::get_user_area_id_sqlite(conn, user_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::canonical::get_user_area_id_mysql(conn, user_id)
            }
        }
    }

    /// Get area details for override operations.
    ///
    /// # Arguments
    ///
    /// * `area_id` - The canonical area ID
    ///
    /// # Returns
    ///
    /// Returns a tuple of (`area_code`, `area_name`).
    ///
    /// # Errors
    ///
    /// Returns an error if the area does not exist or the database operation fails.
    pub fn get_area_details(
        &mut self,
        area_id: i64,
    ) -> Result<(String, Option<String>), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::canonical::get_area_details_for_override_sqlite(conn, area_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::canonical::get_area_details_for_override_mysql(conn, area_id)
            }
        }
    }

    /// Get current canonical area assignment for a user.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    /// * `user_id` - The canonical user ID
    ///
    /// # Returns
    ///
    /// Returns the current `area_id`.
    ///
    /// # Errors
    ///
    /// Returns an error if the canonical record does not exist or the database operation fails.
    pub fn get_current_area_assignment(
        &mut self,
        bid_year_id: i64,
        user_id: i64,
    ) -> Result<i64, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::canonical::get_current_area_assignment_for_override_sqlite(
                    conn,
                    bid_year_id,
                    user_id,
                )
            }
            BackendConnection::Mysql(conn) => {
                queries::canonical::get_current_area_assignment_for_override_mysql(
                    conn,
                    bid_year_id,
                    user_id,
                )
            }
        }
    }

    // ========================================================================
    // Phase 29B: Round Groups and Rounds
    // ========================================================================

    /// Lists all round groups for a bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The bid year ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn list_round_groups(
        &mut self,
        bid_year_id: i64,
    ) -> Result<Vec<RoundGroup>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::rounds::list_round_groups_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::rounds::list_round_groups_mysql(conn, bid_year_id)
            }
        }
    }

    /// Gets a single round group by ID.
    ///
    /// # Arguments
    ///
    /// * `round_group_id` - The round group ID
    ///
    /// # Errors
    ///
    /// Returns an error if the round group does not exist or the query fails.
    pub fn get_round_group(&mut self, round_group_id: i64) -> Result<RoundGroup, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::rounds::get_round_group_sqlite(conn, round_group_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::rounds::get_round_group_mysql(conn, round_group_id)
            }
        }
    }

    /// Inserts a new round group.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The bid year ID
    /// * `name` - The round group name
    /// * `editing_enabled` - Whether editing is enabled
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails.
    pub fn insert_round_group(
        &mut self,
        bid_year_id: i64,
        name: &str,
        editing_enabled: bool,
    ) -> Result<i64, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::rounds::insert_round_group_sqlite(conn, bid_year_id, name, editing_enabled)
            }
            BackendConnection::Mysql(conn) => {
                queries::rounds::insert_round_group_mysql(conn, bid_year_id, name, editing_enabled)
            }
        }
    }

    /// Updates an existing round group.
    ///
    /// # Arguments
    ///
    /// * `round_group_id` - The round group ID
    /// * `name` - The new name
    /// * `editing_enabled` - The new `editing_enabled` value
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    pub fn update_round_group(
        &mut self,
        round_group_id: i64,
        name: &str,
        editing_enabled: bool,
    ) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::rounds::update_round_group_sqlite(
                conn,
                round_group_id,
                name,
                editing_enabled,
            ),
            BackendConnection::Mysql(conn) => queries::rounds::update_round_group_mysql(
                conn,
                round_group_id,
                name,
                editing_enabled,
            ),
        }
    }

    /// Deletes a round group.
    ///
    /// # Arguments
    ///
    /// * `round_group_id` - The round group ID
    ///
    /// # Errors
    ///
    /// Returns an error if the delete fails.
    pub fn delete_round_group(&mut self, round_group_id: i64) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::rounds::delete_round_group_sqlite(conn, round_group_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::rounds::delete_round_group_mysql(conn, round_group_id)
            }
        }
    }

    /// Checks if a round group is referenced by any rounds.
    ///
    /// # Arguments
    ///
    /// * `round_group_id` - The round group ID
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn count_rounds_using_group(
        &mut self,
        round_group_id: i64,
    ) -> Result<usize, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::rounds::count_rounds_using_group_sqlite(conn, round_group_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::rounds::count_rounds_using_group_mysql(conn, round_group_id)
            }
        }
    }

    /// Checks if a round group name exists within a bid year.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The bid year ID
    /// * `name` - The round group name
    /// * `exclude_id` - Optional round group ID to exclude from the check
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn round_group_name_exists(
        &mut self,
        bid_year_id: i64,
        name: &str,
        exclude_id: Option<i64>,
    ) -> Result<bool, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::rounds::round_group_name_exists_sqlite(conn, bid_year_id, name, exclude_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::rounds::round_group_name_exists_mysql(conn, bid_year_id, name, exclude_id)
            }
        }
    }

    /// Lists all rounds for a given round group.
    ///
    /// # Arguments
    ///
    /// * `round_group_id` - The round group ID
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn list_rounds(&mut self, round_group_id: i64) -> Result<Vec<Round>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::rounds::list_rounds_sqlite(conn, round_group_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::rounds::list_rounds_mysql(conn, round_group_id)
            }
        }
    }

    /// Gets a single round by ID.
    ///
    /// # Arguments
    ///
    /// * `round_id` - The round ID
    ///
    /// # Errors
    ///
    /// Returns an error if the round does not exist or the query fails.
    pub fn get_round(&mut self, round_id: i64) -> Result<Round, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::rounds::get_round_sqlite(conn, round_id),
            BackendConnection::Mysql(conn) => queries::rounds::get_round_mysql(conn, round_id),
        }
    }

    /// Inserts a new round.
    ///
    /// # Arguments
    ///
    /// * `round_group_id` - The round group ID
    /// * `round_number` - The round number
    /// * `name` - The round name
    /// * `slots_per_day` - Slots per day
    /// * `max_groups` - Maximum groups
    /// * `max_total_hours` - Maximum total hours
    /// * `include_holidays` - Whether holidays are included
    /// * `allow_overbid` - Whether overbidding is allowed
    ///
    /// # Errors
    ///
    /// Returns an error if the insert fails.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_round(
        &mut self,
        round_group_id: i64,
        round_number: u32,
        name: &str,
        slots_per_day: u32,
        max_groups: u32,
        max_total_hours: u32,
        include_holidays: bool,
        allow_overbid: bool,
    ) -> Result<i64, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::rounds::insert_round_sqlite(
                conn,
                round_group_id,
                round_number,
                name,
                slots_per_day,
                max_groups,
                max_total_hours,
                include_holidays,
                allow_overbid,
            ),
            BackendConnection::Mysql(conn) => queries::rounds::insert_round_mysql(
                conn,
                round_group_id,
                round_number,
                name,
                slots_per_day,
                max_groups,
                max_total_hours,
                include_holidays,
                allow_overbid,
            ),
        }
    }

    /// Updates an existing round.
    ///
    /// # Arguments
    ///
    /// * `round_id` - The round ID
    /// * `name` - The new name
    /// * `slots_per_day` - The new `slots_per_day`
    /// * `max_groups` - The new `max_groups`
    /// * `max_total_hours` - The new `max_total_hours`
    /// * `include_holidays` - The new `include_holidays`
    /// * `allow_overbid` - The new `allow_overbid`
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    #[allow(clippy::too_many_arguments)]
    pub fn update_round(
        &mut self,
        round_id: i64,
        name: &str,
        slots_per_day: u32,
        max_groups: u32,
        max_total_hours: u32,
        include_holidays: bool,
        allow_overbid: bool,
    ) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::rounds::update_round_sqlite(
                conn,
                round_id,
                name,
                slots_per_day,
                max_groups,
                max_total_hours,
                include_holidays,
                allow_overbid,
            ),
            BackendConnection::Mysql(conn) => queries::rounds::update_round_mysql(
                conn,
                round_id,
                name,
                slots_per_day,
                max_groups,
                max_total_hours,
                include_holidays,
                allow_overbid,
            ),
        }
    }

    /// Deletes a round.
    ///
    /// # Arguments
    ///
    /// * `round_id` - The round ID
    ///
    /// # Errors
    ///
    /// Returns an error if the delete fails.
    pub fn delete_round(&mut self, round_id: i64) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::rounds::delete_round_sqlite(conn, round_id),
            BackendConnection::Mysql(conn) => queries::rounds::delete_round_mysql(conn, round_id),
        }
    }

    /// Checks if a round number exists within a round group.
    ///
    /// # Arguments
    ///
    /// * `round_group_id` - The round group ID
    /// * `round_number` - The round number
    /// * `exclude_id` - Optional round ID to exclude from the check
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub fn round_number_exists(
        &mut self,
        round_group_id: i64,
        round_number: u32,
        exclude_id: Option<i64>,
    ) -> Result<bool, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => queries::rounds::round_number_exists_sqlite(
                conn,
                round_group_id,
                round_number,
                exclude_id,
            ),
            BackendConnection::Mysql(conn) => queries::rounds::round_number_exists_mysql(
                conn,
                round_group_id,
                round_number,
                exclude_id,
            ),
        }
    }

    /// Gets an area by its canonical ID, returning both the Area and its `bid_year_id`.
    ///
    /// # Arguments
    ///
    /// * `area_id` - The canonical area ID
    ///
    /// # Errors
    ///
    /// Returns an error if the area does not exist or the query fails.
    pub fn get_area_by_id(&mut self, area_id: i64) -> Result<(Area, i64), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::canonical::get_area_by_id_sqlite(conn, area_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::canonical::get_area_by_id_mysql(conn, area_id)
            }
        }
    }

    // ========================================================================
    // Phase 29D: Readiness Evaluation
    // ========================================================================

    /// Checks if a bid year has a valid bid schedule configured.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    ///
    /// # Returns
    ///
    /// `true` if all bid schedule fields are set, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn is_bid_schedule_set(&mut self, bid_year_id: i64) -> Result<bool, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::readiness::is_bid_schedule_set_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::readiness::is_bid_schedule_set_mysql(conn, bid_year_id)
            }
        }
    }

    /// Gets non-system areas that have no rounds configured.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    ///
    /// # Returns
    ///
    /// Vector of area codes for areas missing round configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_areas_missing_rounds(
        &mut self,
        bid_year_id: i64,
    ) -> Result<Vec<String>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::readiness::get_areas_missing_rounds_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::readiness::get_areas_missing_rounds_mysql(conn, bid_year_id)
            }
        }
    }

    /// Counts users in system areas who have not been reviewed.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    ///
    /// # Returns
    ///
    /// Count of unreviewed users in system areas (No Bid).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_unreviewed_no_bid_users(
        &mut self,
        bid_year_id: i64,
    ) -> Result<i64, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::readiness::count_unreviewed_no_bid_users_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::readiness::count_unreviewed_no_bid_users_mysql(conn, bid_year_id)
            }
        }
    }

    /// Counts users violating the participation flag directional invariant.
    ///
    /// Invariant: `excluded_from_leave_calculation == true` ⇒ `excluded_from_bidding == true`
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    ///
    /// # Returns
    ///
    /// Count of users violating the invariant.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_participation_flag_violations(
        &mut self,
        bid_year_id: i64,
    ) -> Result<i64, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::readiness::count_participation_flag_violations_sqlite(conn, bid_year_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::readiness::count_participation_flag_violations_mysql(conn, bid_year_id)
            }
        }
    }

    /// Marks a user in a system area as reviewed.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user's canonical ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be updated.
    pub fn mark_user_no_bid_reviewed(&mut self, user_id: i64) -> Result<(), PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::readiness::mark_user_no_bid_reviewed_sqlite(conn, user_id)
            }
            BackendConnection::Mysql(conn) => {
                queries::readiness::mark_user_no_bid_reviewed_mysql(conn, user_id)
            }
        }
    }

    /// Gets all users grouped by area for seniority conflict detection.
    ///
    /// Returns users in non-system areas only.
    ///
    /// # Arguments
    ///
    /// * `bid_year_id` - The canonical bid year ID
    ///
    /// # Returns
    ///
    /// Vector of tuples containing (`area_id`, `area_code`, users in that area).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_users_by_area_for_conflict_detection(
        &mut self,
        bid_year_id: i64,
    ) -> Result<Vec<(i64, String, Vec<zab_bid_domain::User>)>, PersistenceError> {
        match &mut self.conn {
            BackendConnection::Sqlite(conn) => {
                queries::readiness::get_users_by_area_for_conflict_detection_sqlite(
                    conn,
                    bid_year_id,
                )
            }
            BackendConnection::Mysql(conn) => {
                queries::readiness::get_users_by_area_for_conflict_detection_mysql(
                    conn,
                    bid_year_id,
                )
            }
        }
    }
}
