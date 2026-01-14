// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use diesel::prelude::*;
use diesel::sql_query;
use diesel::sqlite::SqliteConnection;
use time::Date;
use tracing::info;
use zab_bid::BootstrapMetadata;
use zab_bid_domain::{
    Area, BidYear, CanonicalBidYear, Crew, Initials, SeniorityData, User, UserType,
};

use crate::error::PersistenceError;

// Helper row structs for Diesel queries
#[derive(diesel::QueryableByName)]
struct PragmaRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    foreign_keys: i32,
}

#[derive(diesel::QueryableByName)]
struct BidYearRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    bid_year_id: i64,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    year: i32,
}

#[derive(diesel::QueryableByName)]
struct BidYearMetadataRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    year: i32,
    #[diesel(sql_type = diesel::sql_types::Text)]
    start_date: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    num_pay_periods: i32,
}

#[derive(diesel::QueryableByName)]
struct AreaRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    area_id: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    bid_year_id: i64,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    year: i32,
    #[diesel(sql_type = diesel::sql_types::Text)]
    code: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    name: Option<String>,
}

#[derive(diesel::QueryableByName)]
struct AreaListRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    area_id: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    code: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    name: Option<String>,
}

#[derive(diesel::QueryableByName)]
struct UserRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    user_id: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    initials: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    user_type: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    crew: Option<i32>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    cumulative_natca_bu_date: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    natca_bu_date: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    eod_faa_date: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    service_computation_date: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    lottery_value: Option<i32>,
}

#[derive(diesel::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

#[derive(diesel::QueryableByName)]
struct YearOnlyRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    year: i32,
}

#[derive(diesel::QueryableByName)]
struct ExpectedCountRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Integer>)]
    expected_count: Option<i32>,
}

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
pub fn verify_foreign_key_enforcement(conn: &mut SqliteConnection) -> Result<(), PersistenceError> {
    let foreign_keys_enabled: i32 = sql_query("PRAGMA foreign_keys")
        .get_result::<PragmaRow>(conn)?
        .foreign_keys;

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
pub fn get_bootstrap_metadata(
    conn: &mut SqliteConnection,
) -> Result<BootstrapMetadata, PersistenceError> {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Query canonical bid_years table
    let bid_year_rows: Vec<BidYearRow> = sql_query(
        "SELECT bid_year_id, year
         FROM bid_years
         ORDER BY year ASC",
    )
    .load::<BidYearRow>(conn)?;

    for row in bid_year_rows {
        let year: u16 = u16::try_from(row.year).expect("bid_year value out of u16 range");
        metadata
            .bid_years
            .push(BidYear::with_id(row.bid_year_id, year));
    }

    // Query canonical areas table
    let area_rows: Vec<AreaRow> = sql_query(
        "SELECT a.area_id, a.bid_year_id, b.year, a.area_code AS code, a.area_name AS name
         FROM areas a
         JOIN bid_years b ON a.bid_year_id = b.bid_year_id
         ORDER BY b.year ASC, a.area_code ASC",
    )
    .load::<AreaRow>(conn)?;

    for row in area_rows {
        let year: u16 = u16::try_from(row.year).expect("bid_year value out of u16 range");
        let bid_year: BidYear = BidYear::with_id(row.bid_year_id, year);
        let area: Area = Area::with_id(row.area_id, &row.code, row.name);
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
pub fn list_bid_years(
    conn: &mut SqliteConnection,
) -> Result<Vec<CanonicalBidYear>, PersistenceError> {
    let rows: Vec<BidYearMetadataRow> = sql_query(
        "SELECT year, start_date, num_pay_periods
         FROM bid_years
         ORDER BY year ASC",
    )
    .load::<BidYearMetadataRow>(conn)?;

    let mut bid_years: Vec<CanonicalBidYear> = Vec::new();
    for row in rows {
        let year_value: i32 = row.year;
        let start_date_str: String = row.start_date;
        let num_pay_periods_value: i32 = row.num_pay_periods;

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
/// Phase 23A: Now constructs Area objects with their IDs.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year` - The bid year to list areas for
///
/// # Errors
///
/// Returns an error if the database cannot be queried.
pub fn list_areas(
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
) -> Result<Vec<Area>, PersistenceError> {
    // Phase 23A: Look up the bid_year_id if not already present
    // If the bid year doesn't exist, return an empty list
    let bid_year_id: i64 = match bid_year.bid_year_id() {
        Some(id) => id,
        None => match super::queries::lookup_bid_year_id(conn, bid_year.year()) {
            Ok(id) => id,
            Err(PersistenceError::ReconstructionError(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        },
    };

    let rows: Vec<AreaListRow> = sql_query(
        "SELECT area_id, area_code AS code, area_name AS name
         FROM areas
         WHERE bid_year_id = ?1
         ORDER BY area_code ASC",
    )
    .bind::<diesel::sql_types::BigInt, _>(bid_year_id)
    .load::<AreaListRow>(conn)?;

    let areas: Vec<Area> = rows
        .into_iter()
        .map(|row| Area::with_id(row.area_id, &row.code, row.name))
        .collect();

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
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<Vec<User>, PersistenceError> {
    // Phase 23A: Look up the canonical IDs
    let bid_year_id = super::queries::lookup_bid_year_id(conn, bid_year.year())?;
    let area_id = super::queries::lookup_area_id(conn, bid_year_id, area.id())?;

    let rows: Vec<UserRow> = sql_query(
        "SELECT user_id, initials, name, user_type, crew,
                cumulative_natca_bu_date, natca_bu_date, eod_faa_date,
                service_computation_date, lottery_value
         FROM users
         WHERE bid_year_id = ?1 AND area_id = ?2
         ORDER BY initials ASC",
    )
    .bind::<diesel::sql_types::BigInt, _>(bid_year_id)
    .bind::<diesel::sql_types::BigInt, _>(area_id)
    .load::<UserRow>(conn)?;

    let mut users: Vec<User> = Vec::new();
    for row in rows {
        let initials: Initials = Initials::new(&row.initials);
        let user_type: UserType = UserType::parse(&row.user_type)
            .map_err(|e| PersistenceError::ReconstructionError(e.to_string()))?;
        let crew: Option<Crew> = row
            .crew
            .and_then(|n| u8::try_from(n).ok().and_then(|num| Crew::new(num).ok()));
        let seniority_data: SeniorityData = SeniorityData::new(
            row.cumulative_natca_bu_date,
            row.natca_bu_date,
            row.eod_faa_date,
            row.service_computation_date,
            row.lottery_value.and_then(|v| u32::try_from(v).ok()),
        );

        let user: User = User::with_id(
            row.user_id,
            bid_year.clone(),
            initials,
            row.name,
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
pub fn set_active_bid_year(conn: &mut SqliteConnection, year: u16) -> Result<(), PersistenceError> {
    // First, clear all active flags
    sql_query("UPDATE bid_years SET is_active = 0").execute(conn)?;

    // Then set the specified year as active
    let rows_affected: usize = sql_query("UPDATE bid_years SET is_active = 1 WHERE year = ?1")
        .bind::<diesel::sql_types::Integer, _>(i32::from(year))
        .execute(conn)?;

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
pub fn get_active_bid_year(conn: &mut SqliteConnection) -> Result<Option<u16>, PersistenceError> {
    let result: Result<YearOnlyRow, diesel::result::Error> =
        sql_query("SELECT year FROM bid_years WHERE is_active = 1").get_result::<YearOnlyRow>(conn);

    match result {
        Ok(row) => {
            let year: u16 = u16::try_from(row.year).expect("bid_year value out of u16 range");
            Ok(Some(year))
        }
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(PersistenceError::from(e)),
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
    conn: &mut SqliteConnection,
    year: u16,
    expected_count: u32,
) -> Result<(), PersistenceError> {
    let rows_affected: usize =
        sql_query("UPDATE bid_years SET expected_area_count = ?1 WHERE year = ?2")
            .bind::<diesel::sql_types::Integer, _>(i32::try_from(expected_count).map_err(|_| {
                PersistenceError::Other(format!(
                    "expected_count out of i32 range: {expected_count}"
                ))
            })?)
            .bind::<diesel::sql_types::Integer, _>(i32::from(year))
            .execute(conn)?;

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
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
    area: &Area,
    expected_count: u32,
) -> Result<(), PersistenceError> {
    // Phase 23A: Look up the canonical IDs
    let bid_year_id = super::queries::lookup_bid_year_id(conn, bid_year.year())?;
    let area_id = super::queries::lookup_area_id(conn, bid_year_id, area.id())?;

    let rows_affected: usize = sql_query(
        "UPDATE areas SET expected_user_count = ?1 WHERE bid_year_id = ?2 AND area_id = ?3",
    )
    .bind::<diesel::sql_types::Integer, _>(i32::try_from(expected_count).map_err(|_| {
        PersistenceError::Other(format!("expected_count out of i32 range: {expected_count}"))
    })?)
    .bind::<diesel::sql_types::BigInt, _>(bid_year_id)
    .bind::<diesel::sql_types::BigInt, _>(area_id)
    .execute(conn)?;

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
    conn: &mut SqliteConnection,
    year: u16,
) -> Result<Option<u32>, PersistenceError> {
    let result: Result<ExpectedCountRow, diesel::result::Error> =
        sql_query("SELECT expected_area_count AS expected_count FROM bid_years WHERE year = ?1")
            .bind::<diesel::sql_types::Integer, _>(i32::from(year))
            .get_result::<ExpectedCountRow>(conn);

    match result {
        Ok(row) => Ok(row.expected_count.and_then(|c| u32::try_from(c).ok())),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(PersistenceError::from(e)),
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
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<Option<u32>, PersistenceError> {
    // Phase 23A: Look up the canonical IDs
    let bid_year_id = super::queries::lookup_bid_year_id(conn, bid_year.year())?;
    let area_id = super::queries::lookup_area_id(conn, bid_year_id, area.id())?;

    let result: Result<ExpectedCountRow, diesel::result::Error> =
        sql_query("SELECT expected_user_count AS expected_count FROM areas WHERE bid_year_id = ?1 AND area_id = ?2")
            .bind::<diesel::sql_types::BigInt, _>(bid_year_id)
            .bind::<diesel::sql_types::BigInt, _>(area_id)
            .get_result::<ExpectedCountRow>(conn);

    match result {
        Ok(row) => Ok(row.expected_count.and_then(|c| u32::try_from(c).ok())),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(PersistenceError::from(e)),
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
pub fn get_actual_area_count(
    conn: &mut SqliteConnection,
    year: u16,
) -> Result<usize, PersistenceError> {
    // Phase 23A: Look up the canonical ID
    let bid_year_id = super::queries::lookup_bid_year_id(conn, year)?;

    let count: i64 = sql_query("SELECT COUNT(*) as count FROM areas WHERE bid_year_id = ?1")
        .bind::<diesel::sql_types::BigInt, _>(bid_year_id)
        .get_result::<CountRow>(conn)?
        .count;

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
    conn: &mut SqliteConnection,
    bid_year: &BidYear,
    area: &Area,
) -> Result<usize, PersistenceError> {
    // Phase 23A: Look up the canonical IDs
    let bid_year_id = super::queries::lookup_bid_year_id(conn, bid_year.year())?;
    let area_id = super::queries::lookup_area_id(conn, bid_year_id, area.id())?;

    let count: i64 =
        sql_query("SELECT COUNT(*) as count FROM users WHERE bid_year_id = ?1 AND area_id = ?2")
            .bind::<diesel::sql_types::BigInt, _>(bid_year_id)
            .bind::<diesel::sql_types::BigInt, _>(area_id)
            .get_result::<CountRow>(conn)?
            .count;

    Ok(usize::try_from(count).expect("count out of usize range"))
}
