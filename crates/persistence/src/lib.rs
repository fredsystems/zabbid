// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![deny(
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,
    clippy::style,
    clippy::correctness,
    clippy::all
)]

mod data_models;
mod error;
mod sqlite;

#[cfg(test)]
mod tests;

use rusqlite::Connection;
use std::path::Path;
use zab_bid::{BootstrapMetadata, BootstrapResult, State, TransitionResult};
use zab_bid_audit::AuditEvent;
use zab_bid_domain::{Area, BidYear, CanonicalBidYear, User};

pub use data_models::{OperatorData, SessionData};
pub use error::PersistenceError;

/// Persistence adapter for audit events and state snapshots.
pub struct SqlitePersistence {
    pub(crate) conn: Connection,
}

impl SqlitePersistence {
    /// Creates a new persistence adapter with an in-memory database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn new_in_memory() -> Result<Self, PersistenceError> {
        let conn: Connection = Connection::open_in_memory()?;
        // Enable foreign key constraints
        conn.pragma_update(None, "foreign_keys", "ON")?;
        sqlite::initialize_schema(&conn)?;
        Ok(Self { conn })
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
        let conn: Connection = Connection::open(path)?;
        // Enable WAL mode for better read concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // Enable foreign key constraints
        conn.pragma_update(None, "foreign_keys", "ON")?;
        sqlite::initialize_schema(&conn)?;
        // Verify foreign key enforcement is active
        sqlite::verify_foreign_key_enforcement(&conn)?;
        Ok(Self { conn })
    }

    /// Verifies that foreign key enforcement is enabled.
    ///
    /// This is a startup-time check required by Phase 14 to ensure
    /// referential integrity constraints are enforced.
    ///
    /// # Errors
    ///
    /// Returns an error if foreign key enforcement is not enabled.
    pub fn verify_foreign_key_enforcement(&self) -> Result<(), PersistenceError> {
        sqlite::verify_foreign_key_enforcement(&self.conn)
    }

    /// Persists a transition result (audit event and optionally a full snapshot).
    ///
    /// # Arguments
    ///
    /// * `result` - The transition result to persist
    /// * `should_snapshot` - Whether to persist a full state snapshot
    ///
    /// # Returns
    ///
    /// The event ID assigned to the persisted audit event.
    ///
    /// # Errors
    ///
    /// Returns an error if persistence fails. No partial writes occur.
    pub fn persist_transition(
        &mut self,
        result: &TransitionResult,
        should_snapshot: bool,
    ) -> Result<i64, PersistenceError> {
        let tx = self.conn.transaction()?;
        let event_id: i64 = sqlite::persist_transition(&tx, result, should_snapshot)?;
        tx.commit()?;
        Ok(event_id)
    }

    /// Persists a standalone audit event.
    ///
    /// This is used for operator lifecycle events and other system-level actions
    /// that don't go through the standard transition flow.
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
        let tx = self.conn.transaction()?;
        let event_id: i64 = sqlite::persist_audit_event(&tx, event)?;
        tx.commit()?;
        Ok(event_id)
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
    /// Returns an error if persistence fails. No partial writes occur.
    pub fn persist_bootstrap(&mut self, result: &BootstrapResult) -> Result<i64, PersistenceError> {
        let tx = self.conn.transaction()?;
        let event_id: i64 = sqlite::persist_bootstrap(&tx, result)?;
        tx.commit()?;
        Ok(event_id)
    }

    /// Retrieves an audit event by ID.
    ///
    /// # Arguments
    ///
    /// * `event_id` - The event ID to retrieve
    ///
    /// # Errors
    ///
    /// Returns an error if the event is not found or cannot be deserialized.
    pub fn get_audit_event(&self, event_id: i64) -> Result<AuditEvent, PersistenceError> {
        sqlite::get_audit_event(&self.conn, event_id)
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
        &self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<(State, i64), PersistenceError> {
        sqlite::get_latest_snapshot(&self.conn, bid_year, area)
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
        &self,
        bid_year: &BidYear,
        area: &Area,
        after_event_id: i64,
    ) -> Result<Vec<AuditEvent>, PersistenceError> {
        sqlite::get_events_after(&self.conn, bid_year, area, after_event_id)
    }

    /// Retrieves the current effective state for a given `(bid_year, area)` scope.
    ///
    /// This queries the canonical `users` table to reconstruct the current state.
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
        &self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<State, PersistenceError> {
        sqlite::get_current_state(&self.conn, bid_year, area)
    }

    /// Retrieves the effective state for a given `(bid_year, area)` scope at a specific timestamp.
    ///
    /// This is a read-only operation that returns the most recent snapshot at or before
    /// the target timestamp. In the current implementation, snapshots represent complete
    /// state at specific points, and non-snapshot events are for audit trail purposes only.
    ///
    /// If the timestamp does not correspond exactly to a snapshot, the most recent
    /// prior snapshot defines the state.
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
        &self,
        bid_year: &BidYear,
        area: &Area,
        timestamp: &str,
    ) -> Result<State, PersistenceError> {
        sqlite::get_historical_state(&self.conn, bid_year, area, timestamp)
    }

    /// Retrieves the ordered audit event timeline for a given `(bid_year, area)` scope.
    ///
    /// This is a read-only operation that returns all audit events in strict
    /// chronological order. Rollback events appear as first-class events in the timeline.
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
        &self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<Vec<AuditEvent>, PersistenceError> {
        sqlite::get_audit_timeline(&self.conn, bid_year, area)
    }

    /// Reconstructs bootstrap metadata from canonical tables.
    ///
    /// This method queries the canonical `bid_years` and `areas` tables to retrieve
    /// the set of bid years and areas that have been created.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    ///
    /// # Panics
    ///
    /// Panics if a bid year value from the database is outside the valid `u16` range.
    /// This should not occur in normal operation as bid years are validated on creation.
    pub fn get_bootstrap_metadata(&self) -> Result<BootstrapMetadata, PersistenceError> {
        sqlite::get_bootstrap_metadata(&self.conn)
    }

    /// Lists all bid years that have been created with their canonical metadata.
    ///
    /// This queries the canonical `bid_years` table directly and returns full
    /// canonical bid year definitions including start date and pay period count.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried or if the data cannot
    /// be reconstructed into valid `CanonicalBidYear` instances.
    ///
    /// # Panics
    ///
    /// Panics if a bid year value from the database cannot be converted to `u16`.
    /// This should never happen in practice as the schema enforces valid ranges.
    pub fn list_bid_years(&self) -> Result<Vec<CanonicalBidYear>, PersistenceError> {
        sqlite::list_bid_years(&self.conn)
    }

    /// Lists all areas for a given bid year.
    ///
    /// This queries the canonical `areas` table directly.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year to list areas for
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn list_areas(&self, bid_year: &BidYear) -> Result<Vec<Area>, PersistenceError> {
        sqlite::list_areas(&self.conn, bid_year)
    }

    /// Lists all users for a given `(bid_year, area)` scope.
    ///
    /// This queries the canonical `users` table directly.
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
        &self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<Vec<User>, PersistenceError> {
        sqlite::list_users(&self.conn, bid_year, area)
    }

    /// Counts users per area for a given bid year.
    ///
    /// Returns a vector of tuples containing (`area_id`, `user_count`).
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year to count users for
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_users_by_area(
        &self,
        bid_year: &BidYear,
    ) -> Result<Vec<(String, usize)>, PersistenceError> {
        sqlite::count_users_by_area(&self.conn, bid_year)
    }

    /// Counts areas per bid year.
    ///
    /// Returns a vector of tuples containing (`bid_year`, `area_count`).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_areas_by_bid_year(&self) -> Result<Vec<(u16, usize)>, PersistenceError> {
        sqlite::count_areas_by_bid_year(&self.conn)
    }

    /// Counts total users per bid year across all areas.
    ///
    /// Returns a vector of tuples containing (`bid_year`, `total_user_count`).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_users_by_bid_year(&self) -> Result<Vec<(u16, usize)>, PersistenceError> {
        sqlite::count_users_by_bid_year(&self.conn)
    }

    /// Counts users per (`bid_year`, `area_id`) combination.
    ///
    /// Returns a vector of tuples containing (`bid_year`, `area_id`, `user_count`).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn count_users_by_bid_year_and_area(
        &self,
    ) -> Result<Vec<(u16, String, usize)>, PersistenceError> {
        sqlite::count_users_by_bid_year_and_area(&self.conn)
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
    pub fn should_snapshot(action_name: &str) -> bool {
        sqlite::should_snapshot(action_name)
    }

    // ========================================================================
    // Operator and Session Management (Phase 14)
    // ========================================================================

    /// Creates a new operator.
    ///
    /// The `login_name` is normalized to uppercase for case-insensitive uniqueness.
    ///
    /// # Arguments
    ///
    /// * `login_name` - The login name (will be normalized)
    /// * `display_name` - The display name
    /// * `role` - The role (Admin or Bidder)
    ///
    /// # Errors
    ///
    /// Returns an error if the operator cannot be created or if the login name
    /// already exists.
    pub fn create_operator(
        &mut self,
        login_name: &str,
        display_name: &str,
        password: &str,
        role: &str,
    ) -> Result<i64, PersistenceError> {
        sqlite::create_operator(&self.conn, login_name, display_name, password, role)
    }

    /// Retrieves an operator by login name.
    ///
    /// The `login_name` is normalized to uppercase for case-insensitive lookup.
    ///
    /// # Arguments
    ///
    /// * `login_name` - The login name to search for
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    /// Returns `Ok(None)` if the operator is not found.
    pub fn get_operator_by_login(
        &self,
        login_name: &str,
    ) -> Result<Option<OperatorData>, PersistenceError> {
        sqlite::get_operator_by_login(&self.conn, login_name)
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
    /// Returns `Ok(None)` if the operator is not found.
    pub fn get_operator_by_id(
        &self,
        operator_id: i64,
    ) -> Result<Option<OperatorData>, PersistenceError> {
        sqlite::get_operator_by_id(&self.conn, operator_id)
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
        sqlite::update_last_login(&self.conn, operator_id)
    }

    /// Disables an operator.
    ///
    /// This sets `is_disabled` to true and records the `disabled_at` timestamp.
    ///
    /// # Arguments
    ///
    /// * `operator_id` - The operator ID
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails.
    pub fn disable_operator(&mut self, operator_id: i64) -> Result<(), PersistenceError> {
        sqlite::disable_operator(&self.conn, operator_id)
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
        sqlite::enable_operator(&self.conn, operator_id)
    }

    /// Deletes an operator.
    ///
    /// This operation will fail if the operator is referenced by any audit events.
    ///
    /// # Arguments
    ///
    /// * `operator_id` - The operator ID
    ///
    /// # Errors
    ///
    /// Returns `PersistenceError::OperatorReferenced` if the operator is referenced
    /// by audit events. Returns other errors if the database delete fails.
    pub fn delete_operator(&mut self, operator_id: i64) -> Result<(), PersistenceError> {
        sqlite::delete_operator(&self.conn, operator_id)
    }

    /// Lists all operators.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn list_operators(&self) -> Result<Vec<OperatorData>, PersistenceError> {
        sqlite::list_operators(&self.conn)
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
    pub fn is_operator_referenced(&self, operator_id: i64) -> Result<bool, PersistenceError> {
        sqlite::is_operator_referenced(&self.conn, operator_id)
    }

    /// Counts the total number of operators.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn count_operators(&self) -> Result<i64, PersistenceError> {
        sqlite::count_operators(&self.conn)
    }

    /// Counts the number of active admin operators.
    ///
    /// An active admin operator is one where:
    /// - `role` is 'Admin'
    /// - `is_disabled` is false
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub fn count_active_admin_operators(&self) -> Result<i64, PersistenceError> {
        sqlite::count_active_admin_operators(&self.conn)
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
    pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, PersistenceError> {
        sqlite::verify_password(password, password_hash)
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
    /// Returns an error if the password cannot be hashed or the update fails.
    pub fn update_password(
        &mut self,
        operator_id: i64,
        new_password: &str,
    ) -> Result<(), PersistenceError> {
        sqlite::update_password(&self.conn, operator_id, new_password)
    }

    /// Deletes all sessions for a specific operator.
    ///
    /// This is used when an operator's password is changed to invalidate all active sessions.
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
        sqlite::delete_sessions_for_operator(&self.conn, operator_id)
    }

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
        sqlite::create_session(&self.conn, session_token, operator_id, expires_at)
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
    /// Returns `Ok(None)` if the session is not found.
    pub fn get_session_by_token(
        &self,
        session_token: &str,
    ) -> Result<Option<SessionData>, PersistenceError> {
        sqlite::get_session_by_token(&self.conn, session_token)
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
        sqlite::update_session_activity(&self.conn, session_id)
    }

    /// Deletes a session by token.
    ///
    /// This is used for logout operations.
    ///
    /// # Arguments
    ///
    /// * `session_token` - The session token to delete
    ///
    /// # Errors
    ///
    /// Returns an error if the database delete fails.
    pub fn delete_session(&mut self, session_token: &str) -> Result<(), PersistenceError> {
        sqlite::delete_session(&self.conn, session_token)
    }

    /// Deletes all expired sessions.
    ///
    /// This is a cleanup operation that should be run periodically.
    ///
    /// # Errors
    ///
    /// Returns an error if the database delete fails.
    pub fn delete_expired_sessions(&mut self) -> Result<usize, PersistenceError> {
        sqlite::delete_expired_sessions(&self.conn)
    }

    // ========================================================================
    // Phase 18: Bootstrap Workflow Completion Methods
    // ========================================================================

    /// Sets a bid year as active, ensuring only one bid year is active at a time.
    ///
    /// # Arguments
    ///
    /// * `year` - The year to mark as active
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be updated or the bid year does not exist.
    pub fn set_active_bid_year(&mut self, year: u16) -> Result<(), PersistenceError> {
        sqlite::set_active_bid_year(&self.conn, year)
    }

    /// Gets the currently active bid year, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_active_bid_year(&self) -> Result<Option<u16>, PersistenceError> {
        sqlite::get_active_bid_year(&self.conn)
    }

    /// Sets the expected area count for a bid year.
    ///
    /// # Arguments
    ///
    /// * `year` - The bid year
    /// * `expected_count` - The expected number of areas
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be updated or the bid year does not exist.
    pub fn set_expected_area_count(
        &mut self,
        year: u16,
        expected_count: u32,
    ) -> Result<(), PersistenceError> {
        sqlite::set_expected_area_count(&self.conn, year, expected_count)
    }

    /// Gets the expected area count for a bid year.
    ///
    /// # Arguments
    ///
    /// * `year` - The bid year
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_expected_area_count(&self, year: u16) -> Result<Option<u32>, PersistenceError> {
        sqlite::get_expected_area_count(&self.conn, year)
    }

    /// Sets the expected user count for an area.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    /// * `expected_count` - The expected number of users
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be updated or the area does not exist.
    pub fn set_expected_user_count(
        &mut self,
        bid_year: &BidYear,
        area: &Area,
        expected_count: u32,
    ) -> Result<(), PersistenceError> {
        sqlite::set_expected_user_count(&self.conn, bid_year, area, expected_count)
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
    /// Returns an error if the database cannot be queried.
    pub fn get_expected_user_count(
        &self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<Option<u32>, PersistenceError> {
        sqlite::get_expected_user_count(&self.conn, bid_year, area)
    }

    /// Gets the actual area count for a bid year.
    ///
    /// # Arguments
    ///
    /// * `year` - The bid year
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    pub fn get_actual_area_count(&self, year: u16) -> Result<usize, PersistenceError> {
        sqlite::get_actual_area_count(&self.conn, year)
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
        &self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<usize, PersistenceError> {
        sqlite::get_actual_user_count(&self.conn, bid_year, area)
    }

    /// Updates an existing user's information using `user_id` as the canonical identifier.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The user's canonical internal identifier
    /// * `initials` - The user's initials (mutable field)
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
    /// Returns an error if the database cannot be updated or the user does not exist.
    #[allow(clippy::too_many_arguments)]
    pub fn update_user(
        &mut self,
        user_id: i64,
        initials: &zab_bid_domain::Initials,
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
        let crew_i32: Option<i32> = crew.map(i32::from);
        let lottery_i32: Option<i32> = lottery_value.and_then(|v| i32::try_from(v).ok());

        let rows_affected: usize = self.conn.execute(
            "UPDATE users SET
                initials = ?1,
                name = ?2,
                area_id = ?3,
                user_type = ?4,
                crew = ?5,
                cumulative_natca_bu_date = ?6,
                natca_bu_date = ?7,
                eod_faa_date = ?8,
                service_computation_date = ?9,
                lottery_value = ?10
             WHERE user_id = ?11",
            rusqlite::params![
                initials.value(),
                name,
                area.id(),
                user_type,
                crew_i32,
                cumulative_natca_bu_date,
                natca_bu_date,
                eod_faa_date,
                service_computation_date,
                lottery_i32,
                user_id,
            ],
        )?;

        if rows_affected == 0 {
            return Err(PersistenceError::NotFound(format!(
                "User with user_id {user_id} not found"
            )));
        }

        Ok(())
    }
}
