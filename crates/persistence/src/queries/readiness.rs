// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Phase 29D: Readiness evaluation persistence queries
//!
//! This module provides database queries to support bid year readiness evaluation.

#![allow(dead_code)] // Phase 29D: Functions will be used by API layer

use crate::diesel_schema::{areas, bid_years, users};
use crate::error::PersistenceError;
use diesel::prelude::*;

backend_fn! {
/// Checks if a bid year has a valid bid schedule configured.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// `true` if all bid schedule fields are set, `false` otherwise.
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn is_bid_schedule_set(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<bool, PersistenceError> {
    type BidScheduleRow = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i32>,
    );

    let result: Option<BidScheduleRow> = bid_years::table
        .find(bid_year_id)
        .select((
            bid_years::bid_timezone,
            bid_years::bid_start_date,
            bid_years::bid_window_start_time,
            bid_years::bid_window_end_time,
            bid_years::bidders_per_area_per_day,
        ))
        .first(conn)
        .optional()?;

    match result {
        Some((tz, start, window_start, window_end, bidders)) => {
            Ok(tz.is_some()
                && start.is_some()
                && window_start.is_some()
                && window_end.is_some()
                && bidders.is_some())
        }
        None => Err(PersistenceError::NotFound(format!(
            "Bid year {bid_year_id} not found"
        ))),
    }
}
}

backend_fn! {
/// Gets non-system areas that have no rounds configured.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// Vector of area codes for areas missing round configuration.
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_areas_missing_rounds(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<Vec<String>, PersistenceError> {
    let area_codes: Vec<String> = areas::table
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::is_system_area.eq(0))
        .filter(areas::round_group_id.is_null())
        .select(areas::area_code)
        .load(conn)?;

    Ok(area_codes)
}
}

backend_fn! {
/// Counts users in system areas who have not been reviewed.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// Count of unreviewed users in system areas (No Bid).
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn count_unreviewed_no_bid_users(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<i64, PersistenceError> {
    let count: i64 = users::table
        .inner_join(areas::table.on(users::area_id.eq(areas::area_id)))
        .filter(users::bid_year_id.eq(bid_year_id))
        .filter(areas::is_system_area.eq(1))
        .filter(users::no_bid_reviewed.eq(0))
        .count()
        .get_result(conn)?;

    Ok(count)
}
}

backend_fn! {
/// Counts users violating the participation flag directional invariant.
///
/// Invariant: `excluded_from_leave_calculation == true` ⇒ `excluded_from_bidding == true`
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// Count of users violating the invariant.
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn count_participation_flag_violations(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<i64, PersistenceError> {
    let count: i64 = users::table
        .filter(users::bid_year_id.eq(bid_year_id))
        .filter(users::excluded_from_leave_calculation.eq(1))
        .filter(users::excluded_from_bidding.eq(0))
        .count()
        .get_result(conn)?;

    Ok(count)
}
}

backend_fn! {
/// Marks a user in a system area as reviewed.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `user_id` - The user's canonical ID
///
/// # Errors
///
/// Returns an error if the database cannot be updated.
pub fn mark_user_no_bid_reviewed(
    conn: &mut _,
    user_id: i64,
) -> Result<(), PersistenceError> {
    diesel::update(users::table.find(user_id))
        .set(users::no_bid_reviewed.eq(1))
        .execute(conn)?;

    Ok(())
}
}

backend_fn! {
/// Gets all users grouped by area for bid order conflict detection.
///
/// Returns users in non-system areas only, for seniority conflict checking.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// Vector of tuples containing (`area_id`, `area_code`, users in that area).
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_users_by_area_for_conflict_detection(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<Vec<(i64, String, Vec<zab_bid_domain::User>)>, PersistenceError> {
    use crate::diesel_schema::{areas, users};
    use num_traits::cast::ToPrimitive;
    use zab_bid_domain::{Area, BidYear, Crew, Initials, SeniorityData, User, UserType};

    // Get all non-system areas
    let areas_list: Vec<(i64, String)> = areas::table
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::is_system_area.eq(0))
        .select((areas::area_id, areas::area_code))
        .load(conn)?;

    let mut result = Vec::new();

    for (area_id, area_code) in areas_list {
        // Get users for this area
        type UserRow = (
            i64,    // user_id
            String, // initials
            String, // name
            String, // user_type
            Option<i32>, // crew_number
            String, // cumulative_natca_bu_date
            String, // natca_bu_date
            String, // eod_faa_date
            String, // service_computation_date
            Option<i32>, // lottery_value
            i32,    // excluded_from_bidding
            i32,    // excluded_from_leave_calculation
            i32,    // no_bid_reviewed
        );

        let user_rows: Vec<UserRow> = users::table
            .filter(users::bid_year_id.eq(bid_year_id))
            .filter(users::area_id.eq(area_id))
            .select((
                users::user_id,
                users::initials,
                users::name,
                users::user_type,
                users::crew,
                users::cumulative_natca_bu_date,
                users::natca_bu_date,
                users::eod_faa_date,
                users::service_computation_date,
                users::lottery_value,
                users::excluded_from_bidding,
                users::excluded_from_leave_calculation,
                users::no_bid_reviewed,
            ))
            .load(conn)?;

        let mut users_for_area = Vec::new();

        for (
            user_id,
            initials,
            name,
            user_type_str,
            crew,
            cumulative_natca,
            natca_bu,
            eod_faa,
            scd,
            lottery,
            excluded_bidding,
            excluded_leave,
            reviewed,
        ) in user_rows
        {
            let user_type = UserType::parse(&user_type_str)
                .map_err(|e| PersistenceError::Other(format!("Invalid user type: {e}")))?;

            let crew_opt = crew
                .map(|n| {
                    n.to_u8().ok_or_else(|| {
                        PersistenceError::Other(format!("Crew number {n} out of range for u8"))
                    })
                })
                .transpose()?
                .map(Crew::new)
                .transpose()
                .map_err(|e| PersistenceError::Other(format!("Invalid crew: {e}")))?;

            let seniority_data = SeniorityData::new(
                cumulative_natca,
                natca_bu,
                eod_faa,
                scd,
                lottery.map(i32::cast_unsigned),
            );

            let user = User::with_id(
                user_id,
                BidYear::new(2026), // Placeholder - not used for conflict detection
                Initials::new(&initials),
                name,
                Area::new(&area_code),
                user_type,
                crew_opt,
                seniority_data,
                excluded_bidding != 0,
                excluded_leave != 0,
                reviewed != 0,
            );

            users_for_area.push(user);
        }

        result.push((area_id, area_code, users_for_area));
    }

    Ok(result)
}
}
