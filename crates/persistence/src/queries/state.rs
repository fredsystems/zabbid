// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! State snapshot and reconstruction queries.
//!
//! This module contains queries for retrieving and reconstructing state
//! from snapshots and canonical tables.
//!
//! All queries are generated in backend-specific monomorphic versions
//! (`_sqlite` and `_mysql` suffixes) using the `backend_fn!` macro.

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use zab_bid::State;
use zab_bid_domain::{Area, BidYear, Crew, Initials, SeniorityData, User, UserType};

use crate::data_models::StateData;
use crate::diesel_schema::{audit_events, state_snapshots, users};
use crate::error::PersistenceError;

/// Diesel Queryable struct for state snapshot rows.
#[derive(Queryable, Selectable)]
#[diesel(table_name = state_snapshots)]
#[allow(dead_code)]
struct StateSnapshotRow {
    state_json: String,
    event_id: i64,
}

/// Diesel Queryable struct for user rows.
#[derive(Queryable, Selectable)]
#[diesel(table_name = users)]
struct UserRow {
    user_id: i64,
    #[allow(dead_code)]
    bid_year_id: i64,
    #[allow(dead_code)]
    area_id: i64,
    initials: String,
    name: String,
    user_type: String,
    crew: Option<i32>,
    cumulative_natca_bu_date: String,
    natca_bu_date: String,
    eod_faa_date: String,
    service_computation_date: String,
    lottery_value: Option<i32>,
    excluded_from_bidding: i32,
    excluded_from_leave_calculation: i32,
    no_bid_reviewed: i32,
}

backend_fn! {
/// Retrieves the most recent state snapshot for a `(BidYear, Area)` scope.
///
/// Phase 23A: Now uses `bid_year_id` and `area_id` for queries.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
///
/// # Errors
///
/// Returns an error if no snapshot exists or cannot be deserialized.
///
/// # Generated Functions
///
/// - `get_latest_snapshot_sqlite(&mut SqliteConnection, i64, i64)`
/// - `get_latest_snapshot_mysql(&mut MysqlConnection, i64, i64)`
pub fn get_latest_snapshot(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
) -> Result<(State, i64), PersistenceError> {
    let result = state_snapshots::table
        .filter(state_snapshots::bid_year_id.eq(bid_year_id))
        .filter(state_snapshots::area_id.eq(area_id))
        .order(state_snapshots::event_id.desc())
        .select((state_snapshots::state_json, state_snapshots::event_id))
        .first::<(String, i64)>(conn);

    let (state_json, event_id) = match result {
        Ok(r) => r,
        Err(diesel::result::Error::NotFound) => {
            return Err(PersistenceError::SnapshotNotFound {
                bid_year: 0,
                area: String::from("unknown"),
            });
        }
        Err(e) => return Err(PersistenceError::from(e)),
    };

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
}

backend_fn! {
/// Retrieves the most recent snapshot at or before a given timestamp.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `timestamp` - The target timestamp
///
/// # Errors
///
/// Returns an error if no snapshot exists before the timestamp.
///
/// # Generated Functions
///
/// - `get_snapshot_before_timestamp_sqlite(&mut SqliteConnection, i64, i64, &str)`
/// - `get_snapshot_before_timestamp_mysql(&mut MysqlConnection, i64, i64, &str)`
pub fn get_snapshot_before_timestamp(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
    timestamp: &str,
) -> Result<(State, i64), PersistenceError> {
    let result = state_snapshots::table
        .inner_join(audit_events::table.on(state_snapshots::event_id.eq(audit_events::event_id)))
        .filter(state_snapshots::bid_year_id.eq(bid_year_id))
        .filter(state_snapshots::area_id.eq(area_id))
        .filter(audit_events::created_at.le(timestamp))
        .order(state_snapshots::event_id.desc())
        .select((state_snapshots::state_json, state_snapshots::event_id))
        .first::<(String, i64)>(conn);

    let (state_json, event_id) = match result {
        Ok(r) => r,
        Err(diesel::result::Error::NotFound) => {
            return Err(PersistenceError::SnapshotNotFound {
                bid_year: 0,
                area: String::from("unknown"),
            });
        }
        Err(e) => return Err(PersistenceError::from(e)),
    };

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
}

backend_fn! {
/// Retrieves the current effective state for a given `(BidYear, Area)` scope.
///
/// This queries the canonical `users` table to reconstruct the current state.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `bid_year` - The bid year (for constructing the result)
/// * `area` - The area (for constructing the result)
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
///
/// # Generated Functions
///
/// - `get_current_state_sqlite(&mut SqliteConnection, i64, i64, &BidYear, &Area)`
/// - `get_current_state_mysql(&mut MysqlConnection, i64, i64, &BidYear, &Area)`
pub fn get_current_state(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
    bid_year: &BidYear,
    area: &Area,
) -> Result<State, PersistenceError> {
    tracing::debug!(
        bid_year = bid_year.year(),
        area = area.id(),
        "Retrieving current effective state from canonical tables"
    );

    let rows = users::table
        .filter(users::bid_year_id.eq(bid_year_id))
        .filter(users::area_id.eq(area_id))
        .order(users::initials.asc())
        .select(UserRow::as_select())
        .load::<UserRow>(conn)?;

    let mut users_vec: Vec<User> = Vec::new();
    for row in rows {
        let initials: Initials = Initials::new(&row.initials);
        let user_type: UserType = UserType::parse(&row.user_type)
            .map_err(|e| PersistenceError::ReconstructionError(e.to_string()))?;
        let crew: Option<Crew> = row
            .crew
            .and_then(|n| u8::try_from(n).ok().and_then(|num| Crew::new(num).ok()));
        let seniority_data: SeniorityData = SeniorityData::new(
            row.cumulative_natca_bu_date,
            row.natca_bu_date,
            row.eod_faa_date,
            row.service_computation_date,
            row.lottery_value.and_then(|v| u32::try_from(v).ok()),
        );

        let user: User = User::with_id(
            row.user_id,
            bid_year.clone(),
            initials,
            row.name,
            area.clone(),
            user_type,
            crew,
            seniority_data,
            row.excluded_from_bidding != 0,
            row.excluded_from_leave_calculation != 0,
            row.no_bid_reviewed != 0,
        );
        users_vec.push(user);
    }

    let state: State = State {
        bid_year: bid_year.clone(),
        area: area.clone(),
        users: users_vec,
    };

    tracing::info!(
        bid_year = bid_year.year(),
        area = area.id(),
        user_count = state.users.len(),
        "Retrieved current state from canonical tables"
    );

    Ok(state)
}
}

/// Retrieves the effective state for a given `(BidYear, Area)` scope at a specific timestamp.
///
/// `SQLite` version.
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
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `timestamp` - The target timestamp (ISO 8601 format)
///
/// # Errors
///
/// Returns an error if no snapshot exists before the timestamp.
pub fn get_historical_state_sqlite(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    area_id: i64,
    timestamp: &str,
) -> Result<State, PersistenceError> {
    tracing::debug!(
        bid_year_id,
        area_id,
        timestamp,
        "Retrieving historical state"
    );

    // Get the most recent snapshot at or before the timestamp - this IS the historical state
    let (state, snapshot_event_id): (State, i64) =
        get_snapshot_before_timestamp_sqlite(conn, bid_year_id, area_id, timestamp)?;

    tracing::info!(
        bid_year_id,
        area_id,
        timestamp,
        snapshot_event_id,
        "Retrieved historical state from snapshot"
    );

    Ok(state)
}

/// Retrieves the effective state for a given `(BidYear, Area)` scope at a specific timestamp.
///
/// `MySQL` version.
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
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `timestamp` - The target timestamp (ISO 8601 format)
///
/// # Errors
///
/// Returns an error if no snapshot exists before the timestamp.
pub fn get_historical_state_mysql(
    conn: &mut MysqlConnection,
    bid_year_id: i64,
    area_id: i64,
    timestamp: &str,
) -> Result<State, PersistenceError> {
    tracing::debug!(
        bid_year_id,
        area_id,
        timestamp,
        "Retrieving historical state"
    );

    // Get the most recent snapshot at or before the timestamp - this IS the historical state
    let (state, snapshot_event_id): (State, i64) =
        get_snapshot_before_timestamp_mysql(conn, bid_year_id, area_id, timestamp)?;

    tracing::info!(
        bid_year_id,
        area_id,
        timestamp,
        snapshot_event_id,
        "Retrieved historical state from snapshot"
    );

    Ok(state)
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
