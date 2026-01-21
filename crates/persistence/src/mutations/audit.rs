// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Audit event and snapshot persistence.
//!
//! This module contains backend-agnostic mutations for persisting audit events
//! and state snapshots. Most mutations use Diesel DSL, with minimal backend-specific
//! helpers abstracted via the `PersistenceBackend` trait.

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use tracing::debug;
use zab_bid::State;
use zab_bid_audit::AuditEvent;
use zab_bid_domain::Area;

use crate::backend::PersistenceBackend;
use crate::data_models::{ActionData, ActorData, CauseData, StateData, StateSnapshotData};
use crate::diesel_schema;
use crate::error::PersistenceError;
use crate::queries::canonical::{
    lookup_area_id_mysql, lookup_area_id_sqlite, lookup_bid_year_id_mysql,
    lookup_bid_year_id_sqlite,
};

/// Persists an audit event (`SQLite` version).
///
/// Phase 23B: Handles both scoped and global events by looking up IDs when present.
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
pub fn persist_audit_event_sqlite(
    conn: &mut SqliteConnection,
    event: &AuditEvent,
) -> Result<i64, PersistenceError> {
    // Look up canonical IDs if bid_year and area are present (Phase 23B)
    let (bid_year_id, area_id): (Option<i64>, Option<i64>) = match (&event.bid_year, &event.area) {
        (Some(bid_year), Some(area)) => {
            // Both present - look up IDs
            let bid_year_id: i64 = lookup_bid_year_id_sqlite(conn, bid_year.year())?;
            let area_id: i64 = lookup_area_id_sqlite(conn, bid_year_id, area.id())?;
            (Some(bid_year_id), Some(area_id))
        }
        (Some(bid_year), None) => {
            // Only bid year present
            let bid_year_id: i64 = lookup_bid_year_id_sqlite(conn, bid_year.year())?;
            (Some(bid_year_id), None)
        }
        (None, _) => {
            // Global event - no bid year or area
            (None, None)
        }
    };

    persist_audit_event_with_ids_sqlite(conn, event, bid_year_id, area_id)
}

/// Persists an audit event (`MySQL` version).
///
/// Phase 23B: Handles both scoped and global events by looking up IDs when present.
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
pub fn persist_audit_event_mysql(
    conn: &mut MysqlConnection,
    event: &AuditEvent,
) -> Result<i64, PersistenceError> {
    // Look up canonical IDs if bid_year and area are present (Phase 23B)
    let (bid_year_id, area_id): (Option<i64>, Option<i64>) = match (&event.bid_year, &event.area) {
        (Some(bid_year), Some(area)) => {
            // Both present - look up IDs
            let bid_year_id: i64 = lookup_bid_year_id_mysql(conn, bid_year.year())?;
            let area_id: i64 = lookup_area_id_mysql(conn, bid_year_id, area.id())?;
            (Some(bid_year_id), Some(area_id))
        }
        (Some(bid_year), None) => {
            // Only bid year present
            let bid_year_id: i64 = lookup_bid_year_id_mysql(conn, bid_year.year())?;
            (Some(bid_year_id), None)
        }
        (None, _) => {
            // Global event - no bid year or area
            (None, None)
        }
    };

    persist_audit_event_with_ids_mysql(conn, event, bid_year_id, area_id)
}

backend_fn! {
/// Persists an audit event with explicit IDs.
///
/// This is used when IDs are already known (e.g., during bootstrap operations).
///
/// Phase 23A: `area_id` is optional to support `CreateBidYear` events.
/// Phase 23B: `bid_year_id` is also optional to support global events.
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
pub fn persist_audit_event_with_ids(
    conn: &mut _,
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
        // SAFETY: u16 always fits in i32
        i32::from(by.year())
    });
    let area_code: &str = event.area.as_ref().map_or("", Area::id);

    // Serialize JSON fields
    let actor_json: String = serde_json::to_string(&actor_data)?;
    let cause_json: String = serde_json::to_string(&cause_data)?;
    let action_json: String = serde_json::to_string(&action_data)?;
    let before_json: String = serde_json::to_string(&before_data)?;
    let after_json: String = serde_json::to_string(&after_data)?;

    diesel::insert_into(diesel_schema::audit_events::table)
        .values((
            diesel_schema::audit_events::bid_year_id.eq(bid_year_id),
            diesel_schema::audit_events::area_id.eq(area_id),
            diesel_schema::audit_events::year.eq(year),
            diesel_schema::audit_events::area_code.eq(area_code),
            diesel_schema::audit_events::actor_operator_id.eq(actor_operator_id),
            diesel_schema::audit_events::actor_login_name.eq(actor_login_name),
            diesel_schema::audit_events::actor_display_name.eq(actor_display_name),
            diesel_schema::audit_events::actor_json.eq(actor_json),
            diesel_schema::audit_events::cause_json.eq(cause_json),
            diesel_schema::audit_events::action_json.eq(action_json),
            diesel_schema::audit_events::before_snapshot_json.eq(before_json),
            diesel_schema::audit_events::after_snapshot_json.eq(after_json),
        ))
        .execute(conn)?;

    let event_id: i64 = conn.get_last_insert_rowid()?;

    Ok(event_id)
}
}

/// Persists a full state snapshot (`SQLite` version).
///
/// Phase 23A: Now looks up and uses canonical `bid_year_id` and `area_id`.
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
pub fn persist_state_snapshot_sqlite(
    conn: &mut SqliteConnection,
    state: &State,
    event_id: i64,
) -> Result<(), PersistenceError> {
    // Look up the canonical IDs (Phase 23A)
    let bid_year_id: i64 = lookup_bid_year_id_sqlite(conn, state.bid_year.year())?;
    let area_id: i64 = lookup_area_id_sqlite(conn, bid_year_id, state.area.id())?;

    let state_data: StateData = StateData {
        bid_year: state.bid_year.year(),
        area: state.area.id().to_string(),
        users_json: serde_json::to_string(&state.users)?,
    };

    let state_json: String = serde_json::to_string(&state_data)?;

    diesel::insert_into(diesel_schema::state_snapshots::table)
        .values((
            diesel_schema::state_snapshots::event_id.eq(event_id),
            diesel_schema::state_snapshots::bid_year_id.eq(bid_year_id),
            diesel_schema::state_snapshots::area_id.eq(area_id),
            diesel_schema::state_snapshots::state_json.eq(state_json),
        ))
        .execute(conn)?;

    debug!(event_id, "Persisted state snapshot");

    Ok(())
}

/// Persists a full state snapshot (`MySQL` version).
///
/// Phase 23A: Now looks up and uses canonical `bid_year_id` and `area_id`.
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
pub fn persist_state_snapshot_mysql(
    conn: &mut MysqlConnection,
    state: &State,
    event_id: i64,
) -> Result<(), PersistenceError> {
    // Look up the canonical IDs (Phase 23A)
    let bid_year_id: i64 = lookup_bid_year_id_mysql(conn, state.bid_year.year())?;
    let area_id: i64 = lookup_area_id_mysql(conn, bid_year_id, state.area.id())?;

    let state_data: StateData = StateData {
        bid_year: state.bid_year.year(),
        area: state.area.id().to_string(),
        users_json: serde_json::to_string(&state.users)?,
    };

    let state_json: String = serde_json::to_string(&state_data)?;

    diesel::insert_into(diesel_schema::state_snapshots::table)
        .values((
            diesel_schema::state_snapshots::event_id.eq(event_id),
            diesel_schema::state_snapshots::bid_year_id.eq(bid_year_id),
            diesel_schema::state_snapshots::area_id.eq(area_id),
            diesel_schema::state_snapshots::state_json.eq(state_json),
        ))
        .execute(conn)?;

    debug!(event_id, "Persisted state snapshot");

    Ok(())
}
