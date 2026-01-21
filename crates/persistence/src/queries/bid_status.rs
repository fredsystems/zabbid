// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Bid status query operations.
//!
//! This module provides functions for querying bid status records and history.

use crate::data_models::{BidStatusHistoryRow, BidStatusRow};
use crate::diesel_schema::{bid_status, bid_status_history};
use crate::error::PersistenceError;
use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};

backend_fn! {

/// Query bid status for a specific user in a specific round.
///
/// # Backend-agnostic
///
/// This function uses Diesel DSL exclusively.
#[allow(dead_code)]
pub fn get_bid_status_for_user_and_round(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
    user_id: i64,
    round_id: i64,
) -> Result<Option<BidStatusRow>, PersistenceError> {
    bid_status::table
        .filter(bid_status::bid_year_id.eq(bid_year_id))
        .filter(bid_status::area_id.eq(area_id))
        .filter(bid_status::user_id.eq(user_id))
        .filter(bid_status::round_id.eq(round_id))
        .first::<BidStatusRow>(conn)
        .optional()
        .map_err(|e| {
            PersistenceError::QueryFailed(format!("get_bid_status_for_user_and_round: {e}"))
        })
}

}

backend_fn! {

/// Query all bid status records for a given area.
///
/// Returns bid status for all users in all rounds for the specified area.
#[allow(dead_code)]
pub fn get_bid_status_for_area(
    conn: &mut _,
    bid_year_id: i64,
    area_id: i64,
) -> Result<Vec<BidStatusRow>, PersistenceError> {
    bid_status::table
        .filter(bid_status::bid_year_id.eq(bid_year_id))
        .filter(bid_status::area_id.eq(area_id))
        .order(bid_status::round_id.asc())
        .load::<BidStatusRow>(conn)
        .map_err(|e| PersistenceError::QueryFailed(format!("get_bid_status_for_area: {e}")))
}

}

backend_fn! {

/// Query all bid status records for a given round.
///
/// Returns bid status for all users in all areas for the specified round.
#[allow(dead_code)]
pub fn get_bid_status_for_round(
    conn: &mut _,
    bid_year_id: i64,
    round_id: i64,
) -> Result<Vec<BidStatusRow>, PersistenceError> {
    bid_status::table
        .filter(bid_status::bid_year_id.eq(bid_year_id))
        .filter(bid_status::round_id.eq(round_id))
        .load::<BidStatusRow>(conn)
        .map_err(|e| PersistenceError::QueryFailed(format!("get_bid_status_for_round: {e}")))
}

}

backend_fn! {

/// Query bid status history for a specific bid status record.
///
/// Returns all transitions for the given bid status, ordered chronologically.
#[allow(dead_code)]
pub fn get_bid_status_history(
    conn: &mut _,
    bid_status_id: i64,
) -> Result<Vec<BidStatusHistoryRow>, PersistenceError> {
    bid_status_history::table
        .filter(bid_status_history::bid_status_id.eq(bid_status_id))
        .order(bid_status_history::transitioned_at.asc())
        .load::<BidStatusHistoryRow>(conn)
        .map_err(|e| PersistenceError::QueryFailed(format!("get_bid_status_history: {e}")))
}

}
