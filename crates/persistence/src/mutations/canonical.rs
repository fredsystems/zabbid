// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Canonical entity mutations.
//!
//! This module contains backend-agnostic mutations for persisting canonical
//! entities (users, bid years, areas). All mutations use Diesel DSL and work
//! across all supported database backends.

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use num_traits::ToPrimitive;
use tracing::debug;
use zab_bid::State;
use zab_bid_domain::{Area, Initials};

use crate::data_models::{
    NewCanonicalAreaMembership, NewCanonicalBidOrder, NewCanonicalBidWindows,
    NewCanonicalEligibility,
};
use crate::diesel_schema;
use crate::error::PersistenceError;
use crate::queries::canonical::{
    lookup_area_id_mysql, lookup_area_id_sqlite, lookup_bid_year_id_mysql,
    lookup_bid_year_id_sqlite,
};

/// Inserts a new user from the last user in the state (`SQLite` version).
///
/// This is used for incremental `RegisterUser` operations where only one user
/// is being added.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `state` - The state containing the new user (as the last element)
///
/// # Errors
///
/// Returns an error if the state has no users or if the database operation fails.
pub fn insert_new_user_sqlite(
    conn: &mut SqliteConnection,
    state: &State,
) -> Result<i64, PersistenceError> {
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
    let bid_year_id: i64 = lookup_bid_year_id_sqlite(conn, user.bid_year.year())?;
    let area_id: i64 = lookup_area_id_sqlite(conn, bid_year_id, user.area.id())?;

    // Seniority data fields are already strings - just borrow them
    let cumulative_natca_bu_date: &str = &user.seniority_data.cumulative_natca_bu_date;
    let natca_bu_date: &str = &user.seniority_data.natca_bu_date;
    let eod_faa_date: &str = &user.seniority_data.eod_faa_date;
    let service_computation_date: &str = &user.seniority_data.service_computation_date;

    // Insert new user and let database assign user_id
    diesel::insert_into(diesel_schema::users::table)
        .values((
            diesel_schema::users::bid_year_id.eq(bid_year_id),
            diesel_schema::users::area_id.eq(area_id),
            diesel_schema::users::initials.eq(user.initials.value()),
            diesel_schema::users::name.eq(&user.name),
            diesel_schema::users::user_type.eq(user.user_type.as_str()),
            diesel_schema::users::crew.eq(user.crew.as_ref().map(|c| i32::from(c.number()))),
            diesel_schema::users::cumulative_natca_bu_date.eq(cumulative_natca_bu_date),
            diesel_schema::users::natca_bu_date.eq(natca_bu_date),
            diesel_schema::users::eod_faa_date.eq(eod_faa_date),
            diesel_schema::users::service_computation_date.eq(service_computation_date),
            diesel_schema::users::lottery_value
                .eq(user.seniority_data.lottery_value.and_then(|v| v.to_i32())),
        ))
        .execute(conn)?;

    // Retrieve the newly assigned user_id
    let user_id: i64 = diesel::select(diesel::dsl::sql::<diesel::sql_types::BigInt>(
        "last_insert_rowid()",
    ))
    .get_result(conn)?;

    debug!(
        bid_year_id,
        area_id,
        user_id,
        initials = user.initials.value(),
        "Inserted new user"
    );

    Ok(user_id)
}

/// Inserts a new user from the last user in the state (`MySQL` version).
///
/// This is used for incremental `RegisterUser` operations where only one user
/// is being added.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `state` - The state containing the new user (as the last element)
///
/// # Errors
///
/// Returns an error if the state has no users or if the database operation fails.
pub fn insert_new_user_mysql(
    conn: &mut MysqlConnection,
    state: &State,
) -> Result<i64, PersistenceError> {
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
    let bid_year_id: i64 = lookup_bid_year_id_mysql(conn, user.bid_year.year())?;
    let area_id: i64 = lookup_area_id_mysql(conn, bid_year_id, user.area.id())?;

    // Seniority data fields are already strings - just borrow them
    let cumulative_natca_bu_date: &str = &user.seniority_data.cumulative_natca_bu_date;
    let natca_bu_date: &str = &user.seniority_data.natca_bu_date;
    let eod_faa_date: &str = &user.seniority_data.eod_faa_date;
    let service_computation_date: &str = &user.seniority_data.service_computation_date;

    // Insert new user and let database assign user_id
    diesel::insert_into(diesel_schema::users::table)
        .values((
            diesel_schema::users::bid_year_id.eq(bid_year_id),
            diesel_schema::users::area_id.eq(area_id),
            diesel_schema::users::initials.eq(user.initials.value()),
            diesel_schema::users::name.eq(&user.name),
            diesel_schema::users::user_type.eq(user.user_type.as_str()),
            diesel_schema::users::crew.eq(user.crew.as_ref().map(|c| i32::from(c.number()))),
            diesel_schema::users::cumulative_natca_bu_date.eq(cumulative_natca_bu_date),
            diesel_schema::users::natca_bu_date.eq(natca_bu_date),
            diesel_schema::users::eod_faa_date.eq(eod_faa_date),
            diesel_schema::users::service_computation_date.eq(service_computation_date),
            diesel_schema::users::lottery_value
                .eq(user.seniority_data.lottery_value.and_then(|v| v.to_i32())),
        ))
        .execute(conn)?;

    // Retrieve the newly assigned user_id
    let user_id: i64 = diesel::select(diesel::dsl::sql::<diesel::sql_types::BigInt>(
        "LAST_INSERT_ID()",
    ))
    .get_result(conn)?;

    debug!(
        bid_year_id,
        area_id,
        user_id,
        initials = user.initials.value(),
        "Inserted new user"
    );

    Ok(user_id)
}

/// Syncs the canonical users table to match the given state (`SQLite` version).
///
/// This is an idempotent operation that replaces all users for the given
/// `(BidYear, Area)` with the users in the provided state.
///
/// Users with existing `user_id` values are inserted with their IDs.
/// Users without `user_id` are inserted and assigned new IDs by the database.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `state` - The state containing users to sync
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn sync_canonical_users_sqlite(
    conn: &mut SqliteConnection,
    state: &State,
) -> Result<(), PersistenceError> {
    // Look up the IDs
    let bid_year_id: i64 = lookup_bid_year_id_sqlite(conn, state.bid_year.year())?;
    let area_id: i64 = lookup_area_id_sqlite(conn, bid_year_id, state.area.id())?;

    // Delete all existing users for this (bid_year_id, area_id)
    diesel::delete(
        diesel_schema::users::table
            .filter(diesel_schema::users::bid_year_id.eq(bid_year_id))
            .filter(diesel_schema::users::area_id.eq(area_id)),
    )
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
            diesel::insert_into(diesel_schema::users::table)
                .values((
                    diesel_schema::users::user_id.eq(user_id),
                    diesel_schema::users::bid_year_id.eq(bid_year_id),
                    diesel_schema::users::area_id.eq(area_id),
                    diesel_schema::users::initials.eq(user.initials.value()),
                    diesel_schema::users::name.eq(&user.name),
                    diesel_schema::users::user_type.eq(user.user_type.as_str()),
                    diesel_schema::users::crew
                        .eq(user.crew.as_ref().map(|c| i32::from(c.number()))),
                    diesel_schema::users::cumulative_natca_bu_date.eq(cumulative_natca_bu_date),
                    diesel_schema::users::natca_bu_date.eq(natca_bu_date),
                    diesel_schema::users::eod_faa_date.eq(eod_faa_date),
                    diesel_schema::users::service_computation_date.eq(service_computation_date),
                    diesel_schema::users::lottery_value
                        .eq(user.seniority_data.lottery_value.and_then(|v| v.to_i32())),
                ))
                .execute(conn)?;
        } else {
            // User has no user_id, insert and let database assign one
            diesel::insert_into(diesel_schema::users::table)
                .values((
                    diesel_schema::users::bid_year_id.eq(bid_year_id),
                    diesel_schema::users::area_id.eq(area_id),
                    diesel_schema::users::initials.eq(user.initials.value()),
                    diesel_schema::users::name.eq(&user.name),
                    diesel_schema::users::user_type.eq(user.user_type.as_str()),
                    diesel_schema::users::crew
                        .eq(user.crew.as_ref().map(|c| i32::from(c.number()))),
                    diesel_schema::users::cumulative_natca_bu_date.eq(cumulative_natca_bu_date),
                    diesel_schema::users::natca_bu_date.eq(natca_bu_date),
                    diesel_schema::users::eod_faa_date.eq(eod_faa_date),
                    diesel_schema::users::service_computation_date.eq(service_computation_date),
                    diesel_schema::users::lottery_value
                        .eq(user.seniority_data.lottery_value.and_then(|v| v.to_i32())),
                ))
                .execute(conn)?;
        }
    }

    debug!(
        bid_year_id,
        area_id,
        user_count = state.users.len(),
        "Synced canonical users table"
    );

    Ok(())
}

/// Syncs the canonical users table to match the given state (`MySQL` version).
///
/// This is an idempotent operation that replaces all users for the given
/// `(BidYear, Area)` with the users in the provided state.
///
/// Users with existing `user_id` values are inserted with their IDs.
/// Users without `user_id` are inserted and assigned new IDs by the database.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `state` - The state containing users to sync
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn sync_canonical_users_mysql(
    conn: &mut MysqlConnection,
    state: &State,
) -> Result<(), PersistenceError> {
    // Look up the IDs
    let bid_year_id: i64 = lookup_bid_year_id_mysql(conn, state.bid_year.year())?;
    let area_id: i64 = lookup_area_id_mysql(conn, bid_year_id, state.area.id())?;

    // Delete all existing users for this (bid_year_id, area_id)
    diesel::delete(
        diesel_schema::users::table
            .filter(diesel_schema::users::bid_year_id.eq(bid_year_id))
            .filter(diesel_schema::users::area_id.eq(area_id)),
    )
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
            diesel::insert_into(diesel_schema::users::table)
                .values((
                    diesel_schema::users::user_id.eq(user_id),
                    diesel_schema::users::bid_year_id.eq(bid_year_id),
                    diesel_schema::users::area_id.eq(area_id),
                    diesel_schema::users::initials.eq(user.initials.value()),
                    diesel_schema::users::name.eq(&user.name),
                    diesel_schema::users::user_type.eq(user.user_type.as_str()),
                    diesel_schema::users::crew
                        .eq(user.crew.as_ref().map(|c| i32::from(c.number()))),
                    diesel_schema::users::cumulative_natca_bu_date.eq(cumulative_natca_bu_date),
                    diesel_schema::users::natca_bu_date.eq(natca_bu_date),
                    diesel_schema::users::eod_faa_date.eq(eod_faa_date),
                    diesel_schema::users::service_computation_date.eq(service_computation_date),
                    diesel_schema::users::lottery_value
                        .eq(user.seniority_data.lottery_value.and_then(|v| v.to_i32())),
                ))
                .execute(conn)?;
        } else {
            // User has no user_id, insert and let database assign one
            diesel::insert_into(diesel_schema::users::table)
                .values((
                    diesel_schema::users::bid_year_id.eq(bid_year_id),
                    diesel_schema::users::area_id.eq(area_id),
                    diesel_schema::users::initials.eq(user.initials.value()),
                    diesel_schema::users::name.eq(&user.name),
                    diesel_schema::users::user_type.eq(user.user_type.as_str()),
                    diesel_schema::users::crew
                        .eq(user.crew.as_ref().map(|c| i32::from(c.number()))),
                    diesel_schema::users::cumulative_natca_bu_date.eq(cumulative_natca_bu_date),
                    diesel_schema::users::natca_bu_date.eq(natca_bu_date),
                    diesel_schema::users::eod_faa_date.eq(eod_faa_date),
                    diesel_schema::users::service_computation_date.eq(service_computation_date),
                    diesel_schema::users::lottery_value
                        .eq(user.seniority_data.lottery_value.and_then(|v| v.to_i32())),
                ))
                .execute(conn)?;
        }
    }

    debug!(
        bid_year_id,
        area_id,
        user_count = state.users.len(),
        "Synced canonical users table"
    );

    Ok(())
}

backend_fn! {
/// Updates an existing user's information using `user_id` as the canonical identifier.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `user_id` - The user's canonical internal identifier
/// * `initials` - The user's initials (mutable field)
/// * `name` - The user's name
/// * `area` - The user's area
/// * `user_type` - The user's type classification
/// * `crew` - The user's crew (optional)
/// * `cumulative_natca_bu_date` - Cumulative NATCA bargaining unit date
/// * `natca_bu_date` - NATCA bargaining unit date
/// * `eod_faa_date` - Entry on Duty / FAA date
/// * `service_computation_date` - Service Computation Date
/// * `lottery_value` - Optional lottery value
///
/// # Errors
///
/// Returns an error if the database cannot be updated or the user does not exist.
#[allow(clippy::too_many_arguments)]
pub fn update_user(
    conn: &mut _,
    user_id: i64,
    initials: &Initials,
    name: &str,
    area: &Area,
    user_type: &str,
    crew: Option<u8>,
    cumulative_natca_bu_date: &str,
    natca_bu_date: &str,
    eod_faa_date: &str,
    service_computation_date: &str,
    lottery_value: Option<u32>,
) -> Result<(), PersistenceError> {
    let crew_i32: Option<i32> = crew.map(i32::from);
    let lottery_i32: Option<i32> = lottery_value.and_then(|v| i32::try_from(v).ok());

    // Area must have a canonical ID to update a user
    let area_id: i64 = area.area_id().ok_or_else(|| {
        PersistenceError::Other(format!(
            "Area '{}' has no canonical area_id",
            area.area_code()
        ))
    })?;

    let rows_affected: usize = diesel::update(diesel_schema::users::table)
        .filter(diesel_schema::users::user_id.eq(user_id))
        .set((
            diesel_schema::users::initials.eq(initials.value()),
            diesel_schema::users::name.eq(name),
            diesel_schema::users::area_id.eq(area_id),
            diesel_schema::users::user_type.eq(user_type),
            diesel_schema::users::crew.eq(crew_i32),
            diesel_schema::users::cumulative_natca_bu_date.eq(cumulative_natca_bu_date),
            diesel_schema::users::natca_bu_date.eq(natca_bu_date),
            diesel_schema::users::eod_faa_date.eq(eod_faa_date),
            diesel_schema::users::service_computation_date.eq(service_computation_date),
            diesel_schema::users::lottery_value.eq(lottery_i32),
        ))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::NotFound(format!(
            "User with user_id {user_id} not found"
        )));
    }

    debug!(user_id, "Updated user");

    Ok(())
}
}

/// Creates a system area (e.g., "No Bid") for a bid year (`SQLite` version).
///
/// Phase 25B: System areas are auto-created and cannot be deleted or renamed.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_code` - The area code (e.g., "NO BID")
///
/// # Returns
///
/// The generated `area_id` for the new system area.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn create_system_area_sqlite(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    area_code: &str,
) -> Result<i64, PersistenceError> {
    diesel::insert_into(diesel_schema::areas::table)
        .values((
            diesel_schema::areas::bid_year_id.eq(bid_year_id),
            diesel_schema::areas::area_code.eq(area_code),
            diesel_schema::areas::is_system_area.eq(1),
        ))
        .execute(conn)?;

    let area_id: i64 = diesel::select(diesel::dsl::sql::<diesel::sql_types::BigInt>(
        "last_insert_rowid()",
    ))
    .get_result(conn)?;

    debug!(area_id, bid_year_id, area_code, "Created system area");

    Ok(area_id)
}

/// Creates a system area (e.g., "No Bid") for a bid year (`MySQL` version).
///
/// Phase 25B: System areas are auto-created and cannot be deleted or renamed.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_code` - The area code (e.g., "NO BID")
///
/// # Returns
///
/// The generated `area_id` for the new system area.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn create_system_area_mysql(
    conn: &mut MysqlConnection,
    bid_year_id: i64,
    area_code: &str,
) -> Result<i64, PersistenceError> {
    diesel::insert_into(diesel_schema::areas::table)
        .values((
            diesel_schema::areas::bid_year_id.eq(bid_year_id),
            diesel_schema::areas::area_code.eq(area_code),
            diesel_schema::areas::is_system_area.eq(1),
        ))
        .execute(conn)?;

    let area_id: i64 = diesel::select(diesel::dsl::sql::<diesel::sql_types::BigInt>(
        "LAST_INSERT_ID()",
    ))
    .get_result(conn)?;

    debug!(area_id, bid_year_id, area_code, "Created system area");

    Ok(area_id)
}

/// Bulk inserts canonical area membership records (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `records` - The canonical area membership records to insert
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn bulk_insert_canonical_area_membership_sqlite(
    conn: &mut SqliteConnection,
    records: &[NewCanonicalAreaMembership],
) -> Result<(), PersistenceError> {
    diesel::insert_into(diesel_schema::canonical_area_membership::table)
        .values(records)
        .execute(conn)?;

    debug!(
        count = records.len(),
        "Bulk inserted canonical area membership"
    );
    Ok(())
}

/// Bulk inserts canonical area membership records (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `records` - The canonical area membership records to insert
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn bulk_insert_canonical_area_membership_mysql(
    conn: &mut MysqlConnection,
    records: &[NewCanonicalAreaMembership],
) -> Result<(), PersistenceError> {
    diesel::insert_into(diesel_schema::canonical_area_membership::table)
        .values(records)
        .execute(conn)?;

    debug!(
        count = records.len(),
        "Bulk inserted canonical area membership"
    );
    Ok(())
}

/// Bulk inserts canonical eligibility records (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `records` - The canonical eligibility records to insert
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn bulk_insert_canonical_eligibility_sqlite(
    conn: &mut SqliteConnection,
    records: &[NewCanonicalEligibility],
) -> Result<(), PersistenceError> {
    diesel::insert_into(diesel_schema::canonical_eligibility::table)
        .values(records)
        .execute(conn)?;

    debug!(count = records.len(), "Bulk inserted canonical eligibility");
    Ok(())
}

/// Bulk inserts canonical eligibility records (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `records` - The canonical eligibility records to insert
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn bulk_insert_canonical_eligibility_mysql(
    conn: &mut MysqlConnection,
    records: &[NewCanonicalEligibility],
) -> Result<(), PersistenceError> {
    diesel::insert_into(diesel_schema::canonical_eligibility::table)
        .values(records)
        .execute(conn)?;

    debug!(count = records.len(), "Bulk inserted canonical eligibility");
    Ok(())
}

/// Bulk inserts canonical bid order records (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `records` - The canonical bid order records to insert
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn bulk_insert_canonical_bid_order_sqlite(
    conn: &mut SqliteConnection,
    records: &[NewCanonicalBidOrder],
) -> Result<(), PersistenceError> {
    diesel::insert_into(diesel_schema::canonical_bid_order::table)
        .values(records)
        .execute(conn)?;

    debug!(count = records.len(), "Bulk inserted canonical bid order");
    Ok(())
}

/// Bulk inserts canonical bid order records (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `records` - The canonical bid order records to insert
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn bulk_insert_canonical_bid_order_mysql(
    conn: &mut MysqlConnection,
    records: &[NewCanonicalBidOrder],
) -> Result<(), PersistenceError> {
    diesel::insert_into(diesel_schema::canonical_bid_order::table)
        .values(records)
        .execute(conn)?;

    debug!(count = records.len(), "Bulk inserted canonical bid order");
    Ok(())
}

/// Bulk inserts canonical bid windows records (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `records` - The canonical bid windows records to insert
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn bulk_insert_canonical_bid_windows_sqlite(
    conn: &mut SqliteConnection,
    records: &[NewCanonicalBidWindows],
) -> Result<(), PersistenceError> {
    diesel::insert_into(diesel_schema::canonical_bid_windows::table)
        .values(records)
        .execute(conn)?;

    debug!(count = records.len(), "Bulk inserted canonical bid windows");
    Ok(())
}

/// Bulk inserts canonical bid windows records (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `records` - The canonical bid windows records to insert
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn bulk_insert_canonical_bid_windows_mysql(
    conn: &mut MysqlConnection,
    records: &[NewCanonicalBidWindows],
) -> Result<(), PersistenceError> {
    diesel::insert_into(diesel_schema::canonical_bid_windows::table)
        .values(records)
        .execute(conn)?;

    debug!(count = records.len(), "Bulk inserted canonical bid windows");
    Ok(())
}

/// Bulk inserts bid window records (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `records` - The bid window records to insert
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn bulk_insert_bid_windows_sqlite(
    conn: &mut SqliteConnection,
    records: &[crate::data_models::NewBidWindow],
) -> Result<(), PersistenceError> {
    diesel::insert_into(diesel_schema::bid_windows::table)
        .values(records)
        .execute(conn)?;

    debug!(count = records.len(), "Bulk inserted bid windows");
    Ok(())
}

/// Bulk inserts bid window records (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `records` - The bid window records to insert
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn bulk_insert_bid_windows_mysql(
    conn: &mut MysqlConnection,
    records: &[crate::data_models::NewBidWindow],
) -> Result<(), PersistenceError> {
    diesel::insert_into(diesel_schema::bid_windows::table)
        .values(records)
        .execute(conn)?;

    debug!(count = records.len(), "Bulk inserted bid windows");
    Ok(())
}

/// Override a user's area assignment (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `user_id` - The canonical user ID
/// * `new_area_id` - The new area ID to assign
/// * `reason` - The reason for the override
///
/// # Returns
///
/// Returns the previous `area_id` and whether the record was already overridden.
///
/// # Errors
///
/// Returns an error if the canonical record does not exist or the database operation fails.
pub fn override_area_assignment_sqlite(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    user_id: i64,
    new_area_id: i64,
    reason: &str,
) -> Result<(i64, bool), PersistenceError> {
    use crate::diesel_schema::canonical_area_membership;

    // First, fetch the current record
    let (previous_area_id, was_overridden): (i64, i32) = canonical_area_membership::table
        .filter(canonical_area_membership::bid_year_id.eq(bid_year_id))
        .filter(canonical_area_membership::user_id.eq(user_id))
        .select((
            canonical_area_membership::area_id,
            canonical_area_membership::is_overridden,
        ))
        .first::<(i64, i32)>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "Canonical area membership not found for user_id={user_id}, bid_year_id={bid_year_id}"
            ))
        })?;

    // Update the record
    diesel::update(
        canonical_area_membership::table
            .filter(canonical_area_membership::bid_year_id.eq(bid_year_id))
            .filter(canonical_area_membership::user_id.eq(user_id)),
    )
    .set((
        canonical_area_membership::area_id.eq(new_area_id),
        canonical_area_membership::is_overridden.eq(1),
        canonical_area_membership::override_reason.eq(reason),
    ))
    .execute(conn)?;

    debug!(
        bid_year_id,
        user_id, previous_area_id, new_area_id, "Overrode area assignment"
    );

    Ok((previous_area_id, was_overridden != 0))
}

/// Override a user's area assignment (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `user_id` - The canonical user ID
/// * `new_area_id` - The new area ID to assign
/// * `reason` - The reason for the override
///
/// # Returns
///
/// Returns the previous `area_id` and whether the record was already overridden.
///
/// # Errors
///
/// Returns an error if the canonical record does not exist or the database operation fails.
pub fn override_area_assignment_mysql(
    conn: &mut MysqlConnection,
    bid_year_id: i64,
    user_id: i64,
    new_area_id: i64,
    reason: &str,
) -> Result<(i64, bool), PersistenceError> {
    use crate::diesel_schema::canonical_area_membership;

    // First, fetch the current record
    let (previous_area_id, was_overridden): (i64, i32) = canonical_area_membership::table
        .filter(canonical_area_membership::bid_year_id.eq(bid_year_id))
        .filter(canonical_area_membership::user_id.eq(user_id))
        .select((
            canonical_area_membership::area_id,
            canonical_area_membership::is_overridden,
        ))
        .first::<(i64, i32)>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "Canonical area membership not found for user_id={user_id}, bid_year_id={bid_year_id}"
            ))
        })?;

    // Update the record
    diesel::update(
        canonical_area_membership::table
            .filter(canonical_area_membership::bid_year_id.eq(bid_year_id))
            .filter(canonical_area_membership::user_id.eq(user_id)),
    )
    .set((
        canonical_area_membership::area_id.eq(new_area_id),
        canonical_area_membership::is_overridden.eq(1),
        canonical_area_membership::override_reason.eq(reason),
    ))
    .execute(conn)?;

    debug!(
        bid_year_id,
        user_id, previous_area_id, new_area_id, "Overrode area assignment"
    );

    Ok((previous_area_id, was_overridden != 0))
}

/// Override a user's eligibility (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `user_id` - The canonical user ID
/// * `can_bid` - The new eligibility status
/// * `reason` - The reason for the override
///
/// # Returns
///
/// Returns the previous eligibility and whether the record was already overridden.
///
/// # Errors
///
/// Returns an error if the canonical record does not exist or the database operation fails.
pub fn override_eligibility_sqlite(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    user_id: i64,
    can_bid: bool,
    reason: &str,
) -> Result<(bool, bool), PersistenceError> {
    use crate::diesel_schema::canonical_eligibility;

    // First, fetch the current record
    let (previous_can_bid, was_overridden): (i32, i32) = canonical_eligibility::table
        .filter(canonical_eligibility::bid_year_id.eq(bid_year_id))
        .filter(canonical_eligibility::user_id.eq(user_id))
        .select((
            canonical_eligibility::can_bid,
            canonical_eligibility::is_overridden,
        ))
        .first::<(i32, i32)>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "Canonical eligibility not found for user_id={user_id}, bid_year_id={bid_year_id}"
            ))
        })?;

    // Update the record
    diesel::update(
        canonical_eligibility::table
            .filter(canonical_eligibility::bid_year_id.eq(bid_year_id))
            .filter(canonical_eligibility::user_id.eq(user_id)),
    )
    .set((
        canonical_eligibility::can_bid.eq(i32::from(can_bid)),
        canonical_eligibility::is_overridden.eq(1),
        canonical_eligibility::override_reason.eq(reason),
    ))
    .execute(conn)?;

    debug!(
        bid_year_id,
        user_id, previous_can_bid, can_bid, "Overrode eligibility"
    );

    Ok((previous_can_bid != 0, was_overridden != 0))
}

/// Override a user's eligibility (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `user_id` - The canonical user ID
/// * `can_bid` - The new eligibility status
/// * `reason` - The reason for the override
///
/// # Returns
///
/// Returns the previous eligibility and whether the record was already overridden.
///
/// # Errors
///
/// Returns an error if the canonical record does not exist or the database operation fails.
pub fn override_eligibility_mysql(
    conn: &mut MysqlConnection,
    bid_year_id: i64,
    user_id: i64,
    can_bid: bool,
    reason: &str,
) -> Result<(bool, bool), PersistenceError> {
    use crate::diesel_schema::canonical_eligibility;

    // First, fetch the current record
    let (previous_can_bid, was_overridden): (i32, i32) = canonical_eligibility::table
        .filter(canonical_eligibility::bid_year_id.eq(bid_year_id))
        .filter(canonical_eligibility::user_id.eq(user_id))
        .select((
            canonical_eligibility::can_bid,
            canonical_eligibility::is_overridden,
        ))
        .first::<(i32, i32)>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "Canonical eligibility not found for user_id={user_id}, bid_year_id={bid_year_id}"
            ))
        })?;

    // Update the record
    diesel::update(
        canonical_eligibility::table
            .filter(canonical_eligibility::bid_year_id.eq(bid_year_id))
            .filter(canonical_eligibility::user_id.eq(user_id)),
    )
    .set((
        canonical_eligibility::can_bid.eq(i32::from(can_bid)),
        canonical_eligibility::is_overridden.eq(1),
        canonical_eligibility::override_reason.eq(reason),
    ))
    .execute(conn)?;

    debug!(
        bid_year_id,
        user_id, previous_can_bid, can_bid, "Overrode eligibility"
    );

    Ok((previous_can_bid != 0, was_overridden != 0))
}

/// Override a user's bid order (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `user_id` - The canonical user ID
/// * `bid_order` - The new bid order (or `None` to clear)
/// * `reason` - The reason for the override
///
/// # Returns
///
/// Returns the previous `bid_order` and whether the record was already overridden.
///
/// # Errors
///
/// Returns an error if the canonical record does not exist or the database operation fails.
pub fn override_bid_order_sqlite(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    user_id: i64,
    bid_order: Option<i32>,
    reason: &str,
) -> Result<(Option<i32>, bool), PersistenceError> {
    use crate::diesel_schema::canonical_bid_order;

    // First, fetch the current record
    let (previous_bid_order, was_overridden): (Option<i32>, i32) = canonical_bid_order::table
        .filter(canonical_bid_order::bid_year_id.eq(bid_year_id))
        .filter(canonical_bid_order::user_id.eq(user_id))
        .select((
            canonical_bid_order::bid_order,
            canonical_bid_order::is_overridden,
        ))
        .first::<(Option<i32>, i32)>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "Canonical bid order not found for user_id={user_id}, bid_year_id={bid_year_id}"
            ))
        })?;

    // Update the record
    diesel::update(
        canonical_bid_order::table
            .filter(canonical_bid_order::bid_year_id.eq(bid_year_id))
            .filter(canonical_bid_order::user_id.eq(user_id)),
    )
    .set((
        canonical_bid_order::bid_order.eq(bid_order),
        canonical_bid_order::is_overridden.eq(1),
        canonical_bid_order::override_reason.eq(reason),
    ))
    .execute(conn)?;

    debug!(
        bid_year_id,
        user_id,
        ?previous_bid_order,
        ?bid_order,
        "Overrode bid order"
    );

    Ok((previous_bid_order, was_overridden != 0))
}

/// Override a user's bid order (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `user_id` - The canonical user ID
/// * `bid_order` - The new bid order (or `None` to clear)
/// * `reason` - The reason for the override
///
/// # Returns
///
/// Returns the previous `bid_order` and whether the record was already overridden.
///
/// # Errors
///
/// Returns an error if the canonical record does not exist or the database operation fails.
pub fn override_bid_order_mysql(
    conn: &mut MysqlConnection,
    bid_year_id: i64,
    user_id: i64,
    bid_order: Option<i32>,
    reason: &str,
) -> Result<(Option<i32>, bool), PersistenceError> {
    use crate::diesel_schema::canonical_bid_order;

    // First, fetch the current record
    let (previous_bid_order, was_overridden): (Option<i32>, i32) = canonical_bid_order::table
        .filter(canonical_bid_order::bid_year_id.eq(bid_year_id))
        .filter(canonical_bid_order::user_id.eq(user_id))
        .select((
            canonical_bid_order::bid_order,
            canonical_bid_order::is_overridden,
        ))
        .first::<(Option<i32>, i32)>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "Canonical bid order not found for user_id={user_id}, bid_year_id={bid_year_id}"
            ))
        })?;

    // Update the record
    diesel::update(
        canonical_bid_order::table
            .filter(canonical_bid_order::bid_year_id.eq(bid_year_id))
            .filter(canonical_bid_order::user_id.eq(user_id)),
    )
    .set((
        canonical_bid_order::bid_order.eq(bid_order),
        canonical_bid_order::is_overridden.eq(1),
        canonical_bid_order::override_reason.eq(reason),
    ))
    .execute(conn)?;

    debug!(
        bid_year_id,
        user_id,
        ?previous_bid_order,
        ?bid_order,
        "Overrode bid order"
    );

    Ok((previous_bid_order, was_overridden != 0))
}

/// Override a user's bid window (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `user_id` - The canonical user ID
/// * `window_start` - The new window start date (or `None` to clear)
/// * `window_end` - The new window end date (or `None` to clear)
/// * `reason` - The reason for the override
///
/// # Returns
///
/// Returns the previous window dates and whether the record was already overridden.
///
/// # Errors
///
/// Returns an error if the canonical record does not exist or the database operation fails.
pub fn override_bid_window_sqlite(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    user_id: i64,
    window_start: Option<&String>,
    window_end: Option<&String>,
    reason: &str,
) -> Result<(Option<String>, Option<String>, bool), PersistenceError> {
    use crate::diesel_schema::canonical_bid_windows;

    // First, fetch the current record
    let (previous_start, previous_end, was_overridden): (Option<String>, Option<String>, i32) =
        canonical_bid_windows::table
            .filter(canonical_bid_windows::bid_year_id.eq(bid_year_id))
            .filter(canonical_bid_windows::user_id.eq(user_id))
            .select((
                canonical_bid_windows::window_start_date,
                canonical_bid_windows::window_end_date,
                canonical_bid_windows::is_overridden,
            ))
            .first::<(Option<String>, Option<String>, i32)>(conn)
            .map_err(|_| {
                PersistenceError::ReconstructionError(format!(
                    "Canonical bid windows not found for user_id={user_id}, bid_year_id={bid_year_id}"
                ))
            })?;

    // Update the record
    diesel::update(
        canonical_bid_windows::table
            .filter(canonical_bid_windows::bid_year_id.eq(bid_year_id))
            .filter(canonical_bid_windows::user_id.eq(user_id)),
    )
    .set((
        canonical_bid_windows::window_start_date.eq(window_start.map(String::as_str)),
        canonical_bid_windows::window_end_date.eq(window_end.map(String::as_str)),
        canonical_bid_windows::is_overridden.eq(1),
        canonical_bid_windows::override_reason.eq(reason),
    ))
    .execute(conn)?;

    debug!(
        bid_year_id,
        user_id,
        ?previous_start,
        ?previous_end,
        ?window_start,
        ?window_end,
        "Overrode bid window"
    );

    Ok((previous_start, previous_end, was_overridden != 0))
}

/// Override a user's bid window (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `user_id` - The canonical user ID
/// * `window_start` - The new window start date (or `None` to clear)
/// * `window_end` - The new window end date (or `None` to clear)
/// * `reason` - The reason for the override
///
/// # Returns
///
/// Returns the previous window dates and whether the record was already overridden.
///
/// # Errors
///
/// Returns an error if the canonical record does not exist or the database operation fails.
pub fn override_bid_window_mysql(
    conn: &mut MysqlConnection,
    bid_year_id: i64,
    user_id: i64,
    window_start: Option<&String>,
    window_end: Option<&String>,
    reason: &str,
) -> Result<(Option<String>, Option<String>, bool), PersistenceError> {
    use crate::diesel_schema::canonical_bid_windows;

    // First, fetch the current record
    let (previous_start, previous_end, was_overridden): (Option<String>, Option<String>, i32) =
        canonical_bid_windows::table
            .filter(canonical_bid_windows::bid_year_id.eq(bid_year_id))
            .filter(canonical_bid_windows::user_id.eq(user_id))
            .select((
                canonical_bid_windows::window_start_date,
                canonical_bid_windows::window_end_date,
                canonical_bid_windows::is_overridden,
            ))
            .first::<(Option<String>, Option<String>, i32)>(conn)
            .map_err(|_| {
                PersistenceError::ReconstructionError(format!(
                    "Canonical bid windows not found for user_id={user_id}, bid_year_id={bid_year_id}"
                ))
            })?;

    // Update the record
    diesel::update(
        canonical_bid_windows::table
            .filter(canonical_bid_windows::bid_year_id.eq(bid_year_id))
            .filter(canonical_bid_windows::user_id.eq(user_id)),
    )
    .set((
        canonical_bid_windows::window_start_date.eq(window_start.map(String::as_str)),
        canonical_bid_windows::window_end_date.eq(window_end.map(String::as_str)),
        canonical_bid_windows::is_overridden.eq(1),
        canonical_bid_windows::override_reason.eq(reason),
    ))
    .execute(conn)?;

    debug!(
        bid_year_id,
        user_id,
        ?previous_start,
        ?previous_end,
        ?window_start,
        ?window_end,
        "Overrode bid window"
    );

    Ok((previous_start, previous_end, was_overridden != 0))
}

/// Updates an area's display name (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `area_id` - The canonical area identifier
/// * `area_name` - The new display name (or `None` to clear)
///
/// # Errors
///
/// Returns an error if the area doesn't exist or the database operation fails.
pub fn update_area_name_sqlite(
    conn: &mut SqliteConnection,
    area_id: i64,
    area_name: Option<&str>,
) -> Result<(), PersistenceError> {
    use crate::diesel_schema::areas;

    let rows_affected = diesel::update(areas::table.filter(areas::area_id.eq(area_id)))
        .set(areas::area_name.eq(area_name))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::ReconstructionError(format!(
            "Area with ID {area_id} not found"
        )));
    }

    debug!(area_id, ?area_name, "Updated area display name");

    Ok(())
}

/// Updates an area's display name (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `area_id` - The canonical area identifier
/// * `area_name` - The new display name (or `None` to clear)
///
/// # Errors
///
/// Returns an error if the area doesn't exist or the database operation fails.
pub fn update_area_name_mysql(
    conn: &mut MysqlConnection,
    area_id: i64,
    area_name: Option<&str>,
) -> Result<(), PersistenceError> {
    use crate::diesel_schema::areas;

    let rows_affected = diesel::update(areas::table.filter(areas::area_id.eq(area_id)))
        .set(areas::area_name.eq(area_name))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::ReconstructionError(format!(
            "Area with ID {area_id} not found"
        )));
    }

    debug!(area_id, ?area_name, "Updated area display name");

    Ok(())
}

/// Updates an area's assigned round group (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `area_id` - The canonical area identifier
/// * `round_group_id` - The round group ID to assign (or `None` to clear)
///
/// # Errors
///
/// Returns an error if the area doesn't exist or the database operation fails.
pub fn update_area_round_group_sqlite(
    conn: &mut SqliteConnection,
    area_id: i64,
    round_group_id: Option<i64>,
) -> Result<(), PersistenceError> {
    use crate::diesel_schema::areas;

    let rows_affected = diesel::update(areas::table.filter(areas::area_id.eq(area_id)))
        .set(areas::round_group_id.eq(round_group_id))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::ReconstructionError(format!(
            "Area with ID {area_id} not found"
        )));
    }

    debug!(
        area_id,
        ?round_group_id,
        "Updated area round group assignment"
    );

    Ok(())
}

/// Updates an area's assigned round group (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `area_id` - The canonical area identifier
/// * `round_group_id` - The round group ID to assign (or `None` to clear)
///
/// # Errors
///
/// Returns an error if the area doesn't exist or the database operation fails.
pub fn update_area_round_group_mysql(
    conn: &mut MysqlConnection,
    area_id: i64,
    round_group_id: Option<i64>,
) -> Result<(), PersistenceError> {
    use crate::diesel_schema::areas;

    let rows_affected = diesel::update(areas::table.filter(areas::area_id.eq(area_id)))
        .set(areas::round_group_id.eq(round_group_id))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::ReconstructionError(format!(
            "Area with ID {area_id} not found"
        )));
    }

    debug!(
        area_id,
        ?round_group_id,
        "Updated area round group assignment"
    );

    Ok(())
}

// ============================================================================
// Phase 29G: Post-Confirmation Bid Order Adjustments
// ============================================================================

/// Adjusts bid window for a specific user and round (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `user_id` - The canonical user ID
/// * `round_id` - The round ID
/// * `new_window_start` - The new window start datetime (ISO 8601)
/// * `new_window_end` - The new window end datetime (ISO 8601)
///
/// # Returns
///
/// Returns the previous window start and end datetimes.
///
/// # Errors
///
/// Returns an error if the bid window record does not exist or the database operation fails.
pub fn adjust_bid_window_sqlite(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    area_id: i64,
    user_id: i64,
    round_id: i64,
    new_window_start: &str,
    new_window_end: &str,
) -> Result<(String, String), PersistenceError> {
    use crate::diesel_schema::bid_windows;

    // First, fetch the current record
    let (previous_start, previous_end): (String, String) = bid_windows::table
        .filter(bid_windows::bid_year_id.eq(bid_year_id))
        .filter(bid_windows::area_id.eq(area_id))
        .filter(bid_windows::user_id.eq(user_id))
        .filter(bid_windows::round_id.eq(round_id))
        .select((
            bid_windows::window_start_datetime,
            bid_windows::window_end_datetime,
        ))
        .first::<(String, String)>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "Bid window not found for user_id={user_id}, round_id={round_id}"
            ))
        })?;

    // Update the record
    diesel::update(
        bid_windows::table
            .filter(bid_windows::bid_year_id.eq(bid_year_id))
            .filter(bid_windows::area_id.eq(area_id))
            .filter(bid_windows::user_id.eq(user_id))
            .filter(bid_windows::round_id.eq(round_id)),
    )
    .set((
        bid_windows::window_start_datetime.eq(new_window_start),
        bid_windows::window_end_datetime.eq(new_window_end),
    ))
    .execute(conn)?;

    debug!(
        bid_year_id,
        area_id,
        user_id,
        round_id,
        ?previous_start,
        ?previous_end,
        ?new_window_start,
        ?new_window_end,
        "Adjusted bid window"
    );

    Ok((previous_start, previous_end))
}

/// Adjusts bid window for a specific user and round (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `user_id` - The canonical user ID
/// * `round_id` - The round ID
/// * `new_window_start` - The new window start datetime (ISO 8601)
/// * `new_window_end` - The new window end datetime (ISO 8601)
///
/// # Returns
///
/// Returns the previous window start and end datetimes.
///
/// # Errors
///
/// Returns an error if the bid window record does not exist or the database operation fails.
pub fn adjust_bid_window_mysql(
    conn: &mut MysqlConnection,
    bid_year_id: i64,
    area_id: i64,
    user_id: i64,
    round_id: i64,
    new_window_start: &str,
    new_window_end: &str,
) -> Result<(String, String), PersistenceError> {
    use crate::diesel_schema::bid_windows;

    // First, fetch the current record
    let (previous_start, previous_end): (String, String) = bid_windows::table
        .filter(bid_windows::bid_year_id.eq(bid_year_id))
        .filter(bid_windows::area_id.eq(area_id))
        .filter(bid_windows::user_id.eq(user_id))
        .filter(bid_windows::round_id.eq(round_id))
        .select((
            bid_windows::window_start_datetime,
            bid_windows::window_end_datetime,
        ))
        .first::<(String, String)>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "Bid window not found for user_id={user_id}, round_id={round_id}"
            ))
        })?;

    // Update the record
    diesel::update(
        bid_windows::table
            .filter(bid_windows::bid_year_id.eq(bid_year_id))
            .filter(bid_windows::area_id.eq(area_id))
            .filter(bid_windows::user_id.eq(user_id))
            .filter(bid_windows::round_id.eq(round_id)),
    )
    .set((
        bid_windows::window_start_datetime.eq(new_window_start),
        bid_windows::window_end_datetime.eq(new_window_end),
    ))
    .execute(conn)?;

    debug!(
        bid_year_id,
        area_id,
        user_id,
        round_id,
        ?previous_start,
        ?previous_end,
        ?new_window_start,
        ?new_window_end,
        "Adjusted bid window"
    );

    Ok((previous_start, previous_end))
}

/// Deletes bid windows for specific users and rounds, used before recalculation (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `user_ids` - List of user IDs
/// * `round_ids` - List of round IDs
///
/// # Returns
///
/// Returns the number of deleted records.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn delete_bid_windows_for_users_and_rounds_sqlite(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    area_id: i64,
    user_ids: &[i64],
    round_ids: &[i64],
) -> Result<usize, PersistenceError> {
    use crate::diesel_schema::bid_windows;

    let deleted = diesel::delete(
        bid_windows::table
            .filter(bid_windows::bid_year_id.eq(bid_year_id))
            .filter(bid_windows::area_id.eq(area_id))
            .filter(bid_windows::user_id.eq_any(user_ids))
            .filter(bid_windows::round_id.eq_any(round_ids)),
    )
    .execute(conn)?;

    debug!(
        bid_year_id,
        area_id,
        user_count = user_ids.len(),
        round_count = round_ids.len(),
        deleted,
        "Deleted bid windows for recalculation"
    );

    Ok(deleted)
}

/// Deletes bid windows for specific users and rounds, used before recalculation (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `user_ids` - List of user IDs
/// * `round_ids` - List of round IDs
///
/// # Returns
///
/// Returns the number of deleted records.
///
/// # Errors
///
/// Returns an error if the database operation fails.
pub fn delete_bid_windows_for_users_and_rounds_mysql(
    conn: &mut MysqlConnection,
    bid_year_id: i64,
    area_id: i64,
    user_ids: &[i64],
    round_ids: &[i64],
) -> Result<usize, PersistenceError> {
    use crate::diesel_schema::bid_windows;

    let deleted = diesel::delete(
        bid_windows::table
            .filter(bid_windows::bid_year_id.eq(bid_year_id))
            .filter(bid_windows::area_id.eq(area_id))
            .filter(bid_windows::user_id.eq_any(user_ids))
            .filter(bid_windows::round_id.eq_any(round_ids)),
    )
    .execute(conn)?;

    debug!(
        bid_year_id,
        area_id,
        user_count = user_ids.len(),
        round_count = round_ids.len(),
        deleted,
        "Deleted bid windows for recalculation"
    );

    Ok(deleted)
}
