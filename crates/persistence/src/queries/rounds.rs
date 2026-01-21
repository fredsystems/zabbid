// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Round and round group queries.
//!
//! This module contains queries for retrieving and managing round groups
//! and rounds from their respective tables.
//!
//! All queries are generated in backend-specific monomorphic versions
//! (`_sqlite` and `_mysql` suffixes) using the `backend_fn!` macro.

#![allow(dead_code)]

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use num_traits::cast::ToPrimitive;
use zab_bid_domain::{BidYear, Round, RoundGroup};

use crate::diesel_schema::{round_groups, rounds};
use crate::error::PersistenceError;

backend_fn! {
/// Lists all round groups for a given bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The bid year ID
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn list_round_groups(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<Vec<RoundGroup>, PersistenceError> {
    let rows = round_groups::table
        .filter(round_groups::bid_year_id.eq(bid_year_id))
        .select((
            round_groups::round_group_id,
            round_groups::bid_year_id,
            round_groups::name,
            round_groups::editing_enabled,
        ))
        .load::<(i64, i64, String, i32)>(conn)?;

    // For now, we construct a minimal BidYear. In a real implementation,
    // we would join with bid_years table to get the full year value.
    // For Phase 29B, we'll accept this limitation since round groups
    // will primarily be queried in contexts where the bid year is known.
    rows.into_iter()
        .map(|(round_group_id, _bid_year_id, name, editing_enabled)| {
            // TODO: This is a placeholder. In a complete implementation,
            // we would join with bid_years to get the actual year value.
            // For now, we construct with a dummy year that will need to be
            // replaced by the caller if needed.
            let bid_year = BidYear::with_id(bid_year_id, 0);
            Ok(RoundGroup::with_id(
                round_group_id,
                bid_year,
                name,
                editing_enabled != 0,
            ))
        })
        .collect()
}
}

backend_fn! {
/// Gets a single round group by ID.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `round_group_id` - The round group ID
///
/// # Errors
///
/// Returns an error if the round group does not exist or the query fails.
pub fn get_round_group(
    conn: &mut _,
    round_group_id: i64,
) -> Result<RoundGroup, PersistenceError> {
    let (rg_id, bid_year_id, name, editing_enabled) = round_groups::table
        .filter(round_groups::round_group_id.eq(round_group_id))
        .select((
            round_groups::round_group_id,
            round_groups::bid_year_id,
            round_groups::name,
            round_groups::editing_enabled,
        ))
        .first::<(i64, i64, String, i32)>(conn)?;

    let bid_year = BidYear::with_id(bid_year_id, 0);
    Ok(RoundGroup::with_id(
        rg_id,
        bid_year,
        name,
        editing_enabled != 0,
    ))
}
}

backend_fn! {
/// Inserts a new round group.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The bid year ID
/// * `name` - The round group name
/// * `editing_enabled` - Whether editing is enabled
///
/// # Errors
///
/// Returns an error if the insert fails (e.g., duplicate name).
pub fn insert_round_group(
    conn: &mut _,
    bid_year_id: i64,
    name: &str,
    editing_enabled: bool,
) -> Result<i64, PersistenceError> {
    diesel::insert_into(round_groups::table)
        .values((
            round_groups::bid_year_id.eq(bid_year_id),
            round_groups::name.eq(name),
            round_groups::editing_enabled.eq(i32::from(editing_enabled)),
        ))
        .execute(conn)?;

    // Get the last inserted ID
    let round_group_id = diesel::select(diesel::dsl::sql::<diesel::sql_types::BigInt>(
        "last_insert_rowid()",
    ))
    .get_result::<i64>(conn)?;

    Ok(round_group_id)
}
}

backend_fn! {
/// Updates an existing round group.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `round_group_id` - The round group ID
/// * `name` - The new name
/// * `editing_enabled` - The new `editing_enabled` value
///
/// # Errors
///
/// Returns an error if the update fails.
pub fn update_round_group(
    conn: &mut _,
    round_group_id: i64,
    name: &str,
    editing_enabled: bool,
) -> Result<(), PersistenceError> {
    diesel::update(round_groups::table.filter(round_groups::round_group_id.eq(round_group_id)))
        .set((
            round_groups::name.eq(name),
            round_groups::editing_enabled.eq(i32::from(editing_enabled)),
        ))
        .execute(conn)?;

    Ok(())
}
}

backend_fn! {
/// Deletes a round group.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `round_group_id` - The round group ID
///
/// # Errors
///
/// Returns an error if the delete fails (e.g., foreign key constraint).
pub fn delete_round_group(
    conn: &mut _,
    round_group_id: i64,
) -> Result<(), PersistenceError> {
    diesel::delete(round_groups::table.filter(round_groups::round_group_id.eq(round_group_id)))
        .execute(conn)?;
    Ok(())
}
}

backend_fn! {
/// Checks if a round group is referenced by any rounds.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `round_group_id` - The round group ID
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn count_rounds_using_group(
    conn: &mut _,
    round_group_id: i64,
) -> Result<usize, PersistenceError> {
    let count = rounds::table
        .filter(rounds::round_group_id.eq(round_group_id))
        .count()
        .get_result::<i64>(conn)?;

    Ok(count.to_usize().unwrap_or(0))
}
}

backend_fn! {
/// Checks if a round group name exists within a bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The bid year ID
/// * `name` - The round group name
/// * `exclude_id` - Optional round group ID to exclude from the check
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn round_group_name_exists(
    conn: &mut _,
    bid_year_id: i64,
    name: &str,
    exclude_id: Option<i64>,
) -> Result<bool, PersistenceError> {
    let mut query = round_groups::table
        .filter(round_groups::bid_year_id.eq(bid_year_id))
        .filter(round_groups::name.eq(name))
        .into_boxed();

    if let Some(id) = exclude_id {
        query = query.filter(round_groups::round_group_id.ne(id));
    }

    let count = query.count().get_result::<i64>(conn)?;
    Ok(count > 0)
}
}

backend_fn! {
/// Lists all rounds for a given round group.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `round_group_id` - The round group ID
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn list_rounds(
    conn: &mut _,
    round_group_id: i64,
) -> Result<Vec<Round>, PersistenceError> {
    // This is a simplified implementation. A full implementation would
    // join with round_groups to reconstruct complete domain objects.
    // For Phase 29B, we accept this limitation.
    let rows = rounds::table
        .filter(rounds::round_group_id.eq(round_group_id))
        .select((
            rounds::round_id,
            rounds::round_group_id,
            rounds::round_number,
            rounds::name,
            rounds::slots_per_day,
            rounds::max_groups,
            rounds::max_total_hours,
            rounds::include_holidays,
            rounds::allow_overbid,
        ))
        .load::<(i64, i64, i32, String, i32, i32, i32, i32, i32)>(conn)?;

    // Placeholder: construct minimal domain objects
    // In production, we would join tables to get full RoundGroup objects
    rows.into_iter()
        .map(
            |(
                round_id,
                rg_id,
                round_number,
                name,
                slots_per_day,
                max_groups,
                max_total_hours,
                include_holidays,
                allow_overbid,
            )| {
                let bid_year = BidYear::with_id(0, 0);
                let round_group = RoundGroup::with_id(rg_id, bid_year, String::new(), true);

                Ok(Round::with_id(
                    round_id,
                    round_group,
                    round_number.to_u32().unwrap_or(0),
                    name,
                    slots_per_day.to_u32().unwrap_or(0),
                    max_groups.to_u32().unwrap_or(0),
                    max_total_hours.to_u32().unwrap_or(0),
                    include_holidays != 0,
                    allow_overbid != 0,
                ))
            },
        )
        .collect()
}
}

backend_fn! {
/// Gets a single round by ID.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `round_id` - The round ID
///
/// # Errors
///
/// Returns an error if the round does not exist or the query fails.
pub fn get_round(
    conn: &mut _,
    round_id: i64,
) -> Result<Round, PersistenceError> {
    let (
        r_id,
        round_group_id,
        round_number,
        name,
        slots_per_day,
        max_groups,
        max_total_hours,
        include_holidays,
        allow_overbid,
    ) = rounds::table
        .filter(rounds::round_id.eq(round_id))
        .select((
            rounds::round_id,
            rounds::round_group_id,
            rounds::round_number,
            rounds::name,
            rounds::slots_per_day,
            rounds::max_groups,
            rounds::max_total_hours,
            rounds::include_holidays,
            rounds::allow_overbid,
        ))
        .first::<(i64, i64, i32, String, i32, i32, i32, i32, i32)>(conn)?;

    let bid_year = BidYear::with_id(0, 0);
    let round_group = RoundGroup::with_id(round_group_id, bid_year, String::new(), true);

    Ok(Round::with_id(
        r_id,
        round_group,
        round_number.to_u32().unwrap_or(0),
        name,
        slots_per_day.to_u32().unwrap_or(0),
        max_groups.to_u32().unwrap_or(0),
        max_total_hours.to_u32().unwrap_or(0),
        include_holidays != 0,
        allow_overbid != 0,
    ))
}
}

backend_fn! {
/// Inserts a new round.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `round_group_id` - The round group ID
/// * `round_number` - The round number
/// * `name` - The round name
/// * `slots_per_day` - Slots per day
/// * `max_groups` - Maximum groups
/// * `max_total_hours` - Maximum total hours
/// * `include_holidays` - Whether holidays are included
/// * `allow_overbid` - Whether overbidding is allowed
///
/// # Errors
///
/// Returns an error if the insert fails (e.g., duplicate round number).
#[allow(clippy::too_many_arguments)]
pub fn insert_round(
    conn: &mut _,
    round_group_id: i64,
    round_number: u32,
    name: &str,
    slots_per_day: u32,
    max_groups: u32,
    max_total_hours: u32,
    include_holidays: bool,
    allow_overbid: bool,
) -> Result<i64, PersistenceError> {
    diesel::insert_into(rounds::table)
        .values((
            rounds::round_group_id.eq(round_group_id),
            rounds::round_number.eq(round_number.to_i32().unwrap_or(0)),
            rounds::name.eq(name),
            rounds::slots_per_day.eq(slots_per_day.to_i32().unwrap_or(0)),
            rounds::max_groups.eq(max_groups.to_i32().unwrap_or(0)),
            rounds::max_total_hours.eq(max_total_hours.to_i32().unwrap_or(0)),
            rounds::include_holidays.eq(i32::from(include_holidays)),
            rounds::allow_overbid.eq(i32::from(allow_overbid)),
        ))
        .execute(conn)?;

    let round_id = diesel::select(diesel::dsl::sql::<diesel::sql_types::BigInt>(
        "last_insert_rowid()",
    ))
    .get_result::<i64>(conn)?;

    Ok(round_id)
}
}

backend_fn! {
/// Updates an existing round.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `round_id` - The round ID
/// * `name` - The new name
/// * `slots_per_day` - The new `slots_per_day`
/// * `max_groups` - The new `max_groups`
/// * `max_total_hours` - The new `max_total_hours`
/// * `include_holidays` - The new `include_holidays`
/// * `allow_overbid` - The new `allow_overbid`
///
/// # Errors
///
/// Returns an error if the update fails.
#[allow(clippy::too_many_arguments)]
pub fn update_round(
    conn: &mut _,
    round_id: i64,
    name: &str,
    slots_per_day: u32,
    max_groups: u32,
    max_total_hours: u32,
    include_holidays: bool,
    allow_overbid: bool,
) -> Result<(), PersistenceError> {
    diesel::update(rounds::table.filter(rounds::round_id.eq(round_id)))
        .set((
            rounds::name.eq(name),
            rounds::slots_per_day.eq(slots_per_day.to_i32().unwrap_or(0)),
            rounds::max_groups.eq(max_groups.to_i32().unwrap_or(0)),
            rounds::max_total_hours.eq(max_total_hours.to_i32().unwrap_or(0)),
            rounds::include_holidays.eq(i32::from(include_holidays)),
            rounds::allow_overbid.eq(i32::from(allow_overbid)),
        ))
        .execute(conn)?;

    Ok(())
}
}

backend_fn! {
/// Deletes a round.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `round_id` - The round ID
///
/// # Errors
///
/// Returns an error if the delete fails.
pub fn delete_round(
    conn: &mut _,
    round_id: i64,
) -> Result<(), PersistenceError> {
    diesel::delete(rounds::table.filter(rounds::round_id.eq(round_id)))
        .execute(conn)?;
    Ok(())
}
}

backend_fn! {
/// Checks if a round number exists within a round group.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `round_group_id` - The round group ID
/// * `round_number` - The round number
/// * `exclude_id` - Optional round ID to exclude from the check
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn round_number_exists(
    conn: &mut _,
    round_group_id: i64,
    round_number: u32,
    exclude_id: Option<i64>,
) -> Result<bool, PersistenceError> {
    let mut query = rounds::table
        .filter(rounds::round_group_id.eq(round_group_id))
        .filter(rounds::round_number.eq(round_number.to_i32().unwrap_or(0)))
        .into_boxed();

    if let Some(id) = exclude_id {
        query = query.filter(rounds::round_id.ne(id));
    }

    let count = query.count().get_result::<i64>(conn)?;
    Ok(count > 0)
}
}

backend_fn! {
/// Lists all rounds for a given bid year (across all round groups).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The bid year ID
///
/// # Errors
///
/// Returns an error if the query fails.
pub fn list_all_rounds_for_bid_year(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<Vec<(i64, String)>, PersistenceError> {
    let rows = rounds::table
        .inner_join(round_groups::table)
        .filter(round_groups::bid_year_id.eq(bid_year_id))
        .select((
            rounds::round_id,
            rounds::name,
        ))
        .load::<(i64, String)>(conn)?;

    Ok(rows)
}
}
