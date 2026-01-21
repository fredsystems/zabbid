// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Bid status mutation operations.
//!
//! This module provides functions for creating and updating bid status records.
//! Status transitions are operator-initiated and generate audit events.

use crate::data_models::{NewBidStatus, NewBidStatusHistory};
use crate::diesel_schema::{bid_status, bid_status_history};
use crate::error::PersistenceError;
use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};

backend_fn! {

/// Insert initial bid status records (at confirmation).
///
/// This function is used to bulk-create initial status records for all users
/// in all rounds after confirmation.
///
/// # Backend-agnostic
///
/// This function uses Diesel DSL exclusively and works with both `SQLite` and `MySQL`.
#[allow(dead_code)]
pub fn bulk_insert_bid_status(
    conn: &mut _,
    records: &[NewBidStatus],
) -> Result<(), PersistenceError> {
    diesel::insert_into(bid_status::table)
        .values(records)
        .execute(conn)?;
    Ok(())
}

}

backend_fn! {

/// Update a single bid status record.
///
/// # Backend-agnostic
///
/// This function uses Diesel DSL exclusively.
#[allow(dead_code)]
pub fn update_bid_status(
    conn: &mut _,
    bid_status_id: i64,
    new_status: &str,
    updated_at: &str,
    updated_by: i64,
    notes: Option<String>,
) -> Result<(), PersistenceError> {
    diesel::update(bid_status::table.filter(bid_status::bid_status_id.eq(bid_status_id)))
        .set((
            bid_status::status.eq(new_status),
            bid_status::updated_at.eq(updated_at),
            bid_status::updated_by.eq(updated_by),
            bid_status::notes.eq(notes),
        ))
        .execute(conn)?;
    Ok(())
}

}

backend_fn! {

/// Insert a bid status history record.
///
/// # Backend-agnostic
///
/// This function uses Diesel DSL exclusively.
///
/// # Errors
///
/// Returns an error if the database insert fails.
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn insert_bid_status_history(
    conn: &mut _,
    bid_status_id: i64,
    audit_event_id: i64,
    previous_status: Option<&str>,
    new_status: &str,
    transitioned_at: &str,
    transitioned_by: i64,
    notes: Option<&str>,
) -> Result<(), PersistenceError> {
    let record = NewBidStatusHistory {
        bid_status_id,
        audit_event_id,
        previous_status: previous_status.map(ToString::to_string),
        new_status: new_status.to_string(),
        transitioned_at: transitioned_at.to_string(),
        transitioned_by,
        notes: notes.map(ToString::to_string),
    };

    diesel::insert_into(bid_status_history::table)
        .values(&record)
        .execute(conn)?;
    Ok(())
}

}

backend_fn! {

/// Bulk insert bid status history records.
///
/// Used when recording multiple status transitions at once.
#[allow(dead_code)]
pub fn bulk_insert_bid_status_history(
    conn: &mut _,
    records: &[NewBidStatusHistory],
) -> Result<(), PersistenceError> {
    diesel::insert_into(bid_status_history::table)
        .values(records)
        .execute(conn)?;
    Ok(())
}

}
