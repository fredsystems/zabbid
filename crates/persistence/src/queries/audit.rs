// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Audit event queries.
//!
//! This module contains backend-agnostic queries for retrieving audit events
//! and audit timelines. All queries use Diesel DSL and work across all
//! supported database backends.

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use num_traits::ToPrimitive;
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{Area, BidYear};

use crate::data_models::{ActionData, ActorData, CauseData, StateSnapshotData};
use crate::diesel_schema::audit_events;
use crate::error::PersistenceError;

/// Diesel Queryable struct for full audit event rows.
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

backend_fn! {
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
pub fn get_audit_event(conn: &mut _, event_id: i64) -> Result<AuditEvent, PersistenceError> {
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
        |id| Area::with_id(id, &row.area_code, None, false, None),
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
}

backend_fn! {
/// Retrieves all audit events for a `(bid_year, area)` scope after a given event ID.
///
/// Phase 23A: Now uses `bid_year_id` and `area_id` for queries.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `after_event_id` - Only return events after this ID (exclusive)
///
/// # Errors
///
/// Returns an error if events cannot be retrieved or deserialized.
pub fn get_events_after(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
    after_event_id: i64,
) -> Result<Vec<AuditEvent>, PersistenceError> {
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
                |id| Area::with_id(id, &row.area_code, None, false, None),
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
}

backend_fn! {
/// Retrieves the complete audit timeline for a given `(bid_year, area)` scope.
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
/// Returns an error if events cannot be retrieved or deserialized.
#[allow(clippy::too_many_lines)]
pub fn get_audit_timeline(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
) -> Result<Vec<AuditEvent>, PersistenceError> {
    tracing::debug!(bid_year_id, area_id, "Retrieving audit timeline");

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
                    Area::with_id(area_id, &area_code, None, false, None),
                ))
            },
        )
        .collect();

    let event_list: Vec<AuditEvent> = events?;

    tracing::info!(
        bid_year_id,
        area_id,
        event_count = event_list.len(),
        "Retrieved audit timeline"
    );

    Ok(event_list)
}
}

backend_fn! {
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
pub fn get_global_audit_events(conn: &mut _) -> Result<Vec<AuditEvent>, PersistenceError> {
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
}
