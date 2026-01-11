// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use rusqlite::Connection;
use tracing::info;

use crate::error::PersistenceError;

/// Initializes the database schema.
///
/// # Arguments
///
/// * `conn` - The database connection to initialize
///
/// # Errors
///
/// Returns an error if schema creation fails.
pub fn initialize_schema(conn: &Connection) -> Result<(), PersistenceError> {
    info!("Initializing database schema");

    conn.execute_batch(
        "
        -- Canonical state tables (Phase 7)
        CREATE TABLE IF NOT EXISTS bid_years (
            year INTEGER PRIMARY KEY NOT NULL,
            start_date TEXT NOT NULL,
            num_pay_periods INTEGER NOT NULL CHECK(num_pay_periods IN (26, 27))
        );

        CREATE TABLE IF NOT EXISTS areas (
            bid_year INTEGER NOT NULL,
            area_id TEXT NOT NULL,
            PRIMARY KEY (bid_year, area_id),
            FOREIGN KEY(bid_year) REFERENCES bid_years(year)
        );

        CREATE TABLE IF NOT EXISTS users (
            bid_year INTEGER NOT NULL,
            area_id TEXT NOT NULL,
            initials TEXT NOT NULL,
            name TEXT NOT NULL,
            user_type TEXT NOT NULL,
            crew INTEGER,
            cumulative_natca_bu_date TEXT NOT NULL,
            natca_bu_date TEXT NOT NULL,
            eod_faa_date TEXT NOT NULL,
            service_computation_date TEXT NOT NULL,
            lottery_value INTEGER,
            PRIMARY KEY (bid_year, area_id, initials),
            FOREIGN KEY(bid_year, area_id) REFERENCES areas(bid_year, area_id)
        );

        CREATE INDEX IF NOT EXISTS idx_users_by_area
            ON users(bid_year, area_id);

        -- Audit log and derived historical state tables
        CREATE TABLE IF NOT EXISTS audit_events (
            event_id INTEGER PRIMARY KEY AUTOINCREMENT,
            bid_year INTEGER NOT NULL,
            area TEXT NOT NULL,
            actor_json TEXT NOT NULL,
            cause_json TEXT NOT NULL,
            action_json TEXT NOT NULL,
            before_snapshot_json TEXT NOT NULL,
            after_snapshot_json TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(bid_year, area, event_id)
        );

        CREATE INDEX IF NOT EXISTS idx_audit_events_scope
            ON audit_events(bid_year, area, event_id);

        CREATE TABLE IF NOT EXISTS state_snapshots (
            snapshot_id INTEGER PRIMARY KEY AUTOINCREMENT,
            bid_year INTEGER NOT NULL,
            area TEXT NOT NULL,
            event_id INTEGER NOT NULL,
            state_json TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(bid_year, area, event_id),
            FOREIGN KEY(event_id) REFERENCES audit_events(event_id)
        );

        CREATE INDEX IF NOT EXISTS idx_state_snapshots_scope
            ON state_snapshots(bid_year, area, event_id DESC);
        ",
    )?;

    Ok(())
}
