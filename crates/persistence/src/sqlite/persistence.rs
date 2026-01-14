// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use diesel::dsl::sql;
use diesel::sql_types::{BigInt, Integer, Nullable, Text};
use diesel::{RunQueryDsl, SqliteConnection, sql_query};
use num_traits::ToPrimitive;
use tracing::{debug, info};
use zab_bid::{BootstrapResult, State, TransitionResult};
use zab_bid_audit::AuditEvent;
use zab_bid_domain::{Area, CanonicalBidYear};

use crate::data_models::{ActionData, ActorData, CauseData, StateData, StateSnapshotData};
use crate::error::PersistenceError;

#[derive(diesel::QueryableByName)]
struct LastInsertRowid {
    #[diesel(sql_type = BigInt)]
    last_insert_rowid: i64,
}

/// Helper function to get the last inserted row ID (Diesel DSL alternative).
///
/// `SQLite` doesn't support `RETURNING` clauses, so we must query `last_insert_rowid()`.
/// This is a justified use of raw SQL as Diesel has no direct API for this.
///
/// Note: Currently unused but available for future migrations.
#[allow(dead_code)]
fn get_last_insert_rowid(conn: &mut SqliteConnection) -> Result<i64, PersistenceError> {
    Ok(diesel::select(sql::<BigInt>("last_insert_rowid()")).get_result(conn)?)
}

/// Persists a transition result (audit event and optionally a full snapshot).
///
/// # Arguments
///
/// * `conn` - The active database connection
/// * `result` - The transition result to persist
/// * `should_snapshot` - Whether to persist a full state snapshot
///
/// # Returns
///
/// The event ID assigned to the persisted audit event.
///
/// # Errors
///
/// Returns an error if persistence fails.
pub fn persist_transition(
    conn: &mut SqliteConnection,
    result: &TransitionResult,
    should_snapshot: bool,
) -> Result<i64, PersistenceError> {
    // Persist the audit event
    let event_id: i64 = persist_audit_event(conn, &result.audit_event)?;
    debug!(event_id, "Persisted audit event");

    // Update canonical state based on action type
    // RegisterUser is incremental (insert one user), others are full state replacement
    if result.audit_event.action.name.as_str() == "RegisterUser" {
        // Insert just the new user incrementally
        insert_new_user_tx(conn, &result.new_state)?;
        debug!(
            bid_year = result.new_state.bid_year.year(),
            area = result.new_state.area.id(),
            "Inserted new user"
        );
    } else {
        // For all other operations, do full state sync
        sync_canonical_users_tx(conn, &result.new_state)?;
        debug!(
            bid_year = result.new_state.bid_year.year(),
            area = result.new_state.area.id(),
            user_count = result.new_state.users.len(),
            "Synced canonical users table"
        );
    }

    // Persist full snapshot if required
    if should_snapshot {
        persist_state_snapshot_tx(conn, &result.new_state, event_id)?;
        debug!(event_id, "Persisted full state snapshot");
    }

    info!(event_id, should_snapshot, "Persisted transition");

    Ok(event_id)
}

/// Persists a bootstrap result (audit event for bid year/area creation).
///
/// Phase 23A: This function must insert the canonical record first to obtain
/// the generated ID, then persist the audit event with both the ID and display values.
///
/// # Arguments
///
/// * `conn` - The active database connection
/// * `result` - The bootstrap result to persist
///
/// # Returns
///
/// The event ID assigned to the persisted audit event.
///
/// # Errors
///
/// Returns an error if persistence fails.
#[allow(clippy::too_many_lines)]
pub fn persist_bootstrap(
    conn: &mut SqliteConnection,
    result: &BootstrapResult,
) -> Result<i64, PersistenceError> {
    // Update canonical tables first to generate IDs
    match result.audit_event.action.name.as_str() {
        "CreateBidYear" => {
            // Extract canonical bid year metadata
            let canonical: &CanonicalBidYear = result
                .canonical_bid_year
                .as_ref()
                .expect("CreateBidYear must include canonical_bid_year");

            // Format date as ISO 8601 string for storage
            let start_date_str: String = canonical.start_date().to_string();
            let year_i32: i32 = canonical
                .year()
                .to_i32()
                .ok_or_else(|| PersistenceError::Other("Year out of range".to_string()))?;
            let num_pay_periods_i32: i32 =
                canonical.num_pay_periods().to_i32().ok_or_else(|| {
                    PersistenceError::Other("num_pay_periods out of range".to_string())
                })?;

            // Insert bid year and get generated ID
            sql_query(
                "INSERT INTO bid_years (year, start_date, num_pay_periods) VALUES (?1, ?2, ?3)",
            )
            .bind::<Integer, _>(year_i32)
            .bind::<Text, _>(&start_date_str)
            .bind::<Integer, _>(num_pay_periods_i32)
            .execute(conn)?;

            let bid_year_id: i64 = sql_query("SELECT last_insert_rowid() as last_insert_rowid")
                .get_result::<LastInsertRowid>(conn)?
                .last_insert_rowid;

            debug!(
                bid_year_id,
                bid_year = canonical.year(),
                start_date = %start_date_str,
                num_pay_periods = canonical.num_pay_periods(),
                "Inserted bid year with canonical metadata into canonical table"
            );

            // Persist audit event with the generated ID
            // Note: For CreateBidYear, area is a placeholder, so area_id is None
            let event_id: i64 =
                persist_audit_event_with_ids(conn, &result.audit_event, Some(bid_year_id), None)?;
            debug!(
                event_id,
                "Persisted bootstrap audit event for CreateBidYear"
            );

            info!(event_id, bid_year_id, "Persisted CreateBidYear");
            Ok(event_id)
        }
        "CreateArea" => {
            // Look up bid_year_id
            let bid_year_id: i64 = crate::sqlite::queries::lookup_bid_year_id_tx(
                conn,
                result
                    .audit_event
                    .bid_year
                    .as_ref()
                    .expect("CreateArea must have bid_year")
                    .year(),
            )?;

            // Insert area and get generated ID
            sql_query("INSERT INTO areas (bid_year_id, area_code) VALUES (?1, ?2)")
                .bind::<BigInt, _>(bid_year_id)
                .bind::<Text, _>(
                    result
                        .audit_event
                        .area
                        .as_ref()
                        .expect("CreateArea must have area")
                        .id(),
                )
                .execute(conn)?;

            let area_id: i64 = sql_query("SELECT last_insert_rowid() as last_insert_rowid")
                .get_result::<LastInsertRowid>(conn)?
                .last_insert_rowid;

            debug!(
                area_id,
                bid_year_id,
                area_code = result
                    .audit_event
                    .area
                    .as_ref()
                    .expect("CreateArea must have area")
                    .id(),
                "Inserted area into canonical table"
            );

            // Persist audit event with the generated IDs
            let event_id: i64 = persist_audit_event_with_ids(
                conn,
                &result.audit_event,
                Some(bid_year_id),
                Some(area_id),
            )?;
            debug!(event_id, "Persisted bootstrap audit event for CreateArea");

            // Create an initial empty snapshot for new areas
            let initial_state: State = State::new(
                result
                    .audit_event
                    .bid_year
                    .clone()
                    .expect("CreateArea must have bid_year"),
                result
                    .audit_event
                    .area
                    .clone()
                    .expect("CreateArea must have area"),
            );
            persist_state_snapshot_tx(conn, &initial_state, event_id)?;
            debug!(event_id, "Created initial empty snapshot for new area");

            info!(event_id, area_id, bid_year_id, "Persisted CreateArea");
            Ok(event_id)
        }
        _ => {
            // Non-bootstrap actions should use the standard persist path
            let event_id: i64 = persist_audit_event(conn, &result.audit_event)?;
            debug!(event_id, "Persisted bootstrap audit event");
            info!(event_id, "Persisted bootstrap operation");
            Ok(event_id)
        }
    }
}

/// Persists an audit event within a transaction.
///
/// Phase 23A: This function now looks up the `bid_year_id` and `area_id`
/// from the `BidYear` and `Area` objects before persisting.
///
/// # Arguments
///
/// * `conn` - The active database connection
/// * `event` - The audit event to persist
///
/// # Returns
///
/// The event ID assigned by the database.
///
/// # Errors
///
/// Returns an error if persistence or serialization fails.
pub fn persist_audit_event(
    conn: &mut SqliteConnection,
    event: &AuditEvent,
) -> Result<i64, PersistenceError> {
    // Look up canonical IDs if bid_year and area are present (Phase 23B)
    let (bid_year_id, area_id): (Option<i64>, Option<i64>) = match (&event.bid_year, &event.area) {
        (Some(bid_year), Some(area)) => {
            // Both present - look up IDs
            let bid_year_id: i64 =
                crate::sqlite::queries::lookup_bid_year_id_tx(conn, bid_year.year())?;
            let area_id: i64 =
                crate::sqlite::queries::lookup_area_id_tx(conn, bid_year_id, area.id())?;
            (Some(bid_year_id), Some(area_id))
        }
        (Some(bid_year), None) => {
            // Only bid year present
            let bid_year_id: i64 =
                crate::sqlite::queries::lookup_bid_year_id_tx(conn, bid_year.year())?;
            (Some(bid_year_id), None)
        }
        (None, _) => {
            // Global event - no bid year or area
            (None, None)
        }
    };

    persist_audit_event_with_ids(conn, event, bid_year_id, area_id)
}

/// Persists an audit event with explicit IDs within a transaction.
///
/// This is an internal helper used when IDs are already known
/// (e.g., during bootstrap operations).
///
/// Phase 23A: `area_id` is optional to support `CreateBidYear` events
/// where the area is not meaningful.
///
/// Phase 23B: `bid_year_id` is also optional to support global events
/// like operator management.
///
/// # Arguments
///
/// * `conn` - The active database connection
/// * `event` - The audit event to persist
/// * `bid_year_id` - The bid year ID (None for global events)
/// * `area_id` - The area ID (None for global or bid-year-only events)
///
/// # Returns
///
/// The event ID assigned by the database.
///
/// # Errors
///
/// Returns an error if persistence or serialization fails.
fn persist_audit_event_with_ids(
    conn: &mut SqliteConnection,
    event: &AuditEvent,
    bid_year_id: Option<i64>,
    area_id: Option<i64>,
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

    // Extract operator information (Phase 14)
    let actor_operator_id: i64 = event.actor.operator_id.unwrap_or(0);
    let actor_login_name: String = event
        .actor
        .operator_login_name
        .as_deref()
        .unwrap_or("system")
        .to_string();
    let actor_display_name: String = event
        .actor
        .operator_display_name
        .as_deref()
        .unwrap_or("System")
        .to_string();

    // Extract display values (may be placeholders for global events)
    let year: i32 = event.bid_year.as_ref().map_or(0, |by| {
        by.year().to_i32().expect("Year value out of i32 range")
    });
    let area_code: &str = event.area.as_ref().map_or("", Area::id);

    // Serialize JSON fields
    let actor_json: String = serde_json::to_string(&actor_data)?;
    let cause_json: String = serde_json::to_string(&cause_data)?;
    let action_json: String = serde_json::to_string(&action_data)?;
    let before_json: String = serde_json::to_string(&before_data)?;
    let after_json: String = serde_json::to_string(&after_data)?;

    sql_query(
        "INSERT INTO audit_events (
            bid_year_id, area_id, year, area_code,
            actor_operator_id, actor_login_name, actor_display_name,
            actor_json, cause_json, action_json,
            before_snapshot_json, after_snapshot_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind::<Nullable<BigInt>, _>(bid_year_id)
    .bind::<Nullable<BigInt>, _>(area_id)
    .bind::<Integer, _>(year)
    .bind::<Text, _>(area_code)
    .bind::<BigInt, _>(actor_operator_id)
    .bind::<Text, _>(actor_login_name)
    .bind::<Text, _>(actor_display_name)
    .bind::<Text, _>(actor_json)
    .bind::<Text, _>(cause_json)
    .bind::<Text, _>(action_json)
    .bind::<Text, _>(before_json)
    .bind::<Text, _>(after_json)
    .execute(conn)?;

    let event_id: i64 = sql_query("SELECT last_insert_rowid() as last_insert_rowid")
        .get_result::<LastInsertRowid>(conn)?
        .last_insert_rowid;

    Ok(event_id)
}

/// Persists a full state snapshot within a transaction.
///
/// Phase 23A: Now looks up and uses `bid_year_id` and `area_id`.
///
/// # Arguments
///
/// * `conn` - The active database connection
/// * `state` - The state to snapshot
/// * `event_id` - The associated audit event ID
///
/// # Errors
///
/// Returns an error if persistence or serialization fails.
fn persist_state_snapshot_tx(
    conn: &mut SqliteConnection,
    state: &State,
    event_id: i64,
) -> Result<(), PersistenceError> {
    // Look up the IDs
    let bid_year_id: i64 =
        crate::sqlite::queries::lookup_bid_year_id_tx(conn, state.bid_year.year())?;
    let area_id: i64 =
        crate::sqlite::queries::lookup_area_id_tx(conn, bid_year_id, state.area.id())?;

    let state_data: StateData = StateData {
        bid_year: state.bid_year.year(),
        area: state.area.id().to_string(),
        users_json: serde_json::to_string(&state.users)?,
    };

    let state_json: String = serde_json::to_string(&state_data)?;

    sql_query(
        "INSERT INTO state_snapshots (bid_year_id, area_id, event_id, state_json)
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind::<BigInt, _>(bid_year_id)
    .bind::<BigInt, _>(area_id)
    .bind::<BigInt, _>(event_id)
    .bind::<Text, _>(state_json)
    .execute(conn)?;

    Ok(())
}

/// Inserts a single new user (the last user in the state) into the canonical users table.
///
/// This is used for incremental `RegisterUser` operations, inserting only the newly added user
/// rather than replacing the entire table. Expects the new user to not have a `user_id` yet.
///
/// Phase 23A: Now looks up and uses `bid_year_id` and `area_id`.
///
/// # Arguments
///
/// * `conn` - The active database connection
/// * `state` - The state containing all users, where the last one is the new user
///
/// # Errors
///
/// Returns an error if the state has no users or if the database operation fails.
fn insert_new_user_tx(conn: &mut SqliteConnection, state: &State) -> Result<(), PersistenceError> {
    let user = state
        .users
        .last()
        .ok_or_else(|| PersistenceError::ReconstructionError("No users in state".to_string()))?;

    // User should not have user_id for a new insertion
    if user.user_id.is_some() {
        return Err(PersistenceError::ReconstructionError(
            "New user should not have user_id".to_string(),
        ));
    }

    // Look up the IDs
    let bid_year_id: i64 =
        crate::sqlite::queries::lookup_bid_year_id_tx(conn, user.bid_year.year())?;
    let area_id: i64 =
        crate::sqlite::queries::lookup_area_id_tx(conn, bid_year_id, user.area.id())?;

    // Seniority data fields are already strings - just borrow them
    let cumulative_natca_bu_date: &str = &user.seniority_data.cumulative_natca_bu_date;
    let natca_bu_date: &str = &user.seniority_data.natca_bu_date;
    let eod_faa_date: &str = &user.seniority_data.eod_faa_date;
    let service_computation_date: &str = &user.seniority_data.service_computation_date;

    // Insert new user and let SQLite assign user_id
    sql_query(
        "INSERT INTO users (
            bid_year_id, area_id, initials, name, user_type, crew,
            cumulative_natca_bu_date, natca_bu_date,
            eod_faa_date, service_computation_date, lottery_value
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )
    .bind::<BigInt, _>(bid_year_id)
    .bind::<BigInt, _>(area_id)
    .bind::<Text, _>(user.initials.value())
    .bind::<Text, _>(&user.name)
    .bind::<Text, _>(user.user_type.as_str())
    .bind::<Nullable<Integer>, _>(
        user.crew
            .as_ref()
            .map(|c| c.number().to_i32().expect("Crew number out of range")),
    )
    .bind::<Text, _>(cumulative_natca_bu_date)
    .bind::<Text, _>(natca_bu_date)
    .bind::<Text, _>(eod_faa_date)
    .bind::<Text, _>(service_computation_date)
    .bind::<Nullable<Integer>, _>(user.seniority_data.lottery_value.and_then(|v| v.to_i32()))
    .execute(conn)?;

    Ok(())
}

/// Syncs the canonical users table to match the given state.
///
/// This is an idempotent operation that replaces all users for the given
/// `(bid_year, area)` with the users in the provided state.
///
/// Users with existing `user_id` values are updated in place.
/// Users without `user_id` are inserted as new rows.
///
/// Phase 23A: Now looks up and uses `bid_year_id` and `area_id`.
///
/// # Arguments
///
/// * `conn` - The active database connection
/// * `state` - The state containing users to sync
///
/// # Errors
///
/// Returns an error if the database operation fails.
fn sync_canonical_users_tx(
    conn: &mut SqliteConnection,
    state: &State,
) -> Result<(), PersistenceError> {
    // Look up the IDs
    let bid_year_id: i64 =
        crate::sqlite::queries::lookup_bid_year_id_tx(conn, state.bid_year.year())?;
    let area_id: i64 =
        crate::sqlite::queries::lookup_area_id_tx(conn, bid_year_id, state.area.id())?;

    // Delete all existing users for this (bid_year_id, area_id)
    sql_query("DELETE FROM users WHERE bid_year_id = ?1 AND area_id = ?2")
        .bind::<BigInt, _>(bid_year_id)
        .bind::<BigInt, _>(area_id)
        .execute(conn)?;

    // Insert all users from the new state
    for user in &state.users {
        // Seniority data fields are already strings - just borrow them
        let cumulative_natca_bu_date: &str = &user.seniority_data.cumulative_natca_bu_date;
        let natca_bu_date: &str = &user.seniority_data.natca_bu_date;
        let eod_faa_date: &str = &user.seniority_data.eod_faa_date;
        let service_computation_date: &str = &user.seniority_data.service_computation_date;

        if let Some(user_id) = user.user_id {
            // User has an existing user_id, insert with explicit ID
            sql_query(
                "INSERT INTO users (
                    user_id, bid_year_id, area_id, initials, name, user_type, crew,
                    cumulative_natca_bu_date, natca_bu_date,
                    eod_faa_date, service_computation_date, lottery_value
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )
            .bind::<BigInt, _>(user_id)
            .bind::<BigInt, _>(bid_year_id)
            .bind::<BigInt, _>(area_id)
            .bind::<Text, _>(user.initials.value())
            .bind::<Text, _>(&user.name)
            .bind::<Text, _>(user.user_type.as_str())
            .bind::<Nullable<Integer>, _>(
                user.crew
                    .as_ref()
                    .map(|c| c.number().to_i32().expect("Crew number out of range")),
            )
            .bind::<Text, _>(cumulative_natca_bu_date)
            .bind::<Text, _>(natca_bu_date)
            .bind::<Text, _>(eod_faa_date)
            .bind::<Text, _>(service_computation_date)
            .bind::<Nullable<Integer>, _>(
                user.seniority_data.lottery_value.and_then(|v| v.to_i32()),
            )
            .execute(conn)?;
        } else {
            // User has no user_id, insert and let SQLite assign one
            sql_query(
                "INSERT INTO users (
                    bid_year_id, area_id, initials, name, user_type, crew,
                    cumulative_natca_bu_date, natca_bu_date,
                    eod_faa_date, service_computation_date, lottery_value
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )
            .bind::<BigInt, _>(bid_year_id)
            .bind::<BigInt, _>(area_id)
            .bind::<Text, _>(user.initials.value())
            .bind::<Text, _>(&user.name)
            .bind::<Text, _>(user.user_type.as_str())
            .bind::<Nullable<Integer>, _>(
                user.crew
                    .as_ref()
                    .map(|c| c.number().to_i32().expect("Crew number out of range")),
            )
            .bind::<Text, _>(cumulative_natca_bu_date)
            .bind::<Text, _>(natca_bu_date)
            .bind::<Text, _>(eod_faa_date)
            .bind::<Text, _>(service_computation_date)
            .bind::<Nullable<Integer>, _>(
                user.seniority_data.lottery_value.and_then(|v| v.to_i32()),
            )
            .execute(conn)?;
        }
    }

    Ok(())
}
