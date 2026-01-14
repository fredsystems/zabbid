// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use diesel::sql_types::{BigInt, Integer, Nullable, Text};
use diesel::{RunQueryDsl, SqliteConnection, sql_query};
use num_traits::ToPrimitive;
use zab_bid::State;
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{Area, BidYear, Crew, Initials, SeniorityData, User, UserType};

use crate::data_models::{ActionData, ActorData, CauseData, StateData, StateSnapshotData};
use crate::error::PersistenceError;

// Helper structs for Diesel query results
#[derive(diesel::QueryableByName)]
pub struct BidYearIdRow {
    #[diesel(sql_type = BigInt)]
    pub bid_year_id: i64,
}

#[derive(diesel::QueryableByName)]
pub struct AreaIdRow {
    #[diesel(sql_type = BigInt)]
    pub area_id: i64,
}

#[derive(diesel::QueryableByName)]
pub struct UserIdRow {
    #[diesel(sql_type = BigInt)]
    pub user_id: i64,
}

#[derive(diesel::QueryableByName)]
#[allow(dead_code)]
pub struct CountRow {
    #[diesel(sql_type = BigInt)]
    pub count: i64,
}

#[derive(diesel::QueryableByName)]
struct AuditEventFullRow {
    #[diesel(sql_type = BigInt)]
    event_id: i64,
    #[diesel(sql_type = BigInt)]
    bid_year_id: i64,
    #[diesel(sql_type = Nullable<BigInt>)]
    area_id: Option<i64>,
    #[diesel(sql_type = Integer)]
    year: i32,
    #[diesel(sql_type = Text)]
    area_code: String,
    #[diesel(sql_type = BigInt)]
    actor_operator_id: i64,
    #[diesel(sql_type = Text)]
    actor_login_name: String,
    #[diesel(sql_type = Text)]
    actor_display_name: String,
    #[diesel(sql_type = Text)]
    actor_json: String,
    #[diesel(sql_type = Text)]
    cause_json: String,
    #[diesel(sql_type = Text)]
    action_json: String,
    #[diesel(sql_type = Text)]
    before_snapshot_json: String,
    #[diesel(sql_type = Text)]
    after_snapshot_json: String,
}

#[derive(diesel::QueryableByName)]
struct StateSnapshotRow {
    #[diesel(sql_type = Text)]
    state_json: String,
    #[diesel(sql_type = BigInt)]
    event_id: i64,
}

#[derive(diesel::QueryableByName)]
struct UserRow {
    #[diesel(sql_type = BigInt)]
    user_id: i64,
    #[diesel(sql_type = Text)]
    initials: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    user_type: String,
    #[diesel(sql_type = Nullable<Integer>)]
    crew: Option<i32>,
    #[diesel(sql_type = Text)]
    cumulative_natca_bu_date: String,
    #[diesel(sql_type = Text)]
    natca_bu_date: String,
    #[diesel(sql_type = Text)]
    eod_faa_date: String,
    #[diesel(sql_type = Text)]
    service_computation_date: String,
    #[diesel(sql_type = Nullable<Integer>)]
    lottery_value: Option<i32>,
}

#[derive(diesel::QueryableByName)]
struct AreaCountRow {
    #[diesel(sql_type = Text)]
    area_code: String,
    #[diesel(sql_type = BigInt)]
    user_count: i64,
}

#[derive(diesel::QueryableByName)]
struct YearCountRow {
    #[diesel(sql_type = Integer)]
    year: i32,
    #[diesel(sql_type = BigInt)]
    count: i64,
}

#[derive(diesel::QueryableByName)]
struct YearAreaCountRow {
    #[diesel(sql_type = Integer)]
    year: i32,
    #[diesel(sql_type = Text)]
    area_code: String,
    #[diesel(sql_type = BigInt)]
    user_count: i64,
}

// Row for audit timeline queries (no bid_year_id in SELECT)
#[derive(diesel::QueryableByName)]
struct AuditEventTimelineRow {
    #[diesel(sql_type = BigInt)]
    event_id: i64,
    #[diesel(sql_type = Integer)]
    year: i32,
    #[diesel(sql_type = Text)]
    area_code: String,
    #[diesel(sql_type = BigInt)]
    actor_operator_id: i64,
    #[diesel(sql_type = Text)]
    actor_login_name: String,
    #[diesel(sql_type = Text)]
    actor_display_name: String,
    #[diesel(sql_type = Text)]
    actor_json: String,
    #[diesel(sql_type = Text)]
    cause_json: String,
    #[diesel(sql_type = Text)]
    action_json: String,
    #[diesel(sql_type = Text)]
    before_snapshot_json: String,
    #[diesel(sql_type = Text)]
    after_snapshot_json: String,
}

// Row for global audit events (no bid year/area fields)
#[derive(diesel::QueryableByName)]
struct GlobalAuditEventRow {
    #[diesel(sql_type = BigInt)]
    event_id: i64,
    #[diesel(sql_type = BigInt)]
    actor_operator_id: i64,
    #[diesel(sql_type = Text)]
    actor_login_name: String,
    #[diesel(sql_type = Text)]
    actor_display_name: String,
    #[diesel(sql_type = Text)]
    actor_json: String,
    #[diesel(sql_type = Text)]
    cause_json: String,
    #[diesel(sql_type = Text)]
    action_json: String,
    #[diesel(sql_type = Text)]
    before_snapshot_json: String,
    #[diesel(sql_type = Text)]
    after_snapshot_json: String,
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

    let result: Result<BidYearIdRow, diesel::result::Error> =
        sql_query("SELECT bid_year_id FROM bid_years WHERE year = ?1")
            .bind::<Integer, _>(year_i32)
            .get_result::<BidYearIdRow>(conn);

    match result {
        Ok(row) => Ok(row.bid_year_id),
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
    let result: Result<AreaIdRow, diesel::result::Error> =
        sql_query("SELECT area_id FROM areas WHERE bid_year_id = ?1 AND area_code = ?2")
            .bind::<BigInt, _>(bid_year_id)
            .bind::<Text, _>(area_code)
            .get_result::<AreaIdRow>(conn);

    match result {
        Ok(row) => Ok(row.area_id),
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
    let result: Result<AuditEventFullRow, diesel::result::Error> = sql_query(
        "SELECT event_id, bid_year_id, area_id, year, area_code,
                actor_operator_id, actor_login_name, actor_display_name,
                actor_json, cause_json, action_json,
                before_snapshot_json, after_snapshot_json
         FROM audit_events
         WHERE event_id = ?1",
    )
    .bind::<BigInt, _>(event_id)
    .get_result::<AuditEventFullRow>(conn);

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
    let bid_year: BidYear = BidYear::with_id(row.bid_year_id, year);
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

    let result: Result<StateSnapshotRow, diesel::result::Error> = sql_query(
        "SELECT state_json, event_id
         FROM state_snapshots
         WHERE bid_year_id = ?1 AND area_id = ?2
         ORDER BY event_id DESC
         LIMIT 1",
    )
    .bind::<BigInt, _>(bid_year_id)
    .bind::<BigInt, _>(area_id)
    .get_result::<StateSnapshotRow>(conn);

    let row: StateSnapshotRow = match result {
        Ok(r) => r,
        Err(diesel::result::Error::NotFound) => {
            return Err(PersistenceError::SnapshotNotFound {
                bid_year: bid_year.year(),
                area: area.id().to_string(),
            });
        }
        Err(e) => return Err(PersistenceError::from(e)),
    };

    let state_data: StateData = serde_json::from_str(&row.state_json)?;
    let users: Vec<_> = serde_json::from_str(&state_data.users_json)?;

    Ok((
        State {
            bid_year: BidYear::new(state_data.bid_year),
            area: Area::new(&state_data.area),
            users,
        },
        row.event_id,
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

    let rows: Vec<AuditEventFullRow> = sql_query(
        "SELECT event_id, bid_year_id, area_id, year, area_code,
                actor_operator_id, actor_login_name, actor_display_name,
                actor_json, cause_json, action_json,
                before_snapshot_json, after_snapshot_json
         FROM audit_events
         WHERE bid_year_id = ?1 AND area_id = ?2 AND event_id > ?3
         ORDER BY event_id ASC",
    )
    .bind::<BigInt, _>(bid_year_id)
    .bind::<BigInt, _>(area_id)
    .bind::<BigInt, _>(after_event_id)
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
            let bid_year: BidYear = BidYear::with_id(row.bid_year_id, year);
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

    let rows: Vec<UserRow> = sql_query(
        "SELECT user_id, initials, name, user_type, crew,
                cumulative_natca_bu_date, natca_bu_date, eod_faa_date,
                service_computation_date, lottery_value
         FROM users
         WHERE bid_year_id = ?1 AND area_id = ?2
         ORDER BY initials ASC",
    )
    .bind::<BigInt, _>(bid_year_id)
    .bind::<BigInt, _>(area_id)
    .load::<UserRow>(conn)?;

    let mut users: Vec<User> = Vec::new();
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

    let rows: Vec<AuditEventTimelineRow> = sql_query(
        "SELECT event_id, year, area_code, actor_operator_id, actor_login_name,
                actor_display_name, actor_json, cause_json, action_json,
                before_snapshot_json, after_snapshot_json
         FROM audit_events
         WHERE bid_year_id = ?1 AND area_id = ?2
         ORDER BY event_id ASC",
    )
    .bind::<BigInt, _>(bid_year_id)
    .bind::<BigInt, _>(area_id)
    .load::<AuditEventTimelineRow>(conn)?;

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

            // Phase 23A: Reconstruct BidYear and Area with IDs
            Ok(AuditEvent::with_id(
                row.event_id,
                actor,
                Cause::new(cause_data.id, cause_data.description),
                Action::new(action_data.name, action_data.details),
                StateSnapshot::new(before_data.data),
                StateSnapshot::new(after_data.data),
                BidYear::with_id(bid_year_id, year),
                Area::with_id(area_id, &row.area_code, None),
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

    let rows: Vec<GlobalAuditEventRow> = sql_query(
        "SELECT event_id, actor_operator_id, actor_login_name,
                actor_display_name, actor_json, cause_json, action_json,
                before_snapshot_json, after_snapshot_json
         FROM audit_events
         WHERE bid_year_id IS NULL AND area_id IS NULL
         ORDER BY event_id ASC",
    )
    .load::<GlobalAuditEventRow>(conn)?;

    let events: Result<Vec<AuditEvent>, PersistenceError> = rows
        .into_iter()
        .map(|row| {
            let actor_data: ActorData = serde_json::from_str(&row.actor_json)?;
            let cause_data: CauseData = serde_json::from_str(&row.cause_json)?;
            let action_data: ActionData = serde_json::from_str(&row.action_json)?;
            let before_data: StateSnapshotData = serde_json::from_str(&row.before_snapshot_json)?;
            let after_data: StateSnapshotData = serde_json::from_str(&row.after_snapshot_json)?;

            // Reconstruct Actor with operator information if available
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

            // Global events have no bid year or area
            // Create event with event_id but no scope
            Ok(AuditEvent {
                event_id: Some(row.event_id),
                actor,
                cause: Cause::new(cause_data.id, cause_data.description),
                action: Action::new(action_data.name, action_data.details),
                before: StateSnapshot::new(before_data.data),
                after: StateSnapshot::new(after_data.data),
                bid_year: None,
                area: None,
            })
        })
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

    let result: Result<StateSnapshotRow, diesel::result::Error> = sql_query(
        "SELECT s.state_json, s.event_id
         FROM state_snapshots s
         JOIN audit_events e ON s.event_id = e.event_id
         WHERE s.bid_year_id = ?1 AND s.area_id = ?2 AND e.created_at <= ?3
         ORDER BY s.event_id DESC
         LIMIT 1",
    )
    .bind::<BigInt, _>(bid_year_id)
    .bind::<BigInt, _>(area_id)
    .bind::<Text, _>(timestamp)
    .get_result::<StateSnapshotRow>(conn);

    let row: StateSnapshotRow = match result {
        Ok(r) => r,
        Err(diesel::result::Error::NotFound) => {
            return Err(PersistenceError::SnapshotNotFound {
                bid_year: bid_year.year(),
                area: area.id().to_string(),
            });
        }
        Err(e) => return Err(PersistenceError::from(e)),
    };

    let state_data: StateData = serde_json::from_str(&row.state_json)?;
    let users: Vec<_> = serde_json::from_str(&state_data.users_json)?;

    Ok((
        State {
            bid_year: BidYear::new(state_data.bid_year),
            area: Area::new(&state_data.area),
            users,
        },
        row.event_id,
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

    let rows: Vec<AreaCountRow> = sql_query(
        "SELECT a.area_code, COUNT(*) as user_count
         FROM users u
         JOIN areas a ON u.area_id = a.area_id
         WHERE u.bid_year_id = ?1
         GROUP BY a.area_code
         ORDER BY a.area_code ASC",
    )
    .bind::<BigInt, _>(bid_year_id)
    .load::<AreaCountRow>(conn)?;

    let mut result: Vec<(String, usize)> = Vec::new();
    for row in rows {
        let count_usize: usize = row.user_count.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((row.area_code, count_usize));
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
    let rows: Vec<YearCountRow> = sql_query(
        "SELECT b.year, COUNT(*) as count
         FROM areas a
         JOIN bid_years b ON a.bid_year_id = b.bid_year_id
         GROUP BY b.year
         ORDER BY b.year ASC",
    )
    .load::<YearCountRow>(conn)?;

    let mut result: Vec<(u16, usize)> = Vec::new();
    for row in rows {
        let bid_year_u16: u16 = row.year.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = row.count.to_usize().ok_or_else(|| {
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
    let rows: Vec<YearCountRow> = sql_query(
        "SELECT b.year, COUNT(*) as count
         FROM users u
         JOIN bid_years b ON u.bid_year_id = b.bid_year_id
         GROUP BY b.year
         ORDER BY b.year ASC",
    )
    .load::<YearCountRow>(conn)?;

    let mut result: Vec<(u16, usize)> = Vec::new();
    for row in rows {
        let bid_year_u16: u16 = row.year.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = row.count.to_usize().ok_or_else(|| {
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
    let rows: Vec<YearAreaCountRow> = sql_query(
        "SELECT b.year, a.area_code, COUNT(*) as user_count
         FROM users u
         JOIN bid_years b ON u.bid_year_id = b.bid_year_id
         JOIN areas a ON u.area_id = a.area_id
         GROUP BY b.year, a.area_code
         ORDER BY b.year ASC, a.area_code ASC",
    )
    .load::<YearAreaCountRow>(conn)?;

    let mut result: Vec<(u16, String, usize)> = Vec::new();
    for row in rows {
        let bid_year_u16: u16 = row.year.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = row.user_count.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((bid_year_u16, row.area_code, count_usize));
    }

    Ok(result)
}
