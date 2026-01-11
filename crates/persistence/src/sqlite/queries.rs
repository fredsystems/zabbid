// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use num_traits::ToPrimitive;
use rusqlite::{Connection, Result as SqliteResult, params};
use zab_bid::State;
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{Area, BidYear, Crew, Initials, SeniorityData, User, UserType};

use crate::data_models::{
    ActionData, ActorData, AuditEventRow, CauseData, StateData, StateSnapshotData,
};
use crate::error::PersistenceError;

/// Retrieves an audit event by ID.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `event_id` - The event ID to retrieve
///
/// # Errors
///
/// Returns an error if the event is not found or cannot be deserialized.
pub fn get_audit_event(conn: &Connection, event_id: i64) -> Result<AuditEvent, PersistenceError> {
    let row_result: SqliteResult<AuditEventRow> = conn.query_row(
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
                Area::new(&area),
            ))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(PersistenceError::EventNotFound(event_id)),
        Err(e) => Err(PersistenceError::DatabaseError(e.to_string())),
    }
}

/// Retrieves the most recent state snapshot for a `(bid_year, area)` scope.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year` - The bid year
/// * `area` - The area
///
/// # Errors
///
/// Returns an error if no snapshot exists or cannot be deserialized.
pub fn get_latest_snapshot(
    conn: &Connection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<(State, i64), PersistenceError> {
    let row_result: SqliteResult<(String, i64)> = conn.query_row(
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
                    area: Area::new(&state_data.area),
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
/// * `conn` - The database connection
/// * `bid_year` - The bid year
/// * `area` - The area
/// * `after_event_id` - Only return events after this ID (exclusive)
///
/// # Errors
///
/// Returns an error if events cannot be retrieved or deserialized.
pub fn get_events_after(
    conn: &Connection,
    bid_year: &BidYear,
    area: &Area,
    after_event_id: i64,
) -> Result<Vec<AuditEvent>, PersistenceError> {
    let mut stmt = conn.prepare(
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
                Area::new(&area),
            ))
        })
        .collect();

    events
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
    matches!(action_name, "Checkpoint" | "Finalize" | "Rollback")
}

/// Retrieves the current effective state for a given `(bid_year, area)` scope.
///
/// This queries the canonical `users` table to reconstruct the current state.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year` - The bid year
/// * `area` - The area
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_current_state(
    conn: &Connection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<State, PersistenceError> {
    tracing::debug!(
        bid_year = bid_year.year(),
        area = area.id(),
        "Retrieving current effective state from canonical tables"
    );

    let mut stmt = conn.prepare(
        "SELECT initials, name, user_type, crew,
                cumulative_natca_bu_date, natca_bu_date, eod_faa_date,
                service_computation_date, lottery_value
         FROM users
         WHERE bid_year = ?1 AND area_id = ?2
         ORDER BY initials ASC",
    )?;

    let rows = stmt.query_map(params![bid_year.year(), area.id()], |row| {
        Ok((
            row.get::<_, String>(0)?,      // initials
            row.get::<_, String>(1)?,      // name
            row.get::<_, String>(2)?,      // user_type
            row.get::<_, Option<i32>>(3)?, // crew
            row.get::<_, String>(4)?,      // cumulative_natca_bu_date
            row.get::<_, String>(5)?,      // natca_bu_date
            row.get::<_, String>(6)?,      // eod_faa_date
            row.get::<_, String>(7)?,      // service_computation_date
            row.get::<_, Option<i32>>(8)?, // lottery_value
        ))
    })?;

    let mut users: Vec<User> = Vec::new();
    for row_result in rows {
        let (
            initials_str,
            name,
            user_type_str,
            crew_num,
            cumulative_natca_bu_date,
            natca_bu_date,
            eod_faa_date,
            service_computation_date,
            lottery_value,
        ) = row_result?;

        let initials: Initials = Initials::new(&initials_str);
        let user_type: UserType = UserType::parse(&user_type_str)
            .map_err(|e| PersistenceError::ReconstructionError(e.to_string()))?;
        let crew: Option<Crew> =
            crew_num.and_then(|n| u8::try_from(n).ok().and_then(|num| Crew::new(num).ok()));
        let seniority_data: SeniorityData = SeniorityData::new(
            cumulative_natca_bu_date,
            natca_bu_date,
            eod_faa_date,
            service_computation_date,
            lottery_value.and_then(|v| u32::try_from(v).ok()),
        );

        let user: User = User::new(
            bid_year.clone(),
            initials,
            name,
            area.clone(),
            user_type,
            crew,
            seniority_data,
        );
        users.push(user);
    }

    let state: State = State {
        bid_year: bid_year.clone(),
        area: area.clone(),
        users,
    };

    tracing::info!(
        bid_year = bid_year.year(),
        area = area.id(),
        user_count = state.users.len(),
        "Retrieved current state from canonical tables"
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
/// * `conn` - The database connection
/// * `bid_year` - The bid year
/// * `area` - The area
/// * `timestamp` - The target timestamp (ISO 8601 format)
///
/// # Errors
///
/// Returns an error if no snapshot exists before the timestamp.
pub fn get_historical_state(
    conn: &Connection,
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
        get_snapshot_before_timestamp(conn, bid_year, area, timestamp)?;

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
/// * `conn` - The database connection
/// * `bid_year` - The bid year
/// * `area` - The area
///
/// # Errors
///
/// Returns an error if events cannot be retrieved or deserialized.
pub fn get_audit_timeline(
    conn: &Connection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<Vec<AuditEvent>, PersistenceError> {
    tracing::debug!(
        bid_year = bid_year.year(),
        area = area.id(),
        "Retrieving audit timeline"
    );

    let mut stmt = conn.prepare(
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
                Area::new(&area),
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
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year` - The bid year
/// * `area` - The area
/// * `timestamp` - The target timestamp
///
/// # Errors
///
/// Returns an error if no snapshot exists before the timestamp.
fn get_snapshot_before_timestamp(
    conn: &Connection,
    bid_year: &BidYear,
    area: &Area,
    timestamp: &str,
) -> Result<(State, i64), PersistenceError> {
    let row_result: SqliteResult<(String, i64)> = conn.query_row(
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
                    area: Area::new(&state_data.area),
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

/// Counts users per area for a given bid year.
///
/// Returns a vector of tuples containing (`area_id`, `user_count`).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year` - The bid year to count users for
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if count conversion fails.
pub fn count_users_by_area(
    conn: &Connection,
    bid_year: &BidYear,
) -> Result<Vec<(String, usize)>, PersistenceError> {
    let mut stmt = conn.prepare(
        "SELECT area_id, COUNT(*) as user_count
         FROM users
         WHERE bid_year = ?1
         GROUP BY area_id
         ORDER BY area_id ASC",
    )?;

    let rows = stmt.query_map(params![bid_year.year()], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;

    let mut result: Vec<(String, usize)> = Vec::new();
    for row in rows {
        let (area_id, count) = row.map_err(|e| PersistenceError::DatabaseError(e.to_string()))?;
        let count_usize: usize = count.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((area_id, count_usize));
    }

    Ok(result)
}

/// Counts areas per bid year.
///
/// Returns a vector of tuples containing (`bid_year`, `area_count`).
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if conversions fail.
pub fn count_areas_by_bid_year(conn: &Connection) -> Result<Vec<(u16, usize)>, PersistenceError> {
    let mut stmt = conn.prepare(
        "SELECT bid_year, COUNT(*) as area_count
         FROM areas
         GROUP BY bid_year
         ORDER BY bid_year ASC",
    )?;

    let rows = stmt.query_map([], |row| Ok((row.get::<_, i32>(0)?, row.get::<_, i64>(1)?)))?;

    let mut result: Vec<(u16, usize)> = Vec::new();
    for row in rows {
        let (bid_year, count) = row.map_err(|e| PersistenceError::DatabaseError(e.to_string()))?;
        let bid_year_u16: u16 = bid_year.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = count.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((bid_year_u16, count_usize));
    }

    Ok(result)
}

/// Counts total users per bid year across all areas.
///
/// Returns a vector of tuples containing (`bid_year`, `total_user_count`).
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if conversions fail.
pub fn count_users_by_bid_year(conn: &Connection) -> Result<Vec<(u16, usize)>, PersistenceError> {
    let mut stmt = conn.prepare(
        "SELECT bid_year, COUNT(*) as user_count
         FROM users
         GROUP BY bid_year
         ORDER BY bid_year ASC",
    )?;

    let rows = stmt.query_map([], |row| Ok((row.get::<_, i32>(0)?, row.get::<_, i64>(1)?)))?;

    let mut result: Vec<(u16, usize)> = Vec::new();
    for row in rows {
        let (bid_year, count) = row.map_err(|e| PersistenceError::DatabaseError(e.to_string()))?;
        let bid_year_u16: u16 = bid_year.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = count.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((bid_year_u16, count_usize));
    }

    Ok(result)
}

/// Counts users per (`bid_year`, `area_id`) combination.
///
/// Returns a vector of tuples containing (`bid_year`, `area_id`, `user_count`).
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if conversions fail.
pub fn count_users_by_bid_year_and_area(
    conn: &Connection,
) -> Result<Vec<(u16, String, usize)>, PersistenceError> {
    let mut stmt = conn.prepare(
        "SELECT bid_year, area_id, COUNT(*) as user_count
         FROM users
         GROUP BY bid_year, area_id
         ORDER BY bid_year ASC, area_id ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;

    let mut result: Vec<(u16, String, usize)> = Vec::new();
    for row in rows {
        let (bid_year, area_id, count) =
            row.map_err(|e| PersistenceError::DatabaseError(e.to_string()))?;
        let bid_year_u16: u16 = bid_year.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = count.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((bid_year_u16, area_id, count_usize));
    }

    Ok(result)
}
