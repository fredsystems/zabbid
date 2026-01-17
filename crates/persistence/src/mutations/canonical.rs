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
) -> Result<(), PersistenceError> {
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
            diesel_schema::users::crew.eq(user
                .crew
                .as_ref()
                .map(|c| c.number().to_i32().expect("Crew number out of range"))),
            diesel_schema::users::cumulative_natca_bu_date.eq(cumulative_natca_bu_date),
            diesel_schema::users::natca_bu_date.eq(natca_bu_date),
            diesel_schema::users::eod_faa_date.eq(eod_faa_date),
            diesel_schema::users::service_computation_date.eq(service_computation_date),
            diesel_schema::users::lottery_value
                .eq(user.seniority_data.lottery_value.and_then(|v| v.to_i32())),
        ))
        .execute(conn)?;

    debug!(
        bid_year_id,
        area_id,
        initials = user.initials.value(),
        "Inserted new user"
    );

    Ok(())
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
) -> Result<(), PersistenceError> {
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
            diesel_schema::users::crew.eq(user
                .crew
                .as_ref()
                .map(|c| c.number().to_i32().expect("Crew number out of range"))),
            diesel_schema::users::cumulative_natca_bu_date.eq(cumulative_natca_bu_date),
            diesel_schema::users::natca_bu_date.eq(natca_bu_date),
            diesel_schema::users::eod_faa_date.eq(eod_faa_date),
            diesel_schema::users::service_computation_date.eq(service_computation_date),
            diesel_schema::users::lottery_value
                .eq(user.seniority_data.lottery_value.and_then(|v| v.to_i32())),
        ))
        .execute(conn)?;

    debug!(
        bid_year_id,
        area_id,
        initials = user.initials.value(),
        "Inserted new user"
    );

    Ok(())
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
                    diesel_schema::users::crew.eq(user
                        .crew
                        .as_ref()
                        .map(|c| c.number().to_i32().expect("Crew number out of range"))),
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
                    diesel_schema::users::crew.eq(user
                        .crew
                        .as_ref()
                        .map(|c| c.number().to_i32().expect("Crew number out of range"))),
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
                    diesel_schema::users::crew.eq(user
                        .crew
                        .as_ref()
                        .map(|c| c.number().to_i32().expect("Crew number out of range"))),
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
                    diesel_schema::users::crew.eq(user
                        .crew
                        .as_ref()
                        .map(|c| c.number().to_i32().expect("Crew number out of range"))),
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
