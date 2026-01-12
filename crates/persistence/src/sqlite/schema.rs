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

    // Enable foreign key enforcement
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    conn.execute_batch(
        "
        -- Operator tables (Phase 14)
        CREATE TABLE IF NOT EXISTS operators (
            operator_id INTEGER PRIMARY KEY AUTOINCREMENT,
            login_name TEXT NOT NULL UNIQUE COLLATE NOCASE,
            display_name TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('Admin', 'Bidder')),
            is_disabled INTEGER NOT NULL DEFAULT 0 CHECK(is_disabled IN (0, 1)),
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            disabled_at DATETIME,
            last_login_at DATETIME
        );

        CREATE TABLE IF NOT EXISTS sessions (
            session_id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_token TEXT NOT NULL UNIQUE,
            operator_id INTEGER NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            last_activity_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expires_at DATETIME NOT NULL,
            FOREIGN KEY(operator_id) REFERENCES operators(operator_id)
        );

        CREATE INDEX IF NOT EXISTS idx_sessions_token
            ON sessions(session_token);

        CREATE INDEX IF NOT EXISTS idx_sessions_operator
            ON sessions(operator_id);

        -- Canonical state tables (Phase 7)
        CREATE TABLE IF NOT EXISTS bid_years (
            year INTEGER PRIMARY KEY NOT NULL,
            start_date TEXT NOT NULL,
            num_pay_periods INTEGER NOT NULL CHECK(num_pay_periods IN (26, 27)),
            is_active INTEGER NOT NULL DEFAULT 0 CHECK(is_active IN (0, 1)),
            expected_area_count INTEGER
        );

        CREATE TABLE IF NOT EXISTS areas (
            bid_year INTEGER NOT NULL,
            area_id TEXT NOT NULL,
            expected_user_count INTEGER,
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
            actor_operator_id INTEGER NOT NULL,
            actor_login_name TEXT NOT NULL,
            actor_display_name TEXT NOT NULL,
            actor_json TEXT NOT NULL,
            cause_json TEXT NOT NULL,
            action_json TEXT NOT NULL,
            before_snapshot_json TEXT NOT NULL,
            after_snapshot_json TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(bid_year, area, event_id),
            FOREIGN KEY(actor_operator_id) REFERENCES operators(operator_id) ON DELETE RESTRICT
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
