// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use rusqlite::{Transaction, params};
use tracing::{debug, info};
use zab_bid::{BootstrapResult, State, TransitionResult};
use zab_bid_audit::AuditEvent;
use zab_bid_domain::{CanonicalBidYear, Crew};

use crate::data_models::{ActionData, ActorData, CauseData, StateData, StateSnapshotData};
use crate::error::PersistenceError;

/// Persists a transition result (audit event and optionally a full snapshot).
///
/// # Arguments
///
/// * `tx` - The active database transaction
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
    tx: &Transaction<'_>,
    result: &TransitionResult,
    should_snapshot: bool,
) -> Result<i64, PersistenceError> {
    // Persist the audit event
    let event_id: i64 = persist_audit_event(tx, &result.audit_event)?;
    debug!(event_id, "Persisted audit event");

    // Update canonical state based on action type
    // RegisterUser is incremental (insert one user), others are full state replacement
    if result.audit_event.action.name.as_str() == "RegisterUser" {
        // Insert just the new user incrementally
        insert_new_user_tx(tx, &result.new_state)?;
        debug!(
            bid_year = result.new_state.bid_year.year(),
            area = result.new_state.area.id(),
            "Inserted new user"
        );
    } else {
        // For all other operations, do full state sync
        sync_canonical_users_tx(tx, &result.new_state)?;
        debug!(
            bid_year = result.new_state.bid_year.year(),
            area = result.new_state.area.id(),
            user_count = result.new_state.users.len(),
            "Synced canonical users table"
        );
    }

    // Persist full snapshot if required
    if should_snapshot {
        persist_state_snapshot_tx(tx, &result.new_state, event_id)?;
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
/// * `tx` - The active database transaction
/// * `result` - The bootstrap result to persist
///
/// # Returns
///
/// The event ID assigned to the persisted audit event.
///
/// # Errors
///
/// Returns an error if persistence fails.
pub fn persist_bootstrap(
    tx: &Transaction<'_>,
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

            // Insert bid year and get generated ID
            tx.execute(
                "INSERT INTO bid_years (year, start_date, num_pay_periods) VALUES (?1, ?2, ?3)",
                params![
                    canonical.year(),
                    start_date_str,
                    canonical.num_pay_periods()
                ],
            )?;
            let bid_year_id: i64 = tx.last_insert_rowid();
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
                persist_audit_event_with_ids(tx, &result.audit_event, bid_year_id, None)?;
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
                tx,
                result.audit_event.bid_year.year(),
            )?;

            // Insert area and get generated ID
            tx.execute(
                "INSERT INTO areas (bid_year_id, area_code) VALUES (?1, ?2)",
                params![bid_year_id, result.audit_event.area.id()],
            )?;
            let area_id: i64 = tx.last_insert_rowid();
            debug!(
                area_id,
                bid_year_id,
                area_code = result.audit_event.area.id(),
                "Inserted area into canonical table"
            );

            // Persist audit event with the generated IDs
            let event_id: i64 =
                persist_audit_event_with_ids(tx, &result.audit_event, bid_year_id, Some(area_id))?;
            debug!(event_id, "Persisted bootstrap audit event for CreateArea");

            // Create an initial empty snapshot for new areas
            let initial_state: State = State::new(
                result.audit_event.bid_year.clone(),
                result.audit_event.area.clone(),
            );
            persist_state_snapshot_tx(tx, &initial_state, event_id)?;
            debug!(event_id, "Created initial empty snapshot for new area");

            info!(event_id, area_id, bid_year_id, "Persisted CreateArea");
            Ok(event_id)
        }
        _ => {
            // Non-bootstrap actions should use the standard persist path
            let event_id: i64 = persist_audit_event(tx, &result.audit_event)?;
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
/// * `tx` - The active database transaction
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
    tx: &Transaction<'_>,
    event: &AuditEvent,
) -> Result<i64, PersistenceError> {
    // Look up canonical IDs - these must exist or we fail
    let bid_year_id: i64 =
        crate::sqlite::queries::lookup_bid_year_id_tx(tx, event.bid_year.year())?;
    let area_id: i64 = crate::sqlite::queries::lookup_area_id_tx(tx, bid_year_id, event.area.id())?;

    persist_audit_event_with_ids(tx, event, bid_year_id, Some(area_id))
}

/// Persists an audit event with explicit IDs within a transaction.
///
/// This is an internal helper used when IDs are already known
/// (e.g., during bootstrap operations).
///
/// Phase 23A: `area_id` is optional to support `CreateBidYear` events
/// where the area is not meaningful.
///
/// # Arguments
///
/// * `tx` - The active database transaction
/// * `event` - The audit event to persist
/// * `bid_year_id` - The bid year ID
/// * `area_id` - The area ID (`None` for `CreateBidYear`)
///
/// # Returns
///
/// The event ID assigned by the database.
///
/// # Errors
///
/// Returns an error if persistence or serialization fails.
fn persist_audit_event_with_ids(
    tx: &Transaction<'_>,
    event: &AuditEvent,
    bid_year_id: i64,
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

    tx.execute(
        "INSERT INTO audit_events (
            bid_year_id, area_id, year, area_code,
            actor_operator_id, actor_login_name, actor_display_name,
            actor_json, cause_json, action_json,
            before_snapshot_json, after_snapshot_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            bid_year_id,
            area_id,
            event.bid_year.year(),
            event.area.id(),
            actor_operator_id,
            actor_login_name,
            actor_display_name,
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
///
/// Phase 23A: Now looks up and uses `bid_year_id` and `area_id`.
///
/// # Arguments
///
/// * `tx` - The active database transaction
/// * `state` - The state to snapshot
/// * `event_id` - The associated audit event ID
///
/// # Errors
///
/// Returns an error if persistence or serialization fails.
fn persist_state_snapshot_tx(
    tx: &Transaction<'_>,
    state: &State,
    event_id: i64,
) -> Result<(), PersistenceError> {
    // Look up the IDs
    let bid_year_id: i64 =
        crate::sqlite::queries::lookup_bid_year_id_tx(tx, state.bid_year.year())?;
    let area_id: i64 = crate::sqlite::queries::lookup_area_id_tx(tx, bid_year_id, state.area.id())?;

    let state_data: StateData = StateData {
        bid_year: state.bid_year.year(),
        area: state.area.id().to_string(),
        users_json: serde_json::to_string(&state.users)?,
    };

    tx.execute(
        "INSERT INTO state_snapshots (bid_year_id, area_id, event_id, state_json)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            bid_year_id,
            area_id,
            event_id,
            serde_json::to_string(&state_data)?,
        ],
    )?;

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
/// * `tx` - The active database transaction
/// * `state` - The state containing all users, where the last one is the new user
///
/// # Errors
///
/// Returns an error if the state has no users or if the database operation fails.
fn insert_new_user_tx(tx: &Transaction<'_>, state: &State) -> Result<(), PersistenceError> {
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
    let bid_year_id: i64 = crate::sqlite::queries::lookup_bid_year_id_tx(tx, user.bid_year.year())?;
    let area_id: i64 = crate::sqlite::queries::lookup_area_id_tx(tx, bid_year_id, user.area.id())?;

    // Insert new user and let SQLite assign user_id
    tx.execute(
        "INSERT INTO users (
            bid_year_id, area_id, initials, name, user_type, crew,
            cumulative_natca_bu_date, natca_bu_date,
            eod_faa_date, service_computation_date, lottery_value
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            bid_year_id,
            area_id,
            user.initials.value(),
            user.name,
            user.user_type.as_str(),
            user.crew.as_ref().map(Crew::number),
            user.seniority_data.cumulative_natca_bu_date,
            user.seniority_data.natca_bu_date,
            user.seniority_data.eod_faa_date,
            user.seniority_data.service_computation_date,
            user.seniority_data.lottery_value,
        ],
    )?;

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
/// * `tx` - The active database transaction
/// * `state` - The state containing users to sync
///
/// # Errors
///
/// Returns an error if the database operation fails.
fn sync_canonical_users_tx(tx: &Transaction<'_>, state: &State) -> Result<(), PersistenceError> {
    // Look up the IDs
    let bid_year_id: i64 =
        crate::sqlite::queries::lookup_bid_year_id_tx(tx, state.bid_year.year())?;
    let area_id: i64 = crate::sqlite::queries::lookup_area_id_tx(tx, bid_year_id, state.area.id())?;

    // Delete all existing users for this (bid_year_id, area_id)
    tx.execute(
        "DELETE FROM users WHERE bid_year_id = ?1 AND area_id = ?2",
        params![bid_year_id, area_id],
    )?;

    // Insert all users from the new state
    for user in &state.users {
        if let Some(user_id) = user.user_id {
            // User has an existing user_id, insert with explicit ID
            tx.execute(
                "INSERT INTO users (
                    user_id, bid_year_id, area_id, initials, name, user_type, crew,
                    cumulative_natca_bu_date, natca_bu_date,
                    eod_faa_date, service_computation_date, lottery_value
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    user_id,
                    bid_year_id,
                    area_id,
                    user.initials.value(),
                    user.name,
                    user.user_type.as_str(),
                    user.crew.as_ref().map(Crew::number),
                    user.seniority_data.cumulative_natca_bu_date,
                    user.seniority_data.natca_bu_date,
                    user.seniority_data.eod_faa_date,
                    user.seniority_data.service_computation_date,
                    user.seniority_data.lottery_value,
                ],
            )?;
        } else {
            // User has no user_id, insert and let SQLite assign one
            tx.execute(
                "INSERT INTO users (
                    bid_year_id, area_id, initials, name, user_type, crew,
                    cumulative_natca_bu_date, natca_bu_date,
                    eod_faa_date, service_computation_date, lottery_value
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    bid_year_id,
                    area_id,
                    user.initials.value(),
                    user.name,
                    user.user_type.as_str(),
                    user.crew.as_ref().map(Crew::number),
                    user.seniority_data.cumulative_natca_bu_date,
                    user.seniority_data.natca_bu_date,
                    user.seniority_data.eod_faa_date,
                    user.seniority_data.service_computation_date,
                    user.seniority_data.lottery_value,
                ],
            )?;
        }
    }

    Ok(())
}
