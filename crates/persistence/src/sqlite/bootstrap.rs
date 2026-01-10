// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use rusqlite::{Connection, params};
use zab_bid::BootstrapMetadata;
use zab_bid_domain::{Area, BidYear, Crew, Initials, SeniorityData, User, UserType};

use crate::error::PersistenceError;

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

/// Lists all bid years that have been created.
///
/// This queries the canonical `bid_years` table directly.
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
/// Panics if a bid year value from the database cannot be converted to `u16`.
/// This should never happen in practice as the schema enforces valid ranges.
pub fn list_bid_years(conn: &Connection) -> Result<Vec<BidYear>, PersistenceError> {
    let mut stmt = conn.prepare("SELECT year FROM bid_years ORDER BY year ASC")?;

    let rows = stmt.query_map([], |row| {
        let year_value: i32 = row.get(0)?;
        Ok(u16::try_from(year_value).expect("bid_year value out of u16 range"))
    })?;

    let mut bid_years: Vec<BidYear> = Vec::new();
    for row_result in rows {
        let year: u16 = row_result?;
        bid_years.push(BidYear::new(year));
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
