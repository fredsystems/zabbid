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
use zab_bid_domain::{Area, BidYear, User};

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
        Ok(Self { conn })
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

    /// Lists all bid years that have been created.
    ///
    /// This queries the canonical `bid_years` table directly.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be queried.
    ///
    /// # Panics
    ///
    /// Panics if a bid year value from the database cannot be converted to `u16`.
    /// This should never happen in practice as the schema enforces valid ranges.
    pub fn list_bid_years(&self) -> Result<Vec<BidYear>, PersistenceError> {
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
}
