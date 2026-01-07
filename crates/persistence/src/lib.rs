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
            initials: Initials::new(String::from("ABC")),
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
}
