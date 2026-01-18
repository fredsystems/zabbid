// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Bootstrap and orchestration mutations.
//!
//! This module contains high-level orchestration functions for persisting
//! bootstrap results and transitions. These functions coordinate multiple
//! lower-level mutations.

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use num_traits::ToPrimitive;
use tracing::{debug, info};
use zab_bid::{BootstrapResult, State, TransitionResult};
use zab_bid_domain::CanonicalBidYear;

use crate::backend::PersistenceBackend;
use crate::data_models::{
    NewCanonicalAreaMembership, NewCanonicalBidOrder, NewCanonicalBidWindows,
    NewCanonicalEligibility,
};
use crate::diesel_schema;
use crate::error::PersistenceError;
use crate::mutations::audit::{
    persist_audit_event_mysql, persist_audit_event_sqlite, persist_audit_event_with_ids_mysql,
    persist_audit_event_with_ids_sqlite, persist_state_snapshot_mysql,
    persist_state_snapshot_sqlite,
};
use crate::mutations::canonical::{
    bulk_insert_canonical_area_membership_mysql, bulk_insert_canonical_area_membership_sqlite,
    bulk_insert_canonical_bid_order_mysql, bulk_insert_canonical_bid_order_sqlite,
    bulk_insert_canonical_bid_windows_mysql, bulk_insert_canonical_bid_windows_sqlite,
    bulk_insert_canonical_eligibility_mysql, bulk_insert_canonical_eligibility_sqlite,
    insert_new_user_mysql, insert_new_user_sqlite, sync_canonical_users_mysql,
    sync_canonical_users_sqlite,
};
use crate::queries::canonical::{lookup_bid_year_id_mysql, lookup_bid_year_id_sqlite};

/// Persists a transition result (audit event and optionally a full snapshot) - `SQLite` version.
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
pub fn persist_transition_sqlite(
    conn: &mut SqliteConnection,
    result: &TransitionResult,
    should_snapshot: bool,
) -> Result<i64, PersistenceError> {
    // Persist the audit event
    let event_id: i64 = persist_audit_event_sqlite(conn, &result.audit_event)?;
    debug!(event_id, "Persisted audit event");

    // Update canonical state based on action type
    // RegisterUser is incremental (insert one user), others are full state replacement
    if result.audit_event.action.name.as_str() == "RegisterUser" {
        // Insert just the new user incrementally
        insert_new_user_sqlite(conn, &result.new_state)?;
        debug!(
            bid_year = result.new_state.bid_year.year(),
            area = result.new_state.area.id(),
            "Inserted new user"
        );
    } else {
        // For all other operations, do full state sync
        sync_canonical_users_sqlite(conn, &result.new_state)?;
        debug!(
            bid_year = result.new_state.bid_year.year(),
            area = result.new_state.area.id(),
            user_count = result.new_state.users.len(),
            "Synced canonical users table"
        );
    }

    // Persist full snapshot if required
    if should_snapshot {
        persist_state_snapshot_sqlite(conn, &result.new_state, event_id)?;
        debug!(event_id, "Persisted full state snapshot");
    }

    info!(event_id, should_snapshot, "Persisted transition");

    Ok(event_id)
}

/// Persists a transition result (audit event and optionally a full snapshot) - `MySQL` version.
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
pub fn persist_transition_mysql(
    conn: &mut MysqlConnection,
    result: &TransitionResult,
    should_snapshot: bool,
) -> Result<i64, PersistenceError> {
    // Persist the audit event
    let event_id: i64 = persist_audit_event_mysql(conn, &result.audit_event)?;
    debug!(event_id, "Persisted audit event");

    // Update canonical state based on action type
    // RegisterUser is incremental (insert one user), others are full state replacement
    if result.audit_event.action.name.as_str() == "RegisterUser" {
        // Insert just the new user incrementally
        insert_new_user_mysql(conn, &result.new_state)?;
        debug!(
            bid_year = result.new_state.bid_year.year(),
            area = result.new_state.area.id(),
            "Inserted new user"
        );
    } else {
        // For all other operations, do full state sync
        sync_canonical_users_mysql(conn, &result.new_state)?;
        debug!(
            bid_year = result.new_state.bid_year.year(),
            area = result.new_state.area.id(),
            user_count = result.new_state.users.len(),
            "Synced canonical users table"
        );
    }

    // Persist full snapshot if required
    if should_snapshot {
        persist_state_snapshot_mysql(conn, &result.new_state, event_id)?;
        debug!(event_id, "Persisted full state snapshot");
    }

    info!(event_id, should_snapshot, "Persisted transition");

    Ok(event_id)
}

/// Persists a bootstrap result (audit event for bid year/area creation) - `SQLite` version.
///
/// Phase 23A: This function inserts the canonical record first to obtain
/// the generated ID, then persists the audit event with both the ID and display values.
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
pub fn persist_bootstrap_sqlite(
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
            diesel::insert_into(diesel_schema::bid_years::table)
                .values((
                    diesel_schema::bid_years::year.eq(year_i32),
                    diesel_schema::bid_years::start_date.eq(&start_date_str),
                    diesel_schema::bid_years::num_pay_periods.eq(num_pay_periods_i32),
                ))
                .execute(conn)?;

            let bid_year_id: i64 = conn.get_last_insert_rowid()?;

            debug!(
                bid_year_id,
                bid_year = canonical.year(),
                start_date = %start_date_str,
                num_pay_periods = canonical.num_pay_periods(),
                "Inserted bid year with canonical metadata into canonical table"
            );

            // Persist audit event with the generated ID
            // Note: For CreateBidYear, area is a placeholder, so area_id is None
            let event_id: i64 = persist_audit_event_with_ids_sqlite(
                conn,
                &result.audit_event,
                Some(bid_year_id),
                None,
            )?;
            debug!(
                event_id,
                "Persisted bootstrap audit event for CreateBidYear"
            );

            info!(event_id, bid_year_id, "Persisted CreateBidYear");
            Ok(event_id)
        }
        "CreateArea" => {
            // Look up bid_year_id
            let bid_year_id: i64 = lookup_bid_year_id_sqlite(
                conn,
                result
                    .audit_event
                    .bid_year
                    .as_ref()
                    .expect("CreateArea must have bid_year")
                    .year(),
            )?;

            // Insert area and get generated ID
            diesel::insert_into(diesel_schema::areas::table)
                .values((
                    diesel_schema::areas::bid_year_id.eq(bid_year_id),
                    diesel_schema::areas::area_code.eq(result
                        .audit_event
                        .area
                        .as_ref()
                        .expect("CreateArea must have area")
                        .id()),
                ))
                .execute(conn)?;

            let area_id: i64 = conn.get_last_insert_rowid()?;

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
            let event_id: i64 = persist_audit_event_with_ids_sqlite(
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
            persist_state_snapshot_sqlite(conn, &initial_state, event_id)?;
            debug!(event_id, "Created initial empty snapshot for new area");

            info!(event_id, area_id, bid_year_id, "Persisted CreateArea");
            Ok(event_id)
        }
        _ => {
            // Non-bootstrap actions should use the standard persist path
            let event_id: i64 = persist_audit_event_sqlite(conn, &result.audit_event)?;
            debug!(event_id, "Persisted bootstrap audit event");
            info!(event_id, "Persisted bootstrap operation");
            Ok(event_id)
        }
    }
}

/// Persists a bootstrap result (audit event for bid year/area creation) - `MySQL` version.
///
/// Phase 23A: This function inserts the canonical record first to obtain
/// the generated ID, then persists the audit event with both the ID and display values.
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
pub fn persist_bootstrap_mysql(
    conn: &mut MysqlConnection,
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
            diesel::insert_into(diesel_schema::bid_years::table)
                .values((
                    diesel_schema::bid_years::year.eq(year_i32),
                    diesel_schema::bid_years::start_date.eq(&start_date_str),
                    diesel_schema::bid_years::num_pay_periods.eq(num_pay_periods_i32),
                ))
                .execute(conn)?;

            let bid_year_id: i64 = conn.get_last_insert_rowid()?;

            debug!(
                bid_year_id,
                bid_year = canonical.year(),
                start_date = %start_date_str,
                num_pay_periods = canonical.num_pay_periods(),
                "Inserted bid year with canonical metadata into canonical table"
            );

            // Persist audit event with the generated ID
            // Note: For CreateBidYear, area is a placeholder, so area_id is None
            let event_id: i64 = persist_audit_event_with_ids_mysql(
                conn,
                &result.audit_event,
                Some(bid_year_id),
                None,
            )?;
            debug!(
                event_id,
                "Persisted bootstrap audit event for CreateBidYear"
            );

            info!(event_id, bid_year_id, "Persisted CreateBidYear");
            Ok(event_id)
        }
        "CreateArea" => {
            // Look up bid_year_id
            let bid_year_id: i64 = lookup_bid_year_id_mysql(
                conn,
                result
                    .audit_event
                    .bid_year
                    .as_ref()
                    .expect("CreateArea must have bid_year")
                    .year(),
            )?;

            // Insert area and get generated ID
            diesel::insert_into(diesel_schema::areas::table)
                .values((
                    diesel_schema::areas::bid_year_id.eq(bid_year_id),
                    diesel_schema::areas::area_code.eq(result
                        .audit_event
                        .area
                        .as_ref()
                        .expect("CreateArea must have area")
                        .id()),
                ))
                .execute(conn)?;

            let area_id: i64 = conn.get_last_insert_rowid()?;

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
            let event_id: i64 = persist_audit_event_with_ids_mysql(
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
            persist_state_snapshot_mysql(conn, &initial_state, event_id)?;
            debug!(event_id, "Created initial empty snapshot for new area");

            info!(event_id, area_id, bid_year_id, "Persisted CreateArea");
            Ok(event_id)
        }
        _ => {
            // Non-bootstrap actions should use the standard persist path
            let event_id: i64 = persist_audit_event_mysql(conn, &result.audit_event)?;
            debug!(event_id, "Persisted bootstrap audit event");
            info!(event_id, "Persisted bootstrap operation");
            Ok(event_id)
        }
    }
}

backend_fn! {
/// Sets a bid year as active, ensuring only one bid year is active at a time.
///
/// This method atomically updates the active status:
/// 1. Clears the active flag from all bid years
/// 2. Sets the active flag on the specified bid year
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID to mark as active
///
/// # Errors
///
/// Returns an error if the database cannot be updated or the bid year doesn't exist.
pub fn set_active_bid_year(conn: &mut _, bid_year_id: i64) -> Result<(), PersistenceError> {
    // Clear active flag from all bid years
    diesel::update(diesel_schema::bid_years::table)
        .set(diesel_schema::bid_years::is_active.eq(0))
        .execute(conn)?;

    // Set active flag on specified bid year
    let rows_affected: usize = diesel::update(diesel_schema::bid_years::table)
        .filter(diesel_schema::bid_years::bid_year_id.eq(bid_year_id))
        .set(diesel_schema::bid_years::is_active.eq(1))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::NotFound(format!(
            "Bid year with ID {bid_year_id} not found"
        )));
    }

    info!(bid_year_id, "Set active bid year");
    Ok(())
}
}

backend_fn! {
/// Sets the expected area count for a bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `count` - The expected area count
///
/// # Errors
///
/// Returns an error if the database cannot be updated or the bid year doesn't exist.
pub fn set_expected_area_count(
    conn: &mut _,
    bid_year_id: i64,
    count: usize,
) -> Result<(), PersistenceError> {
    let count_i32: i32 = count
        .to_i32()
        .ok_or_else(|| PersistenceError::Other("Count out of range".to_string()))?;

    let rows_affected: usize = diesel::update(diesel_schema::bid_years::table)
        .filter(diesel_schema::bid_years::bid_year_id.eq(bid_year_id))
        .set(diesel_schema::bid_years::expected_area_count.eq(Some(count_i32)))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::NotFound(format!(
            "Bid year with ID {bid_year_id} not found"
        )));
    }

    debug!(bid_year_id, count, "Set expected area count");
    Ok(())
}
}

backend_fn! {
/// Sets the expected user count for an area.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `count` - The expected user count
///
/// # Errors
///
/// Returns an error if the database cannot be updated or the area doesn't exist.
pub fn set_expected_user_count(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
    count: usize,
) -> Result<(), PersistenceError> {
    let count_i32: i32 = count
        .to_i32()
        .ok_or_else(|| PersistenceError::Other("Count out of range".to_string()))?;

    let rows_affected: usize = diesel::update(diesel_schema::areas::table)
        .filter(diesel_schema::areas::bid_year_id.eq(bid_year_id))
        .filter(diesel_schema::areas::area_id.eq(area_id))
        .set(diesel_schema::areas::expected_user_count.eq(Some(count_i32)))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::NotFound(String::from("Area not found")));
    }

    debug!(bid_year_id, area_id, count, "Set expected user count");
    Ok(())
}
}

/// Canonicalize a bid year by populating canonical data tables (`SQLite` version).
///
/// This function:
/// 1. Inserts canonical rows for area membership, eligibility, bid order, and bid windows
/// 2. Persists the audit event
/// 3. Returns the `event_id`
///
/// Canonicalization must be called within a transaction to ensure atomicity.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The bid year to canonicalize
/// * `audit_event` - The audit event recording canonicalization
///
/// # Returns
///
/// The `event_id` of the persisted audit event.
///
/// # Errors
///
/// Returns an error if any database operation fails.
/// Helper to build canonical records and snapshot from user/area data.
#[allow(clippy::type_complexity)]
fn build_canonical_records_and_snapshot(
    bid_year_id: i64,
    year: i32,
    user_rows: &[(i64, String, String, i64, String, Option<String>)],
    area_rows: &[(i64, String, Option<String>)],
) -> Result<
    (
        Vec<NewCanonicalAreaMembership>,
        Vec<NewCanonicalEligibility>,
        Vec<NewCanonicalBidOrder>,
        Vec<NewCanonicalBidWindows>,
        crate::data_models::CanonicalizationSnapshot,
    ),
    PersistenceError,
> {
    let mut area_membership_records: Vec<NewCanonicalAreaMembership> = Vec::new();
    let mut eligibility_records: Vec<NewCanonicalEligibility> = Vec::new();
    let mut bid_order_records: Vec<NewCanonicalBidOrder> = Vec::new();
    let mut bid_windows_records: Vec<NewCanonicalBidWindows> = Vec::new();
    let mut snapshot_users: Vec<crate::data_models::CanonicalizedUserSnapshot> = Vec::new();

    for (user_id, initials, name, area_id, area_code, area_name) in user_rows {
        area_membership_records.push(NewCanonicalAreaMembership {
            bid_year_id,
            audit_event_id: 0,
            user_id: *user_id,
            area_id: *area_id,
            is_overridden: 0,
            override_reason: None,
        });

        eligibility_records.push(NewCanonicalEligibility {
            bid_year_id,
            audit_event_id: 0,
            user_id: *user_id,
            can_bid: 1,
            is_overridden: 0,
            override_reason: None,
        });

        bid_order_records.push(NewCanonicalBidOrder {
            bid_year_id,
            audit_event_id: 0,
            user_id: *user_id,
            bid_order: None,
            is_overridden: 0,
            override_reason: None,
        });

        bid_windows_records.push(NewCanonicalBidWindows {
            bid_year_id,
            audit_event_id: 0,
            user_id: *user_id,
            window_start_date: None,
            window_end_date: None,
            is_overridden: 0,
            override_reason: None,
        });

        snapshot_users.push(crate::data_models::CanonicalizedUserSnapshot {
            user_id: *user_id,
            initials: initials.clone(),
            name: name.clone(),
            area_id: *area_id,
            area_code: area_code.clone(),
            area_name: area_name.clone().unwrap_or_default(),
            can_bid: true,
            bid_order: None,
            window_start_date: None,
            window_end_date: None,
        });
    }

    let snapshot_areas: Vec<crate::data_models::CanonicalizedAreaSnapshot> = area_rows
        .iter()
        .map(|(area_id, area_code, area_name)| {
            let user_count = user_rows
                .iter()
                .filter(|(_, _, _, uid, _, _)| uid == area_id)
                .count();

            crate::data_models::CanonicalizedAreaSnapshot {
                area_id: *area_id,
                area_code: area_code.clone(),
                area_name: area_name.clone().unwrap_or_default(),
                user_count,
            }
        })
        .collect();

    let snapshot = crate::data_models::CanonicalizationSnapshot {
        bid_year_id,
        year: year.to_u16().ok_or_else(|| {
            PersistenceError::ReconstructionError("Year out of range".to_string())
        })?,
        user_count: user_rows.len(),
        area_count: area_rows.len(),
        users: snapshot_users,
        areas: snapshot_areas,
        timestamp: {
            use std::time::{SystemTime, UNIX_EPOCH};
            let duration = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("System time before UNIX epoch");
            format!("unix_{}", duration.as_secs())
        },
    };

    Ok((
        area_membership_records,
        eligibility_records,
        bid_order_records,
        bid_windows_records,
        snapshot,
    ))
}

pub fn canonicalize_bid_year_sqlite(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    audit_event: &zab_bid_audit::AuditEvent,
) -> Result<i64, PersistenceError> {
    use crate::diesel_schema::{areas, bid_years, canonical_area_membership, users};
    use crate::queries::canonical::canonical_rows_exist_sqlite;

    type UserWithAreaTuple = (i64, String, String, i64, String, Option<String>);
    type AreaTuple = (i64, String, Option<String>);

    if canonical_rows_exist_sqlite(conn, bid_year_id)? {
        info!(bid_year_id, "Canonicalization already complete");
        let existing_event_id: i64 = canonical_area_membership::table
            .filter(canonical_area_membership::bid_year_id.eq(bid_year_id))
            .select(canonical_area_membership::audit_event_id)
            .first(conn)?;
        return Ok(existing_event_id);
    }

    let user_rows: Vec<UserWithAreaTuple> = users::table
        .inner_join(areas::table.on(users::area_id.eq(areas::area_id)))
        .select((
            users::user_id,
            users::initials,
            users::name,
            areas::area_id,
            areas::area_code,
            areas::area_name,
        ))
        .filter(users::bid_year_id.eq(bid_year_id))
        .order(users::initials.asc())
        .load(conn)?;

    let area_rows: Vec<AreaTuple> = areas::table
        .select((areas::area_id, areas::area_code, areas::area_name))
        .filter(areas::bid_year_id.eq(bid_year_id))
        .order(areas::area_code.asc())
        .load(conn)?;

    let year: i32 = bid_years::table
        .select(bid_years::year)
        .filter(bid_years::bid_year_id.eq(bid_year_id))
        .first(conn)?;

    let (
        mut area_membership_records,
        mut eligibility_records,
        mut bid_order_records,
        mut bid_windows_records,
        snapshot,
    ) = build_canonical_records_and_snapshot(bid_year_id, year, &user_rows, &area_rows)?;

    let snapshot_json = serde_json::to_string(&snapshot)?;
    let mut audit_event_with_snapshot = audit_event.clone();
    audit_event_with_snapshot.after = zab_bid_audit::StateSnapshot::new(snapshot_json);

    let event_id: i64 = persist_audit_event_sqlite(conn, &audit_event_with_snapshot)?;

    for record in &mut area_membership_records {
        record.audit_event_id = event_id;
    }
    for record in &mut eligibility_records {
        record.audit_event_id = event_id;
    }
    for record in &mut bid_order_records {
        record.audit_event_id = event_id;
    }
    for record in &mut bid_windows_records {
        record.audit_event_id = event_id;
    }

    bulk_insert_canonical_area_membership_sqlite(conn, &area_membership_records)?;
    bulk_insert_canonical_eligibility_sqlite(conn, &eligibility_records)?;
    bulk_insert_canonical_bid_order_sqlite(conn, &bid_order_records)?;
    bulk_insert_canonical_bid_windows_sqlite(conn, &bid_windows_records)?;

    info!(
        event_id,
        bid_year_id,
        user_count = area_membership_records.len(),
        "Canonicalized bid year"
    );
    Ok(event_id)
}

/// Canonicalize a bid year by populating canonical data tables (`MySQL` version).
///
/// This function:
/// 1. Inserts canonical rows for area membership, eligibility, bid order, and bid windows
/// 2. Persists the audit event
/// 3. Returns the `event_id`
///
/// Canonicalization must be called within a transaction to ensure atomicity.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The bid year to canonicalize
/// * `audit_event` - The audit event recording canonicalization
///
/// # Returns
///
/// The `event_id` of the persisted audit event.
///
/// # Errors
///
/// Returns an error if any database operation fails.
pub fn canonicalize_bid_year_mysql(
    conn: &mut MysqlConnection,
    bid_year_id: i64,
    audit_event: &zab_bid_audit::AuditEvent,
) -> Result<i64, PersistenceError> {
    use crate::diesel_schema::{areas, bid_years, canonical_area_membership, users};
    use crate::queries::canonical::canonical_rows_exist_mysql;

    type UserWithAreaTuple = (i64, String, String, i64, String, Option<String>);
    type AreaTuple = (i64, String, Option<String>);

    if canonical_rows_exist_mysql(conn, bid_year_id)? {
        info!(bid_year_id, "Canonicalization already complete");
        let existing_event_id: i64 = canonical_area_membership::table
            .filter(canonical_area_membership::bid_year_id.eq(bid_year_id))
            .select(canonical_area_membership::audit_event_id)
            .first(conn)?;
        return Ok(existing_event_id);
    }

    let user_rows: Vec<UserWithAreaTuple> = users::table
        .inner_join(areas::table.on(users::area_id.eq(areas::area_id)))
        .select((
            users::user_id,
            users::initials,
            users::name,
            areas::area_id,
            areas::area_code,
            areas::area_name,
        ))
        .filter(users::bid_year_id.eq(bid_year_id))
        .order(users::initials.asc())
        .load(conn)?;

    let area_rows: Vec<AreaTuple> = areas::table
        .select((areas::area_id, areas::area_code, areas::area_name))
        .filter(areas::bid_year_id.eq(bid_year_id))
        .order(areas::area_code.asc())
        .load(conn)?;

    let year: i32 = bid_years::table
        .select(bid_years::year)
        .filter(bid_years::bid_year_id.eq(bid_year_id))
        .first(conn)?;

    let (
        mut area_membership_records,
        mut eligibility_records,
        mut bid_order_records,
        mut bid_windows_records,
        snapshot,
    ) = build_canonical_records_and_snapshot(bid_year_id, year, &user_rows, &area_rows)?;

    let snapshot_json = serde_json::to_string(&snapshot)?;
    let mut audit_event_with_snapshot = audit_event.clone();
    audit_event_with_snapshot.after = zab_bid_audit::StateSnapshot::new(snapshot_json);

    let event_id: i64 = persist_audit_event_mysql(conn, &audit_event_with_snapshot)?;

    for record in &mut area_membership_records {
        record.audit_event_id = event_id;
    }
    for record in &mut eligibility_records {
        record.audit_event_id = event_id;
    }
    for record in &mut bid_order_records {
        record.audit_event_id = event_id;
    }
    for record in &mut bid_windows_records {
        record.audit_event_id = event_id;
    }

    bulk_insert_canonical_area_membership_mysql(conn, &area_membership_records)?;
    bulk_insert_canonical_eligibility_mysql(conn, &eligibility_records)?;
    bulk_insert_canonical_bid_order_mysql(conn, &bid_order_records)?;
    bulk_insert_canonical_bid_windows_mysql(conn, &bid_windows_records)?;

    info!(
        event_id,
        bid_year_id,
        user_count = area_membership_records.len(),
        "Canonicalized bid year"
    );
    Ok(event_id)
}
