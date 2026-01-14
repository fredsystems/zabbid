// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use diesel::prelude::*;
use diesel::sql_types::BigInt;
use num_traits::ToPrimitive;
use zab_bid::State;
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{Area, BidYear, Crew, Initials, SeniorityData, User, UserType};

use crate::data_models::{ActionData, ActorData, CauseData, StateData, StateSnapshotData};
use crate::diesel_schema::{areas, audit_events, bid_years, state_snapshots, users};
use crate::error::PersistenceError;

#[derive(diesel::QueryableByName)]
#[allow(dead_code)]
pub struct CountRow {
    #[diesel(sql_type = BigInt)]
    pub count: i64,
}

// Diesel Queryable structs for table projections
#[derive(Queryable, Selectable)]
#[diesel(table_name = audit_events)]
struct AuditEventFullRow {
    event_id: i64,
    bid_year_id: Option<i64>,
    area_id: Option<i64>,
    year: i32,
    area_code: String,
    actor_operator_id: i64,
    actor_login_name: String,
    actor_display_name: String,
    actor_json: String,
    cause_json: String,
    action_json: String,
    before_snapshot_json: String,
    after_snapshot_json: String,
    #[allow(dead_code)]
    created_at: Option<String>,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = state_snapshots)]
#[allow(dead_code)] // Used in get_snapshot_before_timestamp but Rust doesn't see it
struct StateSnapshotRow {
    state_json: String,
    event_id: i64,
}

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
}

/// Looks up the `bid_year_id` from the year value.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `year` - The year value
///
/// # Errors
///
/// Returns an error if the bid year does not exist.
pub fn lookup_bid_year_id(conn: &mut SqliteConnection, year: u16) -> Result<i64, PersistenceError> {
    let year_i32: i32 = year
        .to_i32()
        .ok_or_else(|| PersistenceError::Other("Year out of range".to_string()))?;

    let result = bid_years::table
        .select(bid_years::bid_year_id)
        .filter(bid_years::year.eq(year_i32))
        .first::<i64>(conn);

    match result {
        Ok(id) => Ok(id),
        Err(diesel::result::Error::NotFound) => Err(PersistenceError::ReconstructionError(
            format!("Bid year {year} does not exist"),
        )),
        Err(e) => Err(PersistenceError::from(e)),
    }
}

/// Looks up the `bid_year_id` from the year value within a transaction.
///
/// # Arguments
///
/// * `conn` - The database connection (Diesel uses same connection type for transactions)
/// * `year` - The year value
///
/// # Errors
///
/// Returns an error if the bid year does not exist.
pub fn lookup_bid_year_id_tx(
    conn: &mut SqliteConnection,
    year: u16,
) -> Result<i64, PersistenceError> {
    // Diesel uses the same connection type for transactions, so this just delegates
    lookup_bid_year_id(conn, year)
}

/// Looks up the `area_id` from the `bid_year_id` and `area_code`.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The bid year ID
/// * `area_code` - The area code
///
/// # Errors
///
/// Returns an error if the area does not exist.
pub fn lookup_area_id(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    area_code: &str,
) -> Result<i64, PersistenceError> {
    let result = areas::table
        .select(areas::area_id)
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::area_code.eq(area_code))
        .first::<i64>(conn);

    match result {
        Ok(id) => Ok(id),
        Err(diesel::result::Error::NotFound) => Err(PersistenceError::ReconstructionError(
            format!("Area {area_code} in bid year ID {bid_year_id} does not exist"),
        )),
        Err(e) => Err(PersistenceError::from(e)),
    }
}

/// Looks up the `area_id` from the `bid_year_id` and `area_code` within a transaction.
///
/// # Arguments
///
/// * `conn` - The database connection (Diesel uses same connection type for transactions)
/// * `bid_year_id` - The bid year ID
/// * `area_code` - The area code
///
/// # Errors
///
/// Returns an error if the area does not exist.
pub fn lookup_area_id_tx(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    area_code: &str,
) -> Result<i64, PersistenceError> {
    // Special case: bid_year_id -1 (sentinel for year 0) with any area code
    // Return a sentinel ID that won't conflict with real IDs
    // TODO: This sentinel logic should be removed post-Phase 23A
    if bid_year_id == -1 {
        return Ok(-1);
    }

    // Diesel uses the same connection type for transactions
    lookup_area_id(conn, bid_year_id, area_code)
}

/// Retrieves an audit event by ID.
///
/// Phase 23A: Now retrieves and uses canonical IDs to construct domain objects.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `event_id` - The event ID to retrieve
///
/// # Errors
///
/// Returns an error if the event is not found or cannot be deserialized.
pub fn get_audit_event(
    conn: &mut SqliteConnection,
    event_id: i64,
) -> Result<AuditEvent, PersistenceError> {
    let result = audit_events::table
        .filter(audit_events::event_id.eq(event_id))
        .select(AuditEventFullRow::as_select())
        .first::<AuditEventFullRow>(conn);

    let row: AuditEventFullRow = match result {
        Ok(r) => r,
        Err(diesel::result::Error::NotFound) => {
            return Err(PersistenceError::EventNotFound(event_id));
        }
        Err(e) => return Err(PersistenceError::from(e)),
    };

    // Convert year from i32 to u16
    let year: u16 = row
        .year
        .to_u16()
        .ok_or_else(|| PersistenceError::ReconstructionError("Year out of range".to_string()))?;

    let actor_data: ActorData = serde_json::from_str(&row.actor_json)?;
    let cause_data: CauseData = serde_json::from_str(&row.cause_json)?;
    let action_data: ActionData = serde_json::from_str(&row.action_json)?;
    let before_data: StateSnapshotData = serde_json::from_str(&row.before_snapshot_json)?;
    let after_data: StateSnapshotData = serde_json::from_str(&row.after_snapshot_json)?;

    // Reconstruct Actor with operator information if available (Phase 14)
    let actor: Actor = if row.actor_operator_id != 0 {
        Actor::with_operator(
            actor_data.id,
            actor_data.actor_type,
            row.actor_operator_id,
            row.actor_login_name,
            row.actor_display_name,
        )
    } else {
        Actor::new(actor_data.id, actor_data.actor_type)
    };

    // Reconstruct domain objects with IDs (Phase 23A)
    // For CreateBidYear and operator events, bid_year_id might be NULL
    let bid_year_id: i64 = row.bid_year_id.unwrap_or(0);
    let bid_year: BidYear = BidYear::with_id(bid_year_id, year);
    // For CreateBidYear events, area_id might be NULL (use a sentinel area)
    let area: Area = row.area_id.map_or_else(
        || Area::new(&row.area_code),
        |id| Area::with_id(id, &row.area_code, None),
    );

    Ok(AuditEvent::with_id(
        row.event_id,
        actor,
        Cause::new(cause_data.id, cause_data.description),
        Action::new(action_data.name, action_data.details),
        StateSnapshot::new(before_data.data),
        StateSnapshot::new(after_data.data),
        bid_year,
        area,
    ))
}

/// Retrieves the most recent state snapshot for a `(bid_year, area)` scope.
///
/// Phase 23A: Now uses `bid_year_id` and `area_id` for queries.
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
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<(State, i64), PersistenceError> {
    // Look up the IDs
    let bid_year_id: i64 = lookup_bid_year_id(conn, bid_year.year())?;
    let area_id: i64 = lookup_area_id(conn, bid_year_id, area.id())?;

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
                bid_year: bid_year.year(),
                area: area.id().to_string(),
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

/// Retrieves all audit events for a `(bid_year, area)` scope after a given event ID.
///
/// Phase 23A: Now uses `bid_year_id` and `area_id` for queries.
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
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
    area: &Area,
    after_event_id: i64,
) -> Result<Vec<AuditEvent>, PersistenceError> {
    // Look up the IDs
    let bid_year_id: i64 = lookup_bid_year_id(conn, bid_year.year())?;
    let area_id: i64 = lookup_area_id(conn, bid_year_id, area.id())?;

    let rows = audit_events::table
        .filter(audit_events::bid_year_id.eq(bid_year_id))
        .filter(audit_events::area_id.eq(area_id))
        .filter(audit_events::event_id.gt(after_event_id))
        .order(audit_events::event_id.asc())
        .select(AuditEventFullRow::as_select())
        .load::<AuditEventFullRow>(conn)?;

    let events: Result<Vec<AuditEvent>, PersistenceError> = rows
        .into_iter()
        .map(|row| {
            let year: u16 = row.year.to_u16().ok_or_else(|| {
                PersistenceError::ReconstructionError("Year out of range".to_string())
            })?;

            let actor_data: ActorData = serde_json::from_str(&row.actor_json)?;
            let cause_data: CauseData = serde_json::from_str(&row.cause_json)?;
            let action_data: ActionData = serde_json::from_str(&row.action_json)?;
            let before_data: StateSnapshotData = serde_json::from_str(&row.before_snapshot_json)?;
            let after_data: StateSnapshotData = serde_json::from_str(&row.after_snapshot_json)?;

            // Reconstruct Actor with operator information if available (Phase 14)
            let actor: Actor = if row.actor_operator_id != 0 {
                Actor::with_operator(
                    actor_data.id,
                    actor_data.actor_type,
                    row.actor_operator_id,
                    row.actor_login_name,
                    row.actor_display_name,
                )
            } else {
                Actor::new(actor_data.id, actor_data.actor_type)
            };

            // Reconstruct domain objects with IDs (Phase 23A)
            // For events after filtering by bid_year_id/area_id, bid_year_id should be present
            let bid_year_id_val: i64 = row.bid_year_id.unwrap_or(0);
            let bid_year: BidYear = BidYear::with_id(bid_year_id_val, year);
            // area_id should always be present in get_events_after (it filters by area_id)
            // but handle None as a safety measure
            let area: Area = row.area_id.map_or_else(
                || Area::new(&row.area_code),
                |id| Area::with_id(id, &row.area_code, None),
            );

            Ok(AuditEvent::with_id(
                row.event_id,
                actor,
                Cause::new(cause_data.id, cause_data.description),
                Action::new(action_data.name, action_data.details),
                StateSnapshot::new(before_data.data),
                StateSnapshot::new(after_data.data),
                bid_year,
                area,
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
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<State, PersistenceError> {
    tracing::debug!(
        bid_year = bid_year.year(),
        area = area.id(),
        "Retrieving current effective state from canonical tables"
    );

    // Look up the IDs (Phase 23A)
    let bid_year_id: i64 = lookup_bid_year_id(conn, bid_year.year())?;
    let area_id: i64 = lookup_area_id(conn, bid_year_id, area.id())?;

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
    conn: &mut SqliteConnection,
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
///
#[allow(clippy::too_many_lines)]
pub fn get_audit_timeline(
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<Vec<AuditEvent>, PersistenceError> {
    tracing::debug!(
        bid_year = bid_year.year(),
        area = area.id(),
        "Retrieving audit timeline"
    );

    // Phase 23A: Look up the canonical IDs
    // If the bid year or area doesn't exist, return an empty timeline
    let bid_year_id = match lookup_bid_year_id(conn, bid_year.year()) {
        Ok(id) => id,
        Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let area_id = match lookup_area_id(conn, bid_year_id, area.id()) {
        Ok(id) => id,
        Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    let rows = audit_events::table
        .filter(audit_events::bid_year_id.eq(bid_year_id))
        .filter(audit_events::area_id.eq(area_id))
        .order(audit_events::event_id.asc())
        .select((
            audit_events::event_id,
            audit_events::year,
            audit_events::area_code,
            audit_events::actor_operator_id,
            audit_events::actor_login_name,
            audit_events::actor_display_name,
            audit_events::actor_json,
            audit_events::cause_json,
            audit_events::action_json,
            audit_events::before_snapshot_json,
            audit_events::after_snapshot_json,
        ))
        .load::<(
            i64,
            i32,
            String,
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        )>(conn)?;

    let events: Result<Vec<AuditEvent>, PersistenceError> = rows
        .into_iter()
        .map(
            |(
                event_id,
                year_i32,
                area_code,
                actor_operator_id,
                actor_login_name,
                actor_display_name,
                actor_json,
                cause_json,
                action_json,
                before_snapshot_json,
                after_snapshot_json,
            )| {
                let year = year_i32.to_u16().ok_or_else(|| {
                    PersistenceError::ReconstructionError("Year out of range".to_string())
                })?;

                let actor: Actor = if actor_operator_id != 0 {
                    Actor::with_operator(
                        serde_json::from_str::<ActorData>(&actor_json)?.id,
                        serde_json::from_str::<ActorData>(&actor_json)?.actor_type,
                        actor_operator_id,
                        actor_login_name,
                        actor_display_name,
                    )
                } else {
                    let actor_data: ActorData = serde_json::from_str(&actor_json)?;
                    Actor::new(actor_data.id, actor_data.actor_type)
                };

                let cause_data: CauseData = serde_json::from_str(&cause_json)?;
                let action_data: ActionData = serde_json::from_str(&action_json)?;

                Ok(AuditEvent::with_id(
                    event_id,
                    actor,
                    Cause::new(cause_data.id, cause_data.description),
                    Action::new(action_data.name, action_data.details),
                    StateSnapshot::new(
                        serde_json::from_str::<StateSnapshotData>(&before_snapshot_json)?.data,
                    ),
                    StateSnapshot::new(
                        serde_json::from_str::<StateSnapshotData>(&after_snapshot_json)?.data,
                    ),
                    BidYear::with_id(bid_year_id, year),
                    Area::with_id(area_id, &area_code, None),
                ))
            },
        )
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

/// Retrieves all global audit events (events with no bid year or area scope).
///
/// Global events include operator-management actions and other system-level operations
/// that are not scoped to a specific bid year or area.
///
/// Events are returned in strict chronological order (ascending by `event_id`).
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if events cannot be retrieved or deserialized.
pub fn get_global_audit_events(
    conn: &mut SqliteConnection,
) -> Result<Vec<AuditEvent>, PersistenceError> {
    tracing::debug!("Retrieving global audit timeline");

    let rows = audit_events::table
        .filter(audit_events::bid_year_id.is_null())
        .filter(audit_events::area_id.is_null())
        .order(audit_events::event_id.asc())
        .select((
            audit_events::event_id,
            audit_events::actor_operator_id,
            audit_events::actor_login_name,
            audit_events::actor_display_name,
            audit_events::actor_json,
            audit_events::cause_json,
            audit_events::action_json,
            audit_events::before_snapshot_json,
            audit_events::after_snapshot_json,
        ))
        .load::<(
            i64,
            i64,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        )>(conn)?;

    let events: Result<Vec<AuditEvent>, PersistenceError> = rows
        .into_iter()
        .map(
            |(
                event_id,
                actor_operator_id,
                actor_login_name,
                actor_display_name,
                actor_json,
                cause_json,
                action_json,
                before_snapshot_json,
                after_snapshot_json,
            )| {
                let actor_data: ActorData = serde_json::from_str(&actor_json)?;
                let cause_data: CauseData = serde_json::from_str(&cause_json)?;
                let action_data: ActionData = serde_json::from_str(&action_json)?;
                let before_data: StateSnapshotData = serde_json::from_str(&before_snapshot_json)?;
                let after_data: StateSnapshotData = serde_json::from_str(&after_snapshot_json)?;

                // Reconstruct Actor with operator information if available
                let actor: Actor = if actor_operator_id != 0 {
                    Actor::with_operator(
                        actor_data.id,
                        actor_data.actor_type,
                        actor_operator_id,
                        actor_login_name,
                        actor_display_name,
                    )
                } else {
                    Actor::new(actor_data.id, actor_data.actor_type)
                };

                // Global events have no bid year or area
                // Create event with event_id but no scope
                Ok(AuditEvent {
                    event_id: Some(event_id),
                    actor,
                    cause: Cause::new(cause_data.id, cause_data.description),
                    action: Action::new(action_data.name, action_data.details),
                    before: StateSnapshot::new(before_data.data),
                    after: StateSnapshot::new(after_data.data),
                    bid_year: None,
                    area: None,
                })
            },
        )
        .collect();

    let event_list: Vec<AuditEvent> = events?;

    tracing::info!(
        event_count = event_list.len(),
        "Retrieved global audit timeline"
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
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
    area: &Area,
    timestamp: &str,
) -> Result<(State, i64), PersistenceError> {
    // Phase 23A: Look up the canonical IDs
    let bid_year_id = lookup_bid_year_id(conn, bid_year.year())?;
    let area_id = lookup_area_id(conn, bid_year_id, area.id())?;

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
                bid_year: bid_year.year(),
                area: area.id().to_string(),
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

/// Counts users per area for a given bid year.
///
/// Returns a vector of tuples containing (`area_id`, `user_count`).
///
/// Phase 23A: Now uses `bid_year_id` for queries.
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
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
) -> Result<Vec<(String, usize)>, PersistenceError> {
    // Get the bid_year_id
    let bid_year_id: i64 = bid_year.bid_year_id().ok_or_else(|| {
        PersistenceError::ReconstructionError(
            "BidYear must have a bid_year_id to count users".to_string(),
        )
    })?;

    let rows = users::table
        .inner_join(areas::table.on(users::area_id.eq(areas::area_id)))
        .filter(users::bid_year_id.eq(bid_year_id))
        .group_by(areas::area_code)
        .order(areas::area_code.asc())
        .select((areas::area_code, diesel::dsl::count(users::user_id)))
        .load::<(String, i64)>(conn)?;

    let mut result: Vec<(String, usize)> = Vec::new();
    for (area_code, count_i64) in rows {
        let count_usize: usize = count_i64.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((area_code, count_usize));
    }

    Ok(result)
}

/// Counts areas per bid year.
///
/// Returns a vector of tuples containing (`bid_year`, `area_count`).
///
/// Phase 23A: Updated to use `bid_year_id` with JOIN.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if conversions fail.
pub fn count_areas_by_bid_year(
    conn: &mut SqliteConnection,
) -> Result<Vec<(u16, usize)>, PersistenceError> {
    let rows = areas::table
        .inner_join(bid_years::table.on(areas::bid_year_id.eq(bid_years::bid_year_id)))
        .group_by(bid_years::year)
        .order(bid_years::year.asc())
        .select((bid_years::year, diesel::dsl::count(areas::area_id)))
        .load::<(i32, i64)>(conn)?;

    let mut result: Vec<(u16, usize)> = Vec::new();
    for (year_i32, count_i64) in rows {
        let bid_year_u16: u16 = year_i32.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = count_i64.to_usize().ok_or_else(|| {
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
/// Phase 23A: Updated to use `bid_year_id`.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if conversions fail.
pub fn count_users_by_bid_year(
    conn: &mut SqliteConnection,
) -> Result<Vec<(u16, usize)>, PersistenceError> {
    let rows = users::table
        .inner_join(bid_years::table.on(users::bid_year_id.eq(bid_years::bid_year_id)))
        .group_by(bid_years::year)
        .order(bid_years::year.asc())
        .select((bid_years::year, diesel::dsl::count(users::user_id)))
        .load::<(i32, i64)>(conn)?;

    let mut result: Vec<(u16, usize)> = Vec::new();
    for (year_i32, count_i64) in rows {
        let bid_year_u16: u16 = year_i32.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = count_i64.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((bid_year_u16, count_usize));
    }

    Ok(result)
}

/// Counts users per (`bid_year`, `area_id`) combination.
///
/// Returns a vector of tuples containing (`bid_year`, `area_code`, `user_count`).
///
/// Phase 23A: Updated to use join tables and return `area_code`.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if conversions fail.
pub fn count_users_by_bid_year_and_area(
    conn: &mut SqliteConnection,
) -> Result<Vec<(u16, String, usize)>, PersistenceError> {
    let rows = users::table
        .inner_join(bid_years::table.on(users::bid_year_id.eq(bid_years::bid_year_id)))
        .inner_join(areas::table.on(users::area_id.eq(areas::area_id)))
        .group_by((bid_years::year, areas::area_code))
        .order((bid_years::year.asc(), areas::area_code.asc()))
        .select((
            bid_years::year,
            areas::area_code,
            diesel::dsl::count(users::user_id),
        ))
        .load::<(i32, String, i64)>(conn)?;

    let mut result: Vec<(u16, String, usize)> = Vec::new();
    for (year_i32, area_code, count_i64) in rows {
        let bid_year_u16: u16 = year_i32.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = count_i64.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((bid_year_u16, area_code, count_usize));
    }

    Ok(result)
}
