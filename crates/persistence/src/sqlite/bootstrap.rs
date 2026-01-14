// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use time::Date;
use tracing::info;
use zab_bid::BootstrapMetadata;
use zab_bid_domain::{
    Area, BidYear, CanonicalBidYear, Crew, Initials, SeniorityData, User, UserType,
};

use crate::error::PersistenceError;

// Helper row struct for PRAGMA queries (justified raw SQL use)
#[derive(diesel::QueryableByName)]
struct PragmaRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    foreign_keys: i32,
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
    let foreign_keys_enabled: i32 = diesel::sql_query("PRAGMA foreign_keys")
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
    use crate::diesel_schema::bid_years;

    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Query canonical bid_years table
    let bid_year_rows: Vec<(i64, i32)> = bid_years::table
        .select((bid_years::bid_year_id, bid_years::year))
        .order(bid_years::year.asc())
        .load::<(i64, i32)>(conn)?;

    for (bid_year_id, year_value) in bid_year_rows {
        let year: u16 = u16::try_from(year_value).expect("bid_year value out of u16 range");
        metadata.bid_years.push(BidYear::with_id(bid_year_id, year));
    }

    // Query canonical areas table - need to adjust this section
    let area_rows: Vec<(i64, i64, i32, String, Option<String>)> = {
        use crate::diesel_schema::areas;

        areas::table
            .inner_join(bid_years::table)
            .select((
                areas::area_id,
                areas::bid_year_id,
                bid_years::year,
                areas::area_code,
                areas::area_name,
            ))
            .order((bid_years::year.asc(), areas::area_code.asc()))
            .load::<(i64, i64, i32, String, Option<String>)>(conn)?
    };

    for (area_id, bid_year_id_val, year_value, code, name) in area_rows {
        let year: u16 = u16::try_from(year_value).expect("bid_year value out of u16 range");
        let bid_year: BidYear = BidYear::with_id(bid_year_id_val, year);
        let area: Area = Area::with_id(area_id, &code, name);
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
    use crate::diesel_schema::bid_years;

    let rows: Vec<(i32, String, i32)> = bid_years::table
        .select((
            bid_years::year,
            bid_years::start_date,
            bid_years::num_pay_periods,
        ))
        .order(bid_years::year.asc())
        .load::<(i32, String, i32)>(conn)?;

    let mut bid_years: Vec<CanonicalBidYear> = Vec::new();
    for (year_value, start_date_str, num_pay_periods_value) in rows {
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
    use crate::diesel_schema::areas;

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

    let rows: Vec<(i64, String, Option<String>)> = areas::table
        .select((areas::area_id, areas::area_code, areas::area_name))
        .filter(areas::bid_year_id.eq(bid_year_id))
        .order(areas::area_code.asc())
        .load::<(i64, String, Option<String>)>(conn)?;

    let areas: Vec<Area> = rows
        .into_iter()
        .map(|(area_id, code, name)| Area::with_id(area_id, &code, name))
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
    use crate::diesel_schema::users;

    // Type alias for the complex user row tuple
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
    );

    // Phase 23A: Look up the canonical IDs
    let bid_year_id = super::queries::lookup_bid_year_id(conn, bid_year.year())?;
    let area_id = super::queries::lookup_area_id(conn, bid_year_id, area.id())?;

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
        ))
        .filter(users::bid_year_id.eq(bid_year_id))
        .filter(users::area_id.eq(area_id))
        .order(users::initials.asc())
        .load(conn)?;

    let mut users: Vec<User> = Vec::new();
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
    use crate::diesel_schema::bid_years;

    // First, clear all active flags
    diesel::update(bid_years::table)
        .set(bid_years::is_active.eq(0))
        .execute(conn)?;

    // Then set the specified year as active
    let rows_affected: usize = diesel::update(bid_years::table)
        .filter(bid_years::year.eq(i32::from(year)))
        .set(bid_years::is_active.eq(1))
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
    use crate::diesel_schema::bid_years;

    let result: Result<i32, diesel::result::Error> = bid_years::table
        .select(bid_years::year)
        .filter(bid_years::is_active.eq(1))
        .first::<i32>(conn);

    match result {
        Ok(year_value) => {
            let year: u16 = u16::try_from(year_value).expect("bid_year value out of u16 range");
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
    use crate::diesel_schema::bid_years;

    let count_i32: i32 = i32::try_from(expected_count).map_err(|_| {
        PersistenceError::Other(format!("expected_count out of i32 range: {expected_count}"))
    })?;

    let rows_affected: usize = diesel::update(bid_years::table)
        .filter(bid_years::year.eq(i32::from(year)))
        .set(bid_years::expected_area_count.eq(count_i32))
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
    use crate::diesel_schema::areas;

    // Phase 23A: Look up the canonical IDs
    let bid_year_id = super::queries::lookup_bid_year_id(conn, bid_year.year())?;
    let area_id = super::queries::lookup_area_id(conn, bid_year_id, area.id())?;

    let count_i32: i32 = i32::try_from(expected_count).map_err(|_| {
        PersistenceError::Other(format!("expected_count out of i32 range: {expected_count}"))
    })?;

    let rows_affected: usize = diesel::update(areas::table)
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::area_id.eq(area_id))
        .set(areas::expected_user_count.eq(count_i32))
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
    use crate::diesel_schema::bid_years;

    let result: Result<Option<i32>, diesel::result::Error> = bid_years::table
        .select(bid_years::expected_area_count)
        .filter(bid_years::year.eq(i32::from(year)))
        .first::<Option<i32>>(conn);

    match result {
        Ok(opt_count) => Ok(opt_count.and_then(|c| u32::try_from(c).ok())),
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
    use crate::diesel_schema::areas;

    // Phase 23A: Look up the canonical IDs
    let bid_year_id = super::queries::lookup_bid_year_id(conn, bid_year.year())?;
    let area_id = super::queries::lookup_area_id(conn, bid_year_id, area.id())?;

    let result: Result<Option<i32>, diesel::result::Error> = areas::table
        .select(areas::expected_user_count)
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::area_id.eq(area_id))
        .first::<Option<i32>>(conn);

    match result {
        Ok(opt_count) => Ok(opt_count.and_then(|c| u32::try_from(c).ok())),
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
    use crate::diesel_schema::areas;

    // Phase 23A: Look up the canonical ID
    let bid_year_id = super::queries::lookup_bid_year_id(conn, year)?;

    let count: i64 = areas::table
        .filter(areas::bid_year_id.eq(bid_year_id))
        .count()
        .get_result(conn)?;

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
    use crate::diesel_schema::users;

    // Phase 23A: Look up the canonical IDs
    let bid_year_id = super::queries::lookup_bid_year_id(conn, bid_year.year())?;
    let area_id = super::queries::lookup_area_id(conn, bid_year_id, area.id())?;

    let count: i64 = users::table
        .filter(users::bid_year_id.eq(bid_year_id))
        .filter(users::area_id.eq(area_id))
        .count()
        .get_result(conn)?;

    Ok(usize::try_from(count).expect("count out of usize range"))
}
