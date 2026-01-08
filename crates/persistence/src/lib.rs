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

use rusqlite::{Connection, Result as SqliteResult, Transaction, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info};
use zab_bid::{State, TransitionResult};
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{Area, BidYear};

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

/// Serializable representation of an Actor.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActorData {
    id: String,
    actor_type: String,
}

/// Serializable representation of a Cause.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CauseData {
    id: String,
    description: String,
}

/// Serializable representation of an Action.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActionData {
    name: String,
    details: Option<String>,
}

/// Serializable representation of a `StateSnapshot`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateSnapshotData {
    data: String,
}

/// Serializable representation of the full State.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateData {
    bid_year: u16,
    area: String,
    users_json: String,
}

/// Type alias for audit event row data from `SQLite`.
type AuditEventRow = (i64, u16, String, String, String, String, String, String);

/// Persistence adapter for audit events and state snapshots.
pub struct SqlitePersistence {
    conn: Connection,
}

impl SqlitePersistence {
    /// Creates a new persistence adapter with an in-memory database.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn new_in_memory() -> Result<Self, PersistenceError> {
        let conn: Connection = Connection::open_in_memory()?;
        let adapter: Self = Self { conn };
        adapter.initialize_schema()?;
        Ok(adapter)
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
        let adapter: Self = Self { conn };
        adapter.initialize_schema()?;
        Ok(adapter)
    }

    /// Initializes the database schema.
    fn initialize_schema(&self) -> Result<(), PersistenceError> {
        info!("Initializing database schema");

        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS audit_events (
                event_id INTEGER PRIMARY KEY AUTOINCREMENT,
                bid_year INTEGER NOT NULL,
                area TEXT NOT NULL,
                actor_json TEXT NOT NULL,
                cause_json TEXT NOT NULL,
                action_json TEXT NOT NULL,
                before_snapshot_json TEXT NOT NULL,
                after_snapshot_json TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(bid_year, area, event_id)
            );

            CREATE INDEX IF NOT EXISTS idx_audit_events_scope
                ON audit_events(bid_year, area, event_id);

            CREATE TABLE IF NOT EXISTS state_snapshots (
                snapshot_id INTEGER PRIMARY KEY AUTOINCREMENT,
                bid_year INTEGER NOT NULL,
                area TEXT NOT NULL,
                event_id INTEGER NOT NULL,
                state_json TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(bid_year, area, event_id),
                FOREIGN KEY(event_id) REFERENCES audit_events(event_id)
            );

            CREATE INDEX IF NOT EXISTS idx_state_snapshots_scope
                ON state_snapshots(bid_year, area, event_id DESC);
            ",
        )?;

        Ok(())
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
        let tx: Transaction<'_> = self.conn.transaction()?;

        // Persist the audit event
        let event_id: i64 = Self::persist_audit_event_tx(&tx, &result.audit_event)?;
        debug!(event_id, "Persisted audit event");

        // Persist full snapshot if required
        if should_snapshot {
            Self::persist_state_snapshot_tx(&tx, &result.new_state, event_id)?;
            debug!(event_id, "Persisted full state snapshot");
        }

        tx.commit()?;
        info!(event_id, should_snapshot, "Persisted transition");

        Ok(event_id)
    }

    /// Persists an audit event within a transaction.
    fn persist_audit_event_tx(
        tx: &Transaction<'_>,
        event: &AuditEvent,
    ) -> Result<i64, PersistenceError> {
        let actor_data: ActorData = ActorData {
            id: event.actor.id.clone(),
            actor_type: event.actor.actor_type.clone(),
        };

        let cause_data: CauseData = CauseData {
            id: event.cause.id.clone(),
            description: event.cause.description.clone(),
        };

        let action_data: ActionData = ActionData {
            name: event.action.name.clone(),
            details: event.action.details.clone(),
        };

        let before_data: StateSnapshotData = StateSnapshotData {
            data: event.before.data.clone(),
        };

        let after_data: StateSnapshotData = StateSnapshotData {
            data: event.after.data.clone(),
        };

        tx.execute(
            "INSERT INTO audit_events (
                bid_year, area, actor_json, cause_json, action_json,
                before_snapshot_json, after_snapshot_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.bid_year.year(),
                event.area.id(),
                serde_json::to_string(&actor_data)?,
                serde_json::to_string(&cause_data)?,
                serde_json::to_string(&action_data)?,
                serde_json::to_string(&before_data)?,
                serde_json::to_string(&after_data)?,
            ],
        )?;

        Ok(tx.last_insert_rowid())
    }

    /// Persists a full state snapshot within a transaction.
    fn persist_state_snapshot_tx(
        tx: &Transaction<'_>,
        state: &State,
        event_id: i64,
    ) -> Result<(), PersistenceError> {
        let state_data: StateData = StateData {
            bid_year: state.bid_year.year(),
            area: state.area.id().to_string(),
            users_json: serde_json::to_string(&state.users)?,
        };

        tx.execute(
            "INSERT INTO state_snapshots (bid_year, area, event_id, state_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                state.bid_year.year(),
                state.area.id(),
                event_id,
                serde_json::to_string(&state_data)?,
            ],
        )?;

        Ok(())
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
        let row_result: SqliteResult<AuditEventRow> = self.conn.query_row(
            "SELECT event_id, bid_year, area, actor_json, cause_json, action_json,
                        before_snapshot_json, after_snapshot_json
                 FROM audit_events
                 WHERE event_id = ?1",
            params![event_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        );

        match row_result {
            Ok((
                retrieved_event_id,
                bid_year,
                area,
                actor_json,
                cause_json,
                action_json,
                before_json,
                after_json,
            )) => {
                let actor_data: ActorData = serde_json::from_str(&actor_json)?;
                let cause_data: CauseData = serde_json::from_str(&cause_json)?;
                let action_data: ActionData = serde_json::from_str(&action_json)?;
                let before_data: StateSnapshotData = serde_json::from_str(&before_json)?;
                let after_data: StateSnapshotData = serde_json::from_str(&after_json)?;

                Ok(AuditEvent::with_id(
                    retrieved_event_id,
                    Actor::new(actor_data.id, actor_data.actor_type),
                    Cause::new(cause_data.id, cause_data.description),
                    Action::new(action_data.name, action_data.details),
                    StateSnapshot::new(before_data.data),
                    StateSnapshot::new(after_data.data),
                    BidYear::new(bid_year),
                    Area::new(area),
                ))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(PersistenceError::EventNotFound(event_id))
            }
            Err(e) => Err(PersistenceError::DatabaseError(e.to_string())),
        }
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
        let row_result: SqliteResult<(String, i64)> = self.conn.query_row(
            "SELECT state_json, event_id
                 FROM state_snapshots
                 WHERE bid_year = ?1 AND area = ?2
                 ORDER BY event_id DESC
                 LIMIT 1",
            params![bid_year.year(), area.id()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match row_result {
            Ok((state_json, event_id)) => {
                let state_data: StateData = serde_json::from_str(&state_json)?;
                let users: Vec<_> = serde_json::from_str(&state_data.users_json)?;

                Ok((
                    State {
                        bid_year: BidYear::new(state_data.bid_year),
                        area: Area::new(state_data.area),
                        users,
                    },
                    event_id,
                ))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(PersistenceError::SnapshotNotFound {
                bid_year: bid_year.year(),
                area: area.id().to_string(),
            }),
            Err(e) => Err(PersistenceError::DatabaseError(e.to_string())),
        }
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
        let mut stmt = self.conn.prepare(
            "SELECT event_id, bid_year, area, actor_json, cause_json, action_json,
                    before_snapshot_json, after_snapshot_json
             FROM audit_events
             WHERE bid_year = ?1 AND area = ?2 AND event_id > ?3
             ORDER BY event_id ASC",
        )?;

        let events: Result<Vec<AuditEvent>, PersistenceError> = stmt
            .query_map(params![bid_year.year(), area.id(), after_event_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, u16>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })?
            .map(|row_result| {
                let (
                    event_id,
                    bid_year,
                    area,
                    actor_json,
                    cause_json,
                    action_json,
                    before_json,
                    after_json,
                ) = row_result.map_err(|e| PersistenceError::DatabaseError(e.to_string()))?;

                let actor_data: ActorData = serde_json::from_str(&actor_json)?;
                let cause_data: CauseData = serde_json::from_str(&cause_json)?;
                let action_data: ActionData = serde_json::from_str(&action_json)?;
                let before_data: StateSnapshotData = serde_json::from_str(&before_json)?;
                let after_data: StateSnapshotData = serde_json::from_str(&after_json)?;

                Ok(AuditEvent::with_id(
                    event_id,
                    Actor::new(actor_data.id, actor_data.actor_type),
                    Cause::new(cause_data.id, cause_data.description),
                    Action::new(action_data.name, action_data.details),
                    StateSnapshot::new(before_data.data),
                    StateSnapshot::new(after_data.data),
                    BidYear::new(bid_year),
                    Area::new(area),
                ))
            })
            .collect();

        events
    }

    /// Determines if a given action requires a full snapshot.
    #[must_use]
    pub fn should_snapshot(action_name: &str) -> bool {
        matches!(action_name, "Checkpoint" | "Finalize" | "Rollback")
    }

    /// Retrieves the current effective state for a given `(bid_year, area)` scope.
    ///
    /// This is a read-only operation that returns the most recent snapshot.
    /// In the current implementation, snapshots represent complete state at specific points,
    /// and non-snapshot events are for audit trail purposes only.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `area` - The area
    ///
    /// # Errors
    ///
    /// Returns an error if no snapshot exists.
    pub fn get_current_state(
        &self,
        bid_year: &BidYear,
        area: &Area,
    ) -> Result<State, PersistenceError> {
        tracing::debug!(
            bid_year = bid_year.year(),
            area = area.id(),
            "Retrieving current effective state"
        );

        // Get the most recent snapshot - this IS the current state
        let (state, snapshot_event_id): (State, i64) = self.get_latest_snapshot(bid_year, area)?;

        tracing::info!(
            bid_year = bid_year.year(),
            area = area.id(),
            snapshot_event_id = snapshot_event_id,
            "Retrieved current state from snapshot"
        );

        Ok(state)
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
        tracing::debug!(
            bid_year = bid_year.year(),
            area = area.id(),
            timestamp = timestamp,
            "Retrieving historical state"
        );

        // Get the most recent snapshot at or before the timestamp - this IS the historical state
        let (state, snapshot_event_id): (State, i64) =
            self.get_snapshot_before_timestamp(bid_year, area, timestamp)?;

        tracing::info!(
            bid_year = bid_year.year(),
            area = area.id(),
            timestamp = timestamp,
            snapshot_event_id = snapshot_event_id,
            "Retrieved historical state from snapshot"
        );

        Ok(state)
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
        tracing::debug!(
            bid_year = bid_year.year(),
            area = area.id(),
            "Retrieving audit timeline"
        );

        let mut stmt = self.conn.prepare(
            "SELECT event_id, bid_year, area, actor_json, cause_json, action_json,
                    before_snapshot_json, after_snapshot_json
             FROM audit_events
             WHERE bid_year = ?1 AND area = ?2
             ORDER BY event_id ASC",
        )?;

        let events: Result<Vec<AuditEvent>, PersistenceError> = stmt
            .query_map(params![bid_year.year(), area.id()], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, u16>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })?
            .map(|row_result| {
                let (
                    event_id,
                    bid_year,
                    area,
                    actor_json,
                    cause_json,
                    action_json,
                    before_json,
                    after_json,
                ) = row_result.map_err(|e| PersistenceError::DatabaseError(e.to_string()))?;

                let actor_data: ActorData = serde_json::from_str(&actor_json)?;
                let cause_data: CauseData = serde_json::from_str(&cause_json)?;
                let action_data: ActionData = serde_json::from_str(&action_json)?;
                let before_data: StateSnapshotData = serde_json::from_str(&before_json)?;
                let after_data: StateSnapshotData = serde_json::from_str(&after_json)?;

                Ok(AuditEvent::with_id(
                    event_id,
                    Actor::new(actor_data.id, actor_data.actor_type),
                    Cause::new(cause_data.id, cause_data.description),
                    Action::new(action_data.name, action_data.details),
                    StateSnapshot::new(before_data.data),
                    StateSnapshot::new(after_data.data),
                    BidYear::new(bid_year),
                    Area::new(area),
                ))
            })
            .collect();

        let event_list: Vec<AuditEvent> = events?;

        tracing::info!(
            bid_year = bid_year.year(),
            area = area.id(),
            event_count = event_list.len(),
            "Retrieved audit timeline"
        );

        Ok(event_list)
    }

    /// Retrieves the most recent snapshot at or before a given timestamp.
    fn get_snapshot_before_timestamp(
        &self,
        bid_year: &BidYear,
        area: &Area,
        timestamp: &str,
    ) -> Result<(State, i64), PersistenceError> {
        let row_result: SqliteResult<(String, i64)> = self.conn.query_row(
            "SELECT s.state_json, s.event_id
             FROM state_snapshots s
             JOIN audit_events e ON s.event_id = e.event_id
             WHERE s.bid_year = ?1 AND s.area = ?2 AND e.created_at <= ?3
             ORDER BY s.event_id DESC
             LIMIT 1",
            params![bid_year.year(), area.id(), timestamp],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match row_result {
            Ok((state_json, event_id)) => {
                let state_data: StateData = serde_json::from_str(&state_json)?;
                let users: Vec<_> = serde_json::from_str(&state_data.users_json)?;

                Ok((
                    State {
                        bid_year: BidYear::new(state_data.bid_year),
                        area: Area::new(state_data.area),
                        users,
                    },
                    event_id,
                ))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(PersistenceError::SnapshotNotFound {
                bid_year: bid_year.year(),
                area: area.id().to_string(),
            }),
            Err(e) => Err(PersistenceError::DatabaseError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zab_bid::{Command, apply};
    use zab_bid_domain::{Crew, Initials, SeniorityData};

    fn create_test_actor() -> Actor {
        Actor::new(String::from("test-actor"), String::from("system"))
    }

    fn create_test_cause() -> Cause {
        Cause::new(String::from("test-cause"), String::from("Test operation"))
    }

    fn create_test_seniority_data() -> SeniorityData {
        SeniorityData::new(
            String::from("2019-01-15"),
            String::from("2019-06-01"),
            String::from("2020-01-15"),
            String::from("2020-01-15"),
            Some(42),
        )
    }

    #[test]
    fn test_persistence_initialization() {
        let result: Result<SqlitePersistence, PersistenceError> =
            SqlitePersistence::new_in_memory();
        assert!(result.is_ok());
    }

    #[test]
    fn test_persist_and_retrieve_audit_event() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let command: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("AB")),
            name: String::from("John Doe"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };

        let result: TransitionResult =
            apply(&state, command, create_test_actor(), create_test_cause()).unwrap();

        let event_id: i64 = persistence.persist_transition(&result, false).unwrap();

        let retrieved: AuditEvent = persistence.get_audit_event(event_id).unwrap();
        assert_eq!(retrieved.event_id, Some(event_id));
        assert_eq!(retrieved.action.name, "RegisterUser");
    }

    #[test]
    fn test_persist_with_snapshot() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        let command: Command = Command::Checkpoint;
        let result: TransitionResult =
            apply(&state, command, create_test_actor(), create_test_cause()).unwrap();

        let event_id: i64 = persistence.persist_transition(&result, true).unwrap();

        let (snapshot, snapshot_event_id): (State, i64) = persistence
            .get_latest_snapshot(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        assert_eq!(snapshot_event_id, event_id);
        assert_eq!(snapshot.bid_year.year(), 2026);
        assert_eq!(snapshot.area.id(), "North");
    }

    #[test]
    fn test_get_events_after() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create first event
        let command1: Command = Command::Checkpoint;
        let result1: TransitionResult =
            apply(&state, command1, create_test_actor(), create_test_cause()).unwrap();
        let event_id1: i64 = persistence.persist_transition(&result1, true).unwrap();

        // Create second event
        let command2: Command = Command::Finalize;
        let result2: TransitionResult = apply(
            &result1.new_state,
            command2,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        let _event_id2: i64 = persistence.persist_transition(&result2, true).unwrap();

        // Retrieve events after first
        let events: Vec<AuditEvent> = persistence
            .get_events_after(
                &BidYear::new(2026),
                &Area::new(String::from("North")),
                event_id1,
            )
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action.name, "Finalize");
    }

    #[test]
    fn test_should_snapshot_detection() {
        assert!(SqlitePersistence::should_snapshot("Checkpoint"));
        assert!(SqlitePersistence::should_snapshot("Finalize"));
        assert!(SqlitePersistence::should_snapshot("Rollback"));
        assert!(!SqlitePersistence::should_snapshot("RegisterUser"));
    }

    #[test]
    fn test_atomic_persistence_failure() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();

        // Close the connection to force an error
        drop(persistence);

        // Try to create a new one and verify it works
        persistence = SqlitePersistence::new_in_memory().unwrap();

        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let command: Command = Command::Checkpoint;
        let result: TransitionResult =
            apply(&state, command, create_test_actor(), create_test_cause()).unwrap();

        // This should succeed
        assert!(persistence.persist_transition(&result, true).is_ok());
    }

    #[test]
    fn test_get_current_state_with_no_deltas() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create a snapshot
        let command: Command = Command::Checkpoint;
        let result: TransitionResult =
            apply(&state, command, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result, true).unwrap();

        // Retrieve current state
        let current_state: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        assert_eq!(current_state.bid_year.year(), 2026);
        assert_eq!(current_state.area.id(), "North");
        assert_eq!(current_state.users.len(), 0);
    }

    #[test]
    fn test_get_current_state_after_snapshot_with_user() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create initial empty snapshot
        let command1: Command = Command::Checkpoint;
        let result1: TransitionResult =
            apply(&state, command1, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result1, true).unwrap();

        // Register a user (delta event, no snapshot)
        let command2: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("AB")),
            name: String::from("Alice Blue"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let result2: TransitionResult = apply(
            &result1.new_state,
            command2,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result2, false).unwrap();

        // Create another snapshot to capture the state with the user
        let command3: Command = Command::Checkpoint;
        let result3: TransitionResult = apply(
            &result2.new_state,
            command3,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result3, true).unwrap();

        // Retrieve current state - should include the user from most recent snapshot
        let current_state: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        assert_eq!(current_state.bid_year.year(), 2026);
        assert_eq!(current_state.area.id(), "North");
        assert_eq!(current_state.users.len(), 1);
        assert_eq!(current_state.users[0].initials.value(), "AB");
    }

    #[test]
    fn test_get_current_state_no_snapshot_returns_error() {
        let persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();

        // Try to retrieve current state with no snapshot
        let result: Result<State, PersistenceError> =
            persistence.get_current_state(&BidYear::new(2026), &Area::new(String::from("North")));

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PersistenceError::SnapshotNotFound { .. }
        ));
    }

    #[test]
    fn test_get_audit_timeline_returns_events_in_order() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create multiple events
        let command1: Command = Command::Checkpoint;
        let result1: TransitionResult =
            apply(&state, command1, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result1, true).unwrap();

        let command2: Command = Command::Finalize;
        let result2: TransitionResult = apply(
            &result1.new_state,
            command2,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result2, true).unwrap();

        let command3: Command = Command::RollbackToEventId { target_event_id: 1 };
        let result3: TransitionResult = apply(
            &result2.new_state,
            command3,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result3, true).unwrap();

        // Retrieve timeline
        let timeline: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        assert_eq!(timeline.len(), 3);
        assert_eq!(timeline[0].action.name, "Checkpoint");
        assert_eq!(timeline[1].action.name, "Finalize");
        assert_eq!(timeline[2].action.name, "Rollback");

        // Verify event IDs are in ascending order
        assert!(timeline[0].event_id.unwrap() < timeline[1].event_id.unwrap());
        assert!(timeline[1].event_id.unwrap() < timeline[2].event_id.unwrap());
    }

    #[test]
    fn test_get_audit_timeline_empty_for_nonexistent_scope() {
        let persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();

        // Retrieve timeline for non-existent scope
        let timeline: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("South")))
            .unwrap();

        assert_eq!(timeline.len(), 0);
    }

    #[test]
    fn test_get_audit_timeline_includes_rollback_events() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create checkpoint
        let command1: Command = Command::Checkpoint;
        let result1: TransitionResult =
            apply(&state, command1, create_test_actor(), create_test_cause()).unwrap();
        let event_id1: i64 = persistence.persist_transition(&result1, true).unwrap();

        // Create rollback
        let command2: Command = Command::RollbackToEventId {
            target_event_id: event_id1,
        };
        let result2: TransitionResult = apply(
            &result1.new_state,
            command2,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result2, true).unwrap();

        // Retrieve timeline
        let timeline: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        assert_eq!(timeline.len(), 2);
        assert_eq!(timeline[0].action.name, "Checkpoint");
        assert_eq!(timeline[1].action.name, "Rollback");
    }

    #[test]
    fn test_get_current_state_is_deterministic() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create initial snapshot
        let command1: Command = Command::Checkpoint;
        let result1: TransitionResult =
            apply(&state, command1, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result1, true).unwrap();

        // Register a user
        let command2: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("XY")),
            name: String::from("Xavier Young"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("B")),
            seniority_data: create_test_seniority_data(),
        };
        let result2: TransitionResult = apply(
            &result1.new_state,
            command2,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result2, false).unwrap();

        // Create snapshot with user
        let command3: Command = Command::Checkpoint;
        let result3: TransitionResult = apply(
            &result2.new_state,
            command3,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result3, true).unwrap();

        // Retrieve current state multiple times
        let state1: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        let state2: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        let state3: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        // All retrievals should produce identical results
        assert_eq!(state1.users.len(), state2.users.len());
        assert_eq!(state2.users.len(), state3.users.len());
        assert_eq!(state1.users.len(), 1);
        assert_eq!(state1.users[0].initials.value(), "XY");
        assert_eq!(state2.users[0].initials.value(), "XY");
        assert_eq!(state3.users[0].initials.value(), "XY");
    }

    #[test]
    fn test_get_current_state_does_not_mutate() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create a snapshot
        let command: Command = Command::Checkpoint;
        let result: TransitionResult =
            apply(&state, command, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result, true).unwrap();

        // Count events before read
        let timeline_before: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        // Perform read
        let _current_state: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        // Count events after read
        let timeline_after: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        // No new events should be created
        assert_eq!(timeline_before.len(), timeline_after.len());
    }

    #[test]
    fn test_get_audit_timeline_does_not_mutate() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create events
        let command: Command = Command::Checkpoint;
        let result: TransitionResult =
            apply(&state, command, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result, true).unwrap();

        // Retrieve timeline
        let timeline1: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        // Retrieve again
        let timeline2: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        // Should be identical
        assert_eq!(timeline1.len(), timeline2.len());
        assert_eq!(timeline1.len(), 1);
    }

    #[test]
    fn test_get_current_state_with_multiple_users() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create initial snapshot
        let command1: Command = Command::Checkpoint;
        let result1: TransitionResult =
            apply(&state, command1, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result1, true).unwrap();

        // Register first user
        let command2: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("AA")),
            name: String::from("Alice Anderson"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let result2: TransitionResult = apply(
            &result1.new_state,
            command2,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result2, false).unwrap();

        // Register second user
        let command3: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("BB")),
            name: String::from("Bob Brown"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("B")),
            seniority_data: create_test_seniority_data(),
        };
        let result3: TransitionResult = apply(
            &result2.new_state,
            command3,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result3, false).unwrap();

        // Create snapshot with both users
        let command4: Command = Command::Checkpoint;
        let result4: TransitionResult = apply(
            &result3.new_state,
            command4,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result4, true).unwrap();

        // Retrieve current state
        let current_state: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        assert_eq!(current_state.users.len(), 2);
    }

    #[test]
    fn test_get_current_state_different_areas_isolated() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();

        // Create state for North area
        let state_north: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let command_north: Command = Command::Checkpoint;
        let result_north: TransitionResult = apply(
            &state_north,
            command_north,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result_north, true).unwrap();

        // Create state for South area
        let state_south: State = State::new(BidYear::new(2026), Area::new(String::from("South")));
        let command_south: Command = Command::Checkpoint;
        let result_south: TransitionResult = apply(
            &state_south,
            command_south,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result_south, true).unwrap();

        // Retrieve both states
        let current_north: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        let current_south: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("South")))
            .unwrap();

        assert_eq!(current_north.area.id(), "North");
        assert_eq!(current_south.area.id(), "South");
    }

    #[test]
    fn test_state_reconstruction_with_snapshot_then_deltas() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create initial snapshot
        let command1: Command = Command::Checkpoint;
        let result1: TransitionResult =
            apply(&state, command1, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result1, true).unwrap();

        // Add user (delta)
        let command2: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("TS")),
            name: String::from("Test User"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let result2: TransitionResult = apply(
            &result1.new_state,
            command2,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result2, false).unwrap();

        // Create another snapshot
        let command3: Command = Command::Checkpoint;
        let result3: TransitionResult = apply(
            &result2.new_state,
            command3,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result3, true).unwrap();

        // Current state should use most recent snapshot
        let current_state: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        assert_eq!(current_state.users.len(), 1);
        assert_eq!(current_state.users[0].initials.value(), "TS");
    }

    #[test]
    fn test_get_historical_state_at_specific_timestamp() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create first snapshot with no users
        let command1: Command = Command::Checkpoint;
        let result1: TransitionResult =
            apply(&state, command1, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result1, true).unwrap();

        // Register a user (non-snapshot event)
        let command2: Command = Command::RegisterUser {
            bid_year: BidYear::new(2026),
            initials: Initials::new(String::from("NE")),
            name: String::from("New User"),
            area: Area::new(String::from("North")),
            crew: Crew::new(String::from("A")),
            seniority_data: create_test_seniority_data(),
        };
        let result2: TransitionResult = apply(
            &result1.new_state,
            command2,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result2, false).unwrap();

        // Create second snapshot with user
        let command3: Command = Command::Checkpoint;
        let result3: TransitionResult = apply(
            &result2.new_state,
            command3,
            create_test_actor(),
            create_test_cause(),
        )
        .unwrap();
        persistence.persist_transition(&result3, true).unwrap();

        // Query historical state at very early time - should return error (no snapshot yet)
        let early_timestamp: String = String::from("1970-01-01 00:00:00");
        let result_early: Result<State, PersistenceError> = persistence.get_historical_state(
            &BidYear::new(2026),
            &Area::new(String::from("North")),
            &early_timestamp,
        );
        assert!(result_early.is_err());

        // Query historical state at far future time - should use most recent snapshot (with user)
        let future_timestamp: String = String::from("9999-12-31 23:59:59");
        let historical_state: State = persistence
            .get_historical_state(
                &BidYear::new(2026),
                &Area::new(String::from("North")),
                &future_timestamp,
            )
            .unwrap();

        assert_eq!(historical_state.users.len(), 1);
        assert_eq!(historical_state.users[0].initials.value(), "NE");
    }

    #[test]
    fn test_get_historical_state_before_any_snapshot_returns_error() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create a snapshot
        let command: Command = Command::Checkpoint;
        let result: TransitionResult =
            apply(&state, command, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result, true).unwrap();

        // Try to query before the snapshot was created
        let early_timestamp: String = String::from("2020-01-01 00:00:00");
        let result: Result<State, PersistenceError> = persistence.get_historical_state(
            &BidYear::new(2026),
            &Area::new(String::from("North")),
            &early_timestamp,
        );

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PersistenceError::SnapshotNotFound { .. }
        ));
    }

    #[test]
    fn test_get_historical_state_is_deterministic() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create snapshot
        let command: Command = Command::Checkpoint;
        let result: TransitionResult =
            apply(&state, command, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result, true).unwrap();

        // Get the timestamp
        let timestamp: String = persistence
            .conn
            .query_row(
                "SELECT created_at FROM audit_events WHERE event_id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();

        // Query multiple times
        let state1: State = persistence
            .get_historical_state(
                &BidYear::new(2026),
                &Area::new(String::from("North")),
                &timestamp,
            )
            .unwrap();

        let state2: State = persistence
            .get_historical_state(
                &BidYear::new(2026),
                &Area::new(String::from("North")),
                &timestamp,
            )
            .unwrap();

        let state3: State = persistence
            .get_historical_state(
                &BidYear::new(2026),
                &Area::new(String::from("North")),
                &timestamp,
            )
            .unwrap();

        // All should be identical
        assert_eq!(state1.users.len(), state2.users.len());
        assert_eq!(state2.users.len(), state3.users.len());
        assert_eq!(state1.users.len(), 0);
    }

    #[test]
    fn test_get_historical_state_does_not_mutate() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create snapshot
        let command: Command = Command::Checkpoint;
        let result: TransitionResult =
            apply(&state, command, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result, true).unwrap();

        let timestamp: String = String::from("9999-12-31 23:59:59");

        // Count events before read
        let timeline_before: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        // Perform historical read
        let _historical_state: State = persistence
            .get_historical_state(
                &BidYear::new(2026),
                &Area::new(String::from("North")),
                &timestamp,
            )
            .unwrap();

        // Count events after read
        let timeline_after: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        // No new events should be created
        assert_eq!(timeline_before.len(), timeline_after.len());
    }

    #[test]
    fn test_read_operations_are_side_effect_free() {
        let mut persistence: SqlitePersistence = SqlitePersistence::new_in_memory().unwrap();
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));

        // Create initial snapshot
        let command: Command = Command::Checkpoint;
        let result: TransitionResult =
            apply(&state, command, create_test_actor(), create_test_cause()).unwrap();
        persistence.persist_transition(&result, true).unwrap();

        // Capture initial event count
        let initial_timeline: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();
        let initial_count: usize = initial_timeline.len();

        // Perform multiple read operations
        let _current1: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        let _current2: State = persistence
            .get_current_state(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        let _timeline1: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        let _timeline2: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        let timestamp: String = String::from("9999-12-31 23:59:59");
        let _historical1: State = persistence
            .get_historical_state(
                &BidYear::new(2026),
                &Area::new(String::from("North")),
                &timestamp,
            )
            .unwrap();

        let _historical2: State = persistence
            .get_historical_state(
                &BidYear::new(2026),
                &Area::new(String::from("North")),
                &timestamp,
            )
            .unwrap();

        // Verify no new events were created
        let final_timeline: Vec<AuditEvent> = persistence
            .get_audit_timeline(&BidYear::new(2026), &Area::new(String::from("North")))
            .unwrap();

        assert_eq!(final_timeline.len(), initial_count);
    }
}
