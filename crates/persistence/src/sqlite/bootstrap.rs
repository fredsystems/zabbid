// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use rusqlite::{Connection, params};
use time::Date;
use tracing::info;
use zab_bid::BootstrapMetadata;
use zab_bid_domain::{
    Area, BidYear, CanonicalBidYear, Crew, Initials, SeniorityData, User, UserType,
};

use crate::error::PersistenceError;

/// Verifies that foreign key enforcement is enabled.
///
/// This function checks whether `SQLite` has foreign key enforcement active.
/// If foreign keys are not enabled, the database cannot guarantee referential
/// integrity constraints required by Phase 14 (e.g., preventing deletion of
/// operators referenced by audit events).
///
/// # Arguments
///
/// * `conn` - The database connection to check
///
/// # Errors
///
/// Returns an error if foreign key enforcement is not enabled.
pub fn verify_foreign_key_enforcement(conn: &Connection) -> Result<(), PersistenceError> {
    let foreign_keys_enabled: i32 = conn.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;

    if foreign_keys_enabled == 0 {
        return Err(PersistenceError::InitializationError(String::from(
            "Foreign key enforcement is not enabled. The server cannot start without FK enforcement.",
        )));
    }

    info!("Foreign key enforcement is enabled");
    Ok(())
}

/// Reconstructs bootstrap metadata from canonical tables.
///
/// This method queries the canonical `bid_years` and `areas` tables to retrieve
/// the set of bid years and areas that have been created.
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
pub fn get_bootstrap_metadata(conn: &Connection) -> Result<BootstrapMetadata, PersistenceError> {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Query canonical bid_years table
    let mut stmt = conn.prepare("SELECT year FROM bid_years ORDER BY year ASC")?;
    let bid_year_rows = stmt.query_map([], |row| {
        let year_value: i32 = row.get(0)?;
        Ok(u16::try_from(year_value).expect("bid_year value out of u16 range"))
    })?;

    for row_result in bid_year_rows {
        let year: u16 = row_result?;
        metadata.bid_years.push(BidYear::new(year));
    }

    // Query canonical areas table
    let mut stmt =
        conn.prepare("SELECT bid_year, area_id FROM areas ORDER BY bid_year ASC, area_id ASC")?;
    let area_rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
    })?;

    for row_result in area_rows {
        let (bid_year_value, area_id) = row_result?;
        let bid_year: BidYear =
            BidYear::new(u16::try_from(bid_year_value).expect("bid_year value out of u16 range"));
        let area: Area = Area::new(&area_id);
        metadata.areas.push((bid_year, area));
    }

    Ok(metadata)
}

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
pub fn list_bid_years(conn: &Connection) -> Result<Vec<CanonicalBidYear>, PersistenceError> {
    let mut stmt =
        conn.prepare("SELECT year, start_date, num_pay_periods FROM bid_years ORDER BY year ASC")?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i32>(0)?,    // year
            row.get::<_, String>(1)?, // start_date
            row.get::<_, i32>(2)?,    // num_pay_periods
        ))
    })?;

    let mut bid_years: Vec<CanonicalBidYear> = Vec::new();
    for row_result in rows {
        let (year_value, start_date_str, num_pay_periods_value) = row_result?;

        let year: u16 = u16::try_from(year_value).expect("bid_year value out of u16 range");
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

        bid_years.push(canonical);
    }

    Ok(bid_years)
}

/// Lists all areas for a given bid year.
///
/// This queries the canonical `areas` table directly.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year` - The bid year to list areas for
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn list_areas(conn: &Connection, bid_year: &BidYear) -> Result<Vec<Area>, PersistenceError> {
    let mut stmt =
        conn.prepare("SELECT area_id FROM areas WHERE bid_year = ?1 ORDER BY area_id ASC")?;

    let rows = stmt.query_map(params![bid_year.year()], |row| {
        let area_id: String = row.get(0)?;
        Ok(area_id)
    })?;

    let mut areas: Vec<Area> = Vec::new();
    for row_result in rows {
        let area_id: String = row_result?;
        areas.push(Area::new(&area_id));
    }

    Ok(areas)
}

/// Lists all users for a given `(bid_year, area)` scope.
///
/// This queries the canonical `users` table directly.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year` - The bid year
/// * `area` - The area
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn list_users(
    conn: &Connection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<Vec<User>, PersistenceError> {
    let mut stmt = conn.prepare(
        "SELECT initials, name, user_type, crew,
                cumulative_natca_bu_date, natca_bu_date, eod_faa_date,
                service_computation_date, lottery_value
         FROM users
         WHERE bid_year = ?1 AND area_id = ?2
         ORDER BY initials ASC",
    )?;

    let rows = stmt.query_map(params![bid_year.year(), area.id()], |row| {
        Ok((
            row.get::<_, String>(0)?,      // initials
            row.get::<_, String>(1)?,      // name
            row.get::<_, String>(2)?,      // user_type
            row.get::<_, Option<i32>>(3)?, // crew
            row.get::<_, String>(4)?,      // cumulative_natca_bu_date
            row.get::<_, String>(5)?,      // natca_bu_date
            row.get::<_, String>(6)?,      // eod_faa_date
            row.get::<_, String>(7)?,      // service_computation_date
            row.get::<_, Option<i32>>(8)?, // lottery_value
        ))
    })?;

    let mut users: Vec<User> = Vec::new();
    for row_result in rows {
        let (
            initials_str,
            name,
            user_type_str,
            crew_num,
            cumulative_natca_bu_date,
            natca_bu_date,
            eod_faa_date,
            service_computation_date,
            lottery_value,
        ) = row_result?;

        let initials: Initials = Initials::new(&initials_str);
        let user_type: UserType = UserType::parse(&user_type_str)
            .map_err(|e| PersistenceError::ReconstructionError(e.to_string()))?;
        let crew: Option<Crew> =
            crew_num.and_then(|n| u8::try_from(n).ok().and_then(|num| Crew::new(num).ok()));
        let seniority_data: SeniorityData = SeniorityData::new(
            cumulative_natca_bu_date,
            natca_bu_date,
            eod_faa_date,
            service_computation_date,
            lottery_value.and_then(|v| u32::try_from(v).ok()),
        );

        let user: User = User::new(
            bid_year.clone(),
            initials,
            name,
            area.clone(),
            user_type,
            crew,
            seniority_data,
        );
        users.push(user);
    }

    Ok(users)
}

/// Sets a bid year as active, ensuring only one bid year is active at a time.
///
/// This method atomically updates the active status:
/// 1. Clears the active flag from all bid years
/// 2. Sets the active flag on the specified bid year
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `year` - The year to mark as active
///
/// # Errors
///
/// Returns an error if:
/// - The database cannot be updated
/// - The bid year does not exist
pub fn set_active_bid_year(conn: &Connection, year: u16) -> Result<(), PersistenceError> {
    // First, clear all active flags
    conn.execute("UPDATE bid_years SET is_active = 0", [])?;

    // Then set the specified year as active
    let rows_affected: usize = conn.execute(
        "UPDATE bid_years SET is_active = 1 WHERE year = ?1",
        params![year],
    )?;

    if rows_affected == 0 {
        return Err(PersistenceError::NotFound(format!(
            "Bid year {year} not found"
        )));
    }

    Ok(())
}

/// Gets the currently active bid year, if any.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_active_bid_year(conn: &Connection) -> Result<Option<u16>, PersistenceError> {
    let result = conn.query_row(
        "SELECT year FROM bid_years WHERE is_active = 1",
        [],
        |row| {
            let year_value: i32 = row.get(0)?;
            Ok(u16::try_from(year_value).expect("bid_year value out of u16 range"))
        },
    );

    match result {
        Ok(year) => Ok(Some(year)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Sets the expected area count for a bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `year` - The bid year
/// * `expected_count` - The expected number of areas
///
/// # Errors
///
/// Returns an error if:
/// - The database cannot be updated
/// - The bid year does not exist
pub fn set_expected_area_count(
    conn: &Connection,
    year: u16,
    expected_count: u32,
) -> Result<(), PersistenceError> {
    let rows_affected: usize = conn.execute(
        "UPDATE bid_years SET expected_area_count = ?1 WHERE year = ?2",
        params![expected_count, year],
    )?;

    if rows_affected == 0 {
        return Err(PersistenceError::NotFound(format!(
            "Bid year {year} not found"
        )));
    }

    Ok(())
}

/// Sets the expected user count for an area.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year` - The bid year
/// * `area` - The area
/// * `expected_count` - The expected number of users
///
/// # Errors
///
/// Returns an error if:
/// - The database cannot be updated
/// - The area does not exist
pub fn set_expected_user_count(
    conn: &Connection,
    bid_year: &BidYear,
    area: &Area,
    expected_count: u32,
) -> Result<(), PersistenceError> {
    let rows_affected: usize = conn.execute(
        "UPDATE areas SET expected_user_count = ?1 WHERE bid_year = ?2 AND area_id = ?3",
        params![expected_count, bid_year.year(), area.id()],
    )?;

    if rows_affected == 0 {
        return Err(PersistenceError::NotFound(format!(
            "Area '{}' in bid year {} not found",
            area.id(),
            bid_year.year()
        )));
    }

    Ok(())
}

/// Gets the expected area count for a bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `year` - The bid year
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_expected_area_count(
    conn: &Connection,
    year: u16,
) -> Result<Option<u32>, PersistenceError> {
    let result = conn.query_row(
        "SELECT expected_area_count FROM bid_years WHERE year = ?1",
        params![year],
        |row| {
            let count: Option<i32> = row.get(0)?;
            Ok(count.and_then(|c| u32::try_from(c).ok()))
        },
    );

    match result {
        Ok(count) => Ok(count),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Gets the expected user count for an area.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year` - The bid year
/// * `area` - The area
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_expected_user_count(
    conn: &Connection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<Option<u32>, PersistenceError> {
    let result = conn.query_row(
        "SELECT expected_user_count FROM areas WHERE bid_year = ?1 AND area_id = ?2",
        params![bid_year.year(), area.id()],
        |row| {
            let count: Option<i32> = row.get(0)?;
            Ok(count.and_then(|c| u32::try_from(c).ok()))
        },
    );

    match result {
        Ok(count) => Ok(count),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Gets the actual area count for a bid year.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `year` - The bid year
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_actual_area_count(conn: &Connection, year: u16) -> Result<usize, PersistenceError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM areas WHERE bid_year = ?1",
        params![year],
        |row| row.get(0),
    )?;

    Ok(usize::try_from(count).expect("count out of usize range"))
}

/// Gets the actual user count for an area.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year` - The bid year
/// * `area` - The area
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn get_actual_user_count(
    conn: &Connection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<usize, PersistenceError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM users WHERE bid_year = ?1 AND area_id = ?2",
        params![bid_year.year(), area.id()],
        |row| row.get(0),
    )?;

    Ok(usize::try_from(count).expect("count out of usize range"))
}
