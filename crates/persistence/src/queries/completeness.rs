// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Completeness and count queries.
//!
//! This module contains backend-agnostic queries for counting entities
//! across bid years and areas. All queries use Diesel DSL and work across
//! all supported database backends.

use diesel::prelude::*;
use num_traits::ToPrimitive;

use crate::diesel_schema::{areas, bid_years, users};
use crate::error::PersistenceError;

/// Counts users per area for a given bid year.
///
/// Returns a vector of tuples containing (`area_code`, `user_count`).
///
/// Phase 23A: Now uses `bid_year_id` for queries.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `bid_year_id` - The canonical bid year ID
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if count conversion fails.
pub fn count_users_by_area(
    conn: &mut SqliteConnection,
    bid_year_id: i64,
) -> Result<Vec<(String, usize)>, PersistenceError> {
    let rows = users::table
        .inner_join(areas::table.on(users::area_id.eq(areas::area_id)))
        .filter(users::bid_year_id.eq(bid_year_id))
        .group_by(areas::area_code)
        .order(areas::area_code.asc())
        .select((areas::area_code, diesel::dsl::count(users::user_id)))
        .load::<(String, i64)>(conn)?;

    let mut result: Vec<(String, usize)> = Vec::new();
    for (area_code, count_i64) in rows {
        let count_usize: usize = count_i64.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((area_code, count_usize));
    }

    Ok(result)
}

/// Counts areas per bid year.
///
/// Returns a vector of tuples containing (`bid_year`, `area_count`).
///
/// Phase 23A: Updated to use `bid_year_id` with JOIN.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if conversions fail.
pub fn count_areas_by_bid_year(
    conn: &mut SqliteConnection,
) -> Result<Vec<(u16, usize)>, PersistenceError> {
    let rows = areas::table
        .inner_join(bid_years::table.on(areas::bid_year_id.eq(bid_years::bid_year_id)))
        .group_by(bid_years::year)
        .order(bid_years::year.asc())
        .select((bid_years::year, diesel::dsl::count(areas::area_id)))
        .load::<(i32, i64)>(conn)?;

    let mut result: Vec<(u16, usize)> = Vec::new();
    for (year_i32, count_i64) in rows {
        let bid_year_u16: u16 = year_i32.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = count_i64.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((bid_year_u16, count_usize));
    }

    Ok(result)
}

/// Counts total users per bid year across all areas.
///
/// Returns a vector of tuples containing (`bid_year`, `total_user_count`).
///
/// Phase 23A: Updated to use `bid_year_id`.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if conversions fail.
pub fn count_users_by_bid_year(
    conn: &mut SqliteConnection,
) -> Result<Vec<(u16, usize)>, PersistenceError> {
    let rows = users::table
        .inner_join(bid_years::table.on(users::bid_year_id.eq(bid_years::bid_year_id)))
        .group_by(bid_years::year)
        .order(bid_years::year.asc())
        .select((bid_years::year, diesel::dsl::count(users::user_id)))
        .load::<(i32, i64)>(conn)?;

    let mut result: Vec<(u16, usize)> = Vec::new();
    for (year_i32, count_i64) in rows {
        let bid_year_u16: u16 = year_i32.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = count_i64.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((bid_year_u16, count_usize));
    }

    Ok(result)
}

/// Counts users per (`bid_year`, `area_id`) combination.
///
/// Returns a vector of tuples containing (`bid_year`, `area_code`, `user_count`).
///
/// Phase 23A: Updated to use join tables and return `area_code`.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database cannot be queried or if conversions fail.
pub fn count_users_by_bid_year_and_area(
    conn: &mut SqliteConnection,
) -> Result<Vec<(u16, String, usize)>, PersistenceError> {
    let rows = users::table
        .inner_join(bid_years::table.on(users::bid_year_id.eq(bid_years::bid_year_id)))
        .inner_join(areas::table.on(users::area_id.eq(areas::area_id)))
        .group_by((bid_years::year, areas::area_code))
        .order((bid_years::year.asc(), areas::area_code.asc()))
        .select((
            bid_years::year,
            areas::area_code,
            diesel::dsl::count(users::user_id),
        ))
        .load::<(i32, String, i64)>(conn)?;

    let mut result: Vec<(u16, String, usize)> = Vec::new();
    for (year_i32, area_code, count_i64) in rows {
        let bid_year_u16: u16 = year_i32.to_u16().ok_or_else(|| {
            PersistenceError::DatabaseError("Bid year conversion failed".to_string())
        })?;
        let count_usize: usize = count_i64.to_usize().ok_or_else(|| {
            PersistenceError::DatabaseError("Count conversion failed".to_string())
        })?;
        result.push((bid_year_u16, area_code, count_usize));
    }

    Ok(result)
}
