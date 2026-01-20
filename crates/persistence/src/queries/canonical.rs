// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Canonical entity queries.
//!
//! This module contains queries for retrieving canonical
//! bid years, areas, and users from their respective tables.
//!
//! All queries are generated in backend-specific monomorphic versions
//! (`_sqlite` and `_mysql` suffixes) using the `backend_fn!` macro.

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use num_traits::ToPrimitive;
use time::Date;
use zab_bid::BootstrapMetadata;
use zab_bid_domain::{
    Area, BidYear, CanonicalBidYear, Crew, Initials, SeniorityData, User, UserType,
};

use crate::diesel_schema::{areas, bid_years, users};
use crate::error::PersistenceError;

backend_fn! {
/// Looks up the canonical `bid_year_id` from the year value.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `year` - The year value
///
/// # Errors
///
/// Returns an error if the bid year does not exist.
pub fn lookup_bid_year_id(conn: &mut _, year: u16) -> Result<i64, PersistenceError> {
    let year_i32: i32 = year
        .to_i32()
        .ok_or_else(|| PersistenceError::Other("Year out of range".to_string()))?;

    let result = bid_years::table
        .select(bid_years::bid_year_id)
        .filter(bid_years::year.eq(year_i32))
        .first::<i64>(conn);

    match result {
        Ok(id) => Ok(id),
        Err(diesel::result::Error::NotFound) => Err(PersistenceError::ReconstructionError(
            format!("Bid year {year} does not exist"),
        )),
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
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
    conn: &mut _,
    bid_year_id: i64,
    area_code: &str,
) -> Result<i64, PersistenceError> {
    let result = areas::table
        .select(areas::area_id)
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::area_code.eq(area_code))
        .first::<i64>(conn);

    match result {
        Ok(id) => Ok(id),
        Err(diesel::result::Error::NotFound) => Err(PersistenceError::ReconstructionError(
            format!("Area {area_code} in bid year ID {bid_year_id} does not exist"),
        )),
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
/// Reconstructs bootstrap metadata from canonical tables.
///
/// This method queries the canonical `bid_years` and `areas` tables to retrieve
/// the set of bid years and areas that have been created.
///
/// Phase 23A: Now retrieves and populates canonical IDs.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
///
/// # Panics
///
/// Panics if a bid year value from the database is outside the valid `u16` range.
/// This should not occur in normal operation as bid years are validated on creation.
pub fn get_bootstrap_metadata(conn: &mut _) -> Result<BootstrapMetadata, PersistenceError> {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Query canonical bid_years table
    let bid_year_rows: Vec<(i64, i32)> = bid_years::table
        .select((bid_years::bid_year_id, bid_years::year))
        .order(bid_years::year.asc())
        .load::<(i64, i32)>(conn)?;

    for (bid_year_id, year_value) in bid_year_rows {
        let year: u16 = u16::try_from(year_value).map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "bid_year value out of u16 range: {year_value}"
            ))
        })?;
        metadata.bid_years.push(BidYear::with_id(bid_year_id, year));
    }

    // Query canonical areas table
    #[allow(clippy::type_complexity)]
    let area_rows = areas::table
        .inner_join(bid_years::table)
        .select((
            areas::area_id,
            areas::bid_year_id,
            bid_years::year,
            areas::area_code,
            areas::area_name,
            areas::is_system_area,
            areas::round_group_id,
        ))
        .order((bid_years::year.asc(), areas::area_code.asc()))
        .load::<(i64, i64, i32, String, Option<String>, i32, Option<i64>)>(conn)?;

    for (area_id, bid_year_id_val, year_value, code, name, is_sys, rg_id) in area_rows {
        let year: u16 = u16::try_from(year_value).map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "bid_year value out of u16 range: {year_value}"
            ))
        })?;
        let bid_year: BidYear = BidYear::with_id(bid_year_id_val, year);
        let area: Area = Area::with_id(area_id, &code, name, is_sys != 0, rg_id);
        metadata.areas.push((bid_year, area));
    }

    Ok(metadata)
}
}

backend_fn! {
/// Lists all bid years that have been created with their canonical metadata.
///
/// This queries the canonical `bid_years` table directly and returns full
/// canonical bid year definitions including start date and pay period count.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if the data cannot
/// be reconstructed into valid `CanonicalBidYear` instances.
///
/// # Panics
///
/// Panics if a bid year value from the database cannot be converted to `u16`.
/// This should never happen in practice as the schema enforces valid ranges.
pub fn list_bid_years(conn: &mut _) -> Result<Vec<CanonicalBidYear>, PersistenceError> {
    let rows: Vec<(i32, String, i32)> = bid_years::table
        .select((
            bid_years::year,
            bid_years::start_date,
            bid_years::num_pay_periods,
        ))
        .order(bid_years::year.asc())
        .load::<(i32, String, i32)>(conn)?;

    let mut bid_years_list: Vec<CanonicalBidYear> = Vec::new();
    for (year_value, start_date_str, num_pay_periods_value) in rows {
        let year: u16 = u16::try_from(year_value).map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "bid_year value out of u16 range: {year_value}"
            ))
        })?;
        let num_pay_periods: u8 = u8::try_from(num_pay_periods_value).map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "Invalid num_pay_periods value: {num_pay_periods_value}"
            ))
        })?;

        let start_date: Date = Date::parse(
            &start_date_str,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .map_err(|e| {
            PersistenceError::ReconstructionError(format!(
                "Failed to parse start_date '{start_date_str}': {e}"
            ))
        })?;

        let canonical: CanonicalBidYear = CanonicalBidYear::new(year, start_date, num_pay_periods)
            .map_err(|e| {
                PersistenceError::ReconstructionError(format!(
                    "Failed to construct CanonicalBidYear: {e}"
                ))
            })?;

        bid_years_list.push(canonical);
    }

    Ok(bid_years_list)
}
}

backend_fn! {
/// Lists all areas for a given bid year.
///
/// This queries the canonical `areas` table directly.
///
/// Phase 23A: Now constructs Area objects with their IDs.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn list_areas(conn: &mut _, bid_year_id: i64) -> Result<Vec<Area>, PersistenceError> {
    #[allow(clippy::type_complexity)]
    let rows = areas::table
        .select((areas::area_id, areas::area_code, areas::area_name, areas::is_system_area, areas::round_group_id))
        .filter(areas::bid_year_id.eq(bid_year_id))
        .order(areas::area_code.asc())
        .load::<(i64, String, Option<String>, i32, Option<i64>)>(conn)?;

    let areas_list: Vec<Area> = rows
        .into_iter()
        .map(|(area_id, code, name, is_sys, rg_id)| Area::with_id(area_id, &code, name, is_sys != 0, rg_id))
        .collect();

    Ok(areas_list)
}
}

backend_fn! {
/// Gets a single area by its canonical ID, returning both the Area and its `bid_year_id`.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `area_id` - The canonical area ID
///
/// # Errors
///
/// Returns an error if the area does not exist or the database cannot be queried.
pub fn get_area_by_id(conn: &mut _, area_id: i64) -> Result<(Area, i64), PersistenceError> {
    #[allow(clippy::type_complexity)]
    let (aid, bid_year_id, code, name, is_sys, rg_id) = areas::table
        .select((areas::area_id, areas::bid_year_id, areas::area_code, areas::area_name, areas::is_system_area, areas::round_group_id))
        .filter(areas::area_id.eq(area_id))
        .first::<(i64, i64, String, Option<String>, i32, Option<i64>)>(conn)?;

    // Area doesn't have a bid_year field - it's tracked in the database
    // Return both the area and its bid_year_id for round validation
    let area = Area::with_id(aid, &code, name, is_sys != 0, rg_id);

    Ok((area, bid_year_id))
}
}

backend_fn! {
/// Lists all users for a given `(bid_year, area)` scope.
///
/// This queries the canonical `users` table directly.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `bid_year` - The bid year (for constructing User objects)
/// * `area` - The area (for constructing User objects)
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn list_users(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
    bid_year: &BidYear,
    area: &Area,
) -> Result<Vec<User>, PersistenceError> {
    type UserRowTuple = (
        i64,
        String,
        String,
        String,
        Option<i32>,
        String,
        String,
        String,
        String,
        Option<i32>,
        i32,
        i32,
    );

    let rows: Vec<UserRowTuple> = users::table
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
        ))
        .filter(users::bid_year_id.eq(bid_year_id))
        .filter(users::area_id.eq(area_id))
        .order(users::initials.asc())
        .load(conn)?;

    let mut users_list: Vec<User> = Vec::new();
    for (
        user_id,
        initials_str,
        name,
        user_type_str,
        crew_val,
        cumulative_natca_bu_date,
        natca_bu_date,
        eod_faa_date,
        service_computation_date,
        lottery_value,
        excluded_from_bidding,
        excluded_from_leave_calculation,
    ) in rows
    {
        let initials: Initials = Initials::new(&initials_str);
        let user_type: UserType = UserType::parse(&user_type_str)
            .map_err(|e| PersistenceError::ReconstructionError(e.to_string()))?;
        let crew: Option<Crew> =
            crew_val.and_then(|n| u8::try_from(n).ok().and_then(|num| Crew::new(num).ok()));
        let seniority_data: SeniorityData = SeniorityData::new(
            cumulative_natca_bu_date,
            natca_bu_date,
            eod_faa_date,
            service_computation_date,
            lottery_value.and_then(|v| u32::try_from(v).ok()),
        );

        let user: User = User::with_id(
            user_id,
            bid_year.clone(),
            initials,
            name,
            area.clone(),
            user_type,
            crew,
            seniority_data,
            excluded_from_bidding != 0,
            excluded_from_leave_calculation != 0,
        );
        users_list.push(user);
    }

    Ok(users_list)
}
}

backend_fn! {
/// Gets the active bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or no active bid year exists.
pub fn get_active_bid_year(conn: &mut _) -> Result<u16, PersistenceError> {
    let result: Result<i32, _> = bid_years::table
        .select(bid_years::year)
        .filter(bid_years::is_active.eq(1))
        .first::<i32>(conn);

    match result {
        Ok(year_i32) => {
            let year: u16 = year_i32
                .to_u16()
                .ok_or_else(|| PersistenceError::Other("Year out of range".to_string()))?;
            Ok(year)
        }
        Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(String::from(
            "No active bid year",
        ))),
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
/// Gets the expected area count for a bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Errors
///
/// Returns an error if the database cannot be queried or the bid year doesn't exist.
pub fn get_expected_area_count(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<Option<usize>, PersistenceError> {
    let result: Result<Option<i32>, _> = bid_years::table
        .select(bid_years::expected_area_count)
        .filter(bid_years::bid_year_id.eq(bid_year_id))
        .first::<Option<i32>>(conn);

    match result {
        Ok(Some(count_i32)) => {
            let count: usize = count_i32.to_usize().ok_or_else(|| {
                PersistenceError::DatabaseError("Count conversion failed".to_string())
            })?;
            Ok(Some(count))
        }
        Ok(None) => Ok(None),
        Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(String::from(
            "Bid year not found",
        ))),
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
/// Gets the expected user count for a bid year and area.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
///
/// # Errors
///
/// Returns an error if the database cannot be queried or the area doesn't exist.
pub fn get_expected_user_count(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
) -> Result<Option<usize>, PersistenceError> {
    let result: Result<Option<i32>, _> = areas::table
        .select(areas::expected_user_count)
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::area_id.eq(area_id))
        .first::<Option<i32>>(conn);

    match result {
        Ok(Some(count_i32)) => {
            let count: usize = count_i32.to_usize().ok_or_else(|| {
                PersistenceError::DatabaseError("Count conversion failed".to_string())
            })?;
            Ok(Some(count))
        }
        Ok(None) => Ok(None),
        Err(diesel::result::Error::NotFound) => {
            Err(PersistenceError::NotFound(String::from("Area not found")))
        }
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
/// Gets the actual area count for a bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_actual_area_count(conn: &mut _, bid_year_id: i64) -> Result<usize, PersistenceError> {
    let count: i64 = areas::table
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::is_system_area.eq(0))
        .count()
        .get_result(conn)?;

    count
        .to_usize()
        .ok_or_else(|| PersistenceError::DatabaseError("Count conversion failed".to_string()))
}
}

backend_fn! {
/// Gets the lifecycle state for a bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Errors
///
/// Returns an error if the bid year doesn't exist or the database cannot be queried.
pub fn get_lifecycle_state(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<String, PersistenceError> {
    let result: Result<String, _> = bid_years::table
        .select(bid_years::lifecycle_state)
        .filter(bid_years::bid_year_id.eq(bid_year_id))
        .first::<String>(conn);

    match result {
        Ok(state) => Ok(state),
        Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(format!(
            "Bid year with ID {bid_year_id} not found"
        ))),
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
/// Updates the lifecycle state for a bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `new_state` - The new lifecycle state as a string
///
/// # Errors
///
/// Returns an error if the bid year doesn't exist or the database cannot be updated.
pub fn update_lifecycle_state(
    conn: &mut _,
    bid_year_id: i64,
    new_state: &str,
) -> Result<(), PersistenceError> {
    use diesel::prelude::*;

    let rows_affected = diesel::update(bid_years::table)
        .filter(bid_years::bid_year_id.eq(bid_year_id))
        .set(bid_years::lifecycle_state.eq(new_state))
        .execute(conn)?;

    if rows_affected == 0 {
        Err(PersistenceError::NotFound(format!(
            "Bid year with ID {bid_year_id} not found"
        )))
    } else {
        Ok(())
    }
}
}

backend_fn! {
/// Retrieves the metadata (label and notes) for a bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// A tuple of `(Option<String>, Option<String>)` for `(label, notes)`.
///
/// # Errors
///
/// Returns an error if the bid year doesn't exist or the database cannot be queried.
pub fn get_bid_year_metadata(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<(Option<String>, Option<String>), PersistenceError> {
    use diesel::prelude::*;

    let result: Result<(Option<String>, Option<String>), _> = bid_years::table
        .select((bid_years::label, bid_years::notes))
        .filter(bid_years::bid_year_id.eq(bid_year_id))
        .first::<(Option<String>, Option<String>)>(conn);

    match result {
        Ok(metadata) => Ok(metadata),
        Err(diesel::result::Error::NotFound) => Err(PersistenceError::NotFound(format!(
            "Bid year with ID {bid_year_id} not found"
        ))),
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
/// Queries whether any bid year is in the `BiddingActive` lifecycle state.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Returns
///
/// * `Ok(Some(year))` if a bid year is `BiddingActive`
/// * `Ok(None)` if no bid year is `BiddingActive`
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_bidding_active_year(conn: &mut _) -> Result<Option<u16>, PersistenceError> {
    let result: Result<i32, _> = bid_years::table
        .select(bid_years::year)
        .filter(bid_years::lifecycle_state.eq("BiddingActive"))
        .first::<i32>(conn);

    match result {
        Ok(year_i32) => {
            let year: u16 = year_i32
                .to_u16()
                .ok_or_else(|| PersistenceError::Other("Year out of range".to_string()))?;
            Ok(Some(year))
        }
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
/// Gets the actual user count for a bid year and area.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_actual_user_count(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
) -> Result<usize, PersistenceError> {
    let count: i64 = users::table
        .filter(users::bid_year_id.eq(bid_year_id))
        .filter(users::area_id.eq(area_id))
        .count()
        .get_result(conn)?;

    count
        .to_usize()
        .ok_or_else(|| PersistenceError::DatabaseError("Count conversion failed".to_string()))
}
}

backend_fn! {
/// Finds the system area (No Bid) for a given bid year.
///
/// Phase 25B: Returns the area ID and area code of the system area.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// * `Ok(Some((area_id, area_code)))` if a system area exists
/// * `Ok(None)` if no system area exists
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn find_system_area(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<Option<(i64, String)>, PersistenceError> {
    let result: Option<(i64, String)> = areas::table
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::is_system_area.eq(1))
        .select((areas::area_id, areas::area_code))
        .first(conn)
        .optional()?;

    Ok(result)
}
}

backend_fn! {
/// Counts users in the system area (No Bid) for a given bid year.
///
/// Phase 25B: Used to check if bootstrap can be completed.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// The number of users in the No Bid area (0 if no system area exists).
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn count_users_in_system_area(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<usize, PersistenceError> {
    // First find the system area ID
    let system_area_id: Option<i64> = areas::table
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::is_system_area.eq(1))
        .select(areas::area_id)
        .first(conn)
        .optional()?;

    if let Some(sys_area_id) = system_area_id {
        let count: i64 = users::table
            .filter(users::bid_year_id.eq(bid_year_id))
            .filter(users::area_id.eq(sys_area_id))
            .count()
            .get_result(conn)?;

        count
            .to_usize()
            .ok_or_else(|| PersistenceError::DatabaseError("Count conversion failed".to_string()))
    } else {
        Ok(0)
    }
}
}

backend_fn! {
/// Lists users in the system area (No Bid) for a given bid year.
///
/// Phase 25B: Returns up to `limit` user initials for error reporting.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `limit` - Maximum number of initials to return
///
/// # Returns
///
/// A vector of user initials (empty if no system area or no users).
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn list_users_in_system_area(
    conn: &mut _,
    bid_year_id: i64,
    limit: i64,
) -> Result<Vec<String>, PersistenceError> {
    // First find the system area ID
    let system_area_id: Option<i64> = areas::table
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::is_system_area.eq(1))
        .select(areas::area_id)
        .first(conn)
        .optional()?;

    if let Some(sys_area_id) = system_area_id {
        let initials: Vec<String> = users::table
            .filter(users::bid_year_id.eq(bid_year_id))
            .filter(users::area_id.eq(sys_area_id))
            .select(users::initials)
            .limit(limit)
            .load(conn)?;

        Ok(initials)
    } else {
        Ok(Vec::new())
    }
}
}

backend_fn! {
/// Checks if an area is a system area.
///
/// Phase 25B: Used to prevent deletion/renaming of system areas.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `area_id` - The canonical area ID to check
///
/// # Returns
///
/// `true` if the area is a system area, `false` otherwise.
///
/// # Errors
///
/// Returns an error if the database cannot be queried or the area doesn't exist.
pub fn is_system_area(
    conn: &mut _,
    area_id: i64,
) -> Result<bool, PersistenceError> {
    let system_flag: i32 = areas::table
        .filter(areas::area_id.eq(area_id))
        .select(areas::is_system_area)
        .first(conn)?;

    Ok(system_flag != 0)
}
}

backend_fn! {
/// Checks if canonical rows exist for a bid year.
///
/// Phase 25C: Used for idempotency checks during canonicalization.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// `true` if canonical area membership rows exist for this bid year, `false` otherwise.
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn canonical_rows_exist(
    conn: &mut _,
    bid_year_id: i64,
) -> Result<bool, PersistenceError> {
    use crate::diesel_schema::canonical_area_membership;

    let count: i64 = canonical_area_membership::table
        .filter(canonical_area_membership::bid_year_id.eq(bid_year_id))
        .count()
        .get_result(conn)?;

    Ok(count > 0)
}
}

backend_fn! {
/// Lists users from canonical tables (post-canonicalization).
///
/// Phase 25C: Used when `lifecycle_state` >= `Canonicalized`.
/// Reads from `canonical_area_membership`, `canonical_eligibility`, `canonical_bid_order`,
/// and `canonical_bid_windows`.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
/// * `bid_year` - The `BidYear` domain object
/// * `area` - The `Area` domain object
///
/// # Errors
///
/// Returns an error if the database cannot be queried or canonical data is missing.
pub fn list_users_canonical(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
    bid_year: &BidYear,
    area: &Area,
) -> Result<Vec<User>, PersistenceError> {
    use crate::diesel_schema::{
        canonical_area_membership, canonical_eligibility, users,
    };

    // Query users joined with canonical area membership
    type UserRowTuple = (
        i64,
        String,
        String,
        String,
        Option<i32>,
        String,
        String,
        String,
        String,
        Option<i32>,
        i32, // can_bid from canonical_eligibility
        i32, // excluded_from_bidding
        i32, // excluded_from_leave_calculation
    );

    let rows: Vec<UserRowTuple> = users::table
        .inner_join(
            canonical_area_membership::table.on(
                users::user_id.eq(canonical_area_membership::user_id)
                    .and(canonical_area_membership::bid_year_id.eq(bid_year_id))
                    .and(canonical_area_membership::area_id.eq(area_id))
            )
        )
        .inner_join(
            canonical_eligibility::table.on(
                users::user_id.eq(canonical_eligibility::user_id)
                    .and(canonical_eligibility::bid_year_id.eq(bid_year_id))
            )
        )
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
            canonical_eligibility::can_bid,
            users::excluded_from_bidding,
            users::excluded_from_leave_calculation,
        ))
        .filter(users::bid_year_id.eq(bid_year_id))
        .order(users::initials.asc())
        .load(conn)?;

    let mut users_list: Vec<User> = Vec::new();
    for (
        user_id,
        initials_str,
        name,
        user_type_str,
        crew_val,
        cumulative_natca_bu_date,
        natca_bu_date,
        eod_faa_date,
        service_computation_date,
        lottery_value,
        _can_bid,
        excluded_from_bidding,
        excluded_from_leave_calculation,
    ) in rows
    {
        let initials: Initials = Initials::new(&initials_str);
        let user_type: UserType = UserType::parse(&user_type_str)
            .map_err(|e| PersistenceError::ReconstructionError(e.to_string()))?;
        let crew: Option<Crew> =
            crew_val.and_then(|n| u8::try_from(n).ok().and_then(|num| Crew::new(num).ok()));
        let seniority_data: SeniorityData = SeniorityData::new(
            cumulative_natca_bu_date,
            natca_bu_date,
            eod_faa_date,
            service_computation_date,
            lottery_value.and_then(|v| u32::try_from(v).ok()),
        );

        let user: User = User::with_id(
            user_id,
            bid_year.clone(),
            initials,
            name,
            area.clone(),
            user_type,
            crew,
            seniority_data,
            excluded_from_bidding != 0,
            excluded_from_leave_calculation != 0,
        );
        users_list.push(user);
    }

    Ok(users_list)
}
}

/// Lists users with lifecycle-aware routing (`SQLite` version).
///
/// Phase 25C: Routes to canonical or derived tables based on lifecycle state.
pub fn list_users_with_routing_sqlite(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
    area_id: i64,
    bid_year: &BidYear,
    area: &Area,
) -> Result<Vec<User>, PersistenceError> {
    use crate::diesel_schema::bid_years;

    // Get lifecycle state
    let lifecycle_state: String = bid_years::table
        .select(bid_years::lifecycle_state)
        .filter(bid_years::bid_year_id.eq(bid_year_id))
        .first::<String>(conn)?;

    // Parse lifecycle state to determine routing
    let requires_canonical: bool = matches!(
        lifecycle_state.as_str(),
        "Canonicalized" | "BiddingActive" | "BiddingClosed"
    );

    if requires_canonical {
        // Verify canonical data exists
        if !canonical_rows_exist_sqlite(conn, bid_year_id)? {
            return Err(PersistenceError::CanonicalDataMissing {
                bid_year_id,
                table: String::from("canonical_area_membership"),
            });
        }
        list_users_canonical_sqlite(conn, bid_year_id, area_id, bid_year, area)
    } else {
        list_users_sqlite(conn, bid_year_id, area_id, bid_year, area)
    }
}

/// Lists users with lifecycle-aware routing (`MySQL` version).
///
/// Phase 25C: Routes to canonical or derived tables based on lifecycle state.
pub fn list_users_with_routing_mysql(
    conn: &mut MysqlConnection,
    bid_year_id: i64,
    area_id: i64,
    bid_year: &BidYear,
    area: &Area,
) -> Result<Vec<User>, PersistenceError> {
    use crate::diesel_schema::bid_years;

    // Get lifecycle state
    let lifecycle_state: String = bid_years::table
        .select(bid_years::lifecycle_state)
        .filter(bid_years::bid_year_id.eq(bid_year_id))
        .first::<String>(conn)?;

    // Parse lifecycle state to determine routing
    let requires_canonical: bool = matches!(
        lifecycle_state.as_str(),
        "Canonicalized" | "BiddingActive" | "BiddingClosed"
    );

    if requires_canonical {
        // Verify canonical data exists
        if !canonical_rows_exist_mysql(conn, bid_year_id)? {
            return Err(PersistenceError::CanonicalDataMissing {
                bid_year_id,
                table: String::from("canonical_area_membership"),
            });
        }
        list_users_canonical_mysql(conn, bid_year_id, area_id, bid_year, area)
    } else {
        list_users_mysql(conn, bid_year_id, area_id, bid_year, area)
    }
}

backend_fn! {
/// Get user details for override operations.
///
/// # Arguments
///
/// * `user_id` - The canonical user ID
///
/// # Returns
///
/// Returns a tuple of (`bid_year_id`, `user_initials`).
///
/// # Errors
///
/// Returns an error if the user does not exist or the database operation fails.
pub fn get_user_details_for_override(
    conn: &mut _,
    user_id: i64,
) -> Result<(i64, String), PersistenceError> {
    users::table
        .filter(users::user_id.eq(user_id))
        .select((users::bid_year_id, users::initials))
        .first::<(i64, String)>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!("User {user_id} not found"))
        })
}
}

backend_fn! {
/// Get area details for override operations.
///
/// # Arguments
///
/// * `area_id` - The canonical area ID
///
/// # Returns
///
/// Returns a tuple of (`area_code`, `area_name`).
///
/// # Errors
///
/// Returns an error if the area does not exist or the database operation fails.
pub fn get_area_details_for_override(
    conn: &mut _,
    area_id: i64,
) -> Result<(String, Option<String>), PersistenceError> {
    areas::table
        .filter(areas::area_id.eq(area_id))
        .select((areas::area_code, areas::area_name))
        .first::<(String, Option<String>)>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!("Area {area_id} not found"))
        })
}
}

backend_fn! {
/// Get the area ID for a user.
///
/// # Arguments
///
/// * `user_id` - The canonical user ID
///
/// # Returns
///
/// Returns the `area_id` where the user is currently assigned.
///
/// # Errors
///
/// Returns an error if the user does not exist or the database operation fails.
pub fn get_user_area_id(
    conn: &mut _,
    user_id: i64,
) -> Result<i64, PersistenceError> {
    users::table
        .filter(users::user_id.eq(user_id))
        .select(users::area_id)
        .first::<i64>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!("User {user_id} not found"))
        })
}
}

backend_fn! {
/// Get current canonical area assignment for a user.
///
/// # Arguments
///
/// * `bid_year_id` - The canonical bid year ID
/// * `user_id` - The canonical user ID
///
/// # Returns
///
/// Returns the current `area_id`.
///
/// # Errors
///
/// Returns an error if the canonical record does not exist or the database operation fails.
pub fn get_current_area_assignment_for_override(
    conn: &mut _,
    bid_year_id: i64,
    user_id: i64,
) -> Result<i64, PersistenceError> {
    use crate::diesel_schema::canonical_area_membership;

    canonical_area_membership::table
        .filter(canonical_area_membership::bid_year_id.eq(bid_year_id))
        .filter(canonical_area_membership::user_id.eq(user_id))
        .select(canonical_area_membership::area_id)
        .first::<i64>(conn)
        .map_err(|_| {
            PersistenceError::ReconstructionError(format!(
                "Canonical area membership not found for user_id={user_id}, bid_year_id={bid_year_id}"
            ))
        })
}
}
