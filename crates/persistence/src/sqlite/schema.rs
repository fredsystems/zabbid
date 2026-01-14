// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use rusqlite::Connection;
use tracing::info;

use crate::error::PersistenceError;

/// Initializes the operator and session tables.
fn initialize_operator_schema(conn: &Connection) -> Result<(), PersistenceError> {
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
        ",
    )?;
    Ok(())
}

/// Initializes the canonical state tables.
fn initialize_canonical_schema(conn: &Connection) -> Result<(), PersistenceError> {
    conn.execute_batch(
        "
        -- Canonical state tables (Phase 7, Phase 23A: canonical IDs)
        CREATE TABLE IF NOT EXISTS bid_years (
            bid_year_id INTEGER PRIMARY KEY AUTOINCREMENT,
            year INTEGER NOT NULL UNIQUE,
            start_date TEXT NOT NULL,
            num_pay_periods INTEGER NOT NULL CHECK(num_pay_periods IN (26, 27)),
            is_active INTEGER NOT NULL DEFAULT 0 CHECK(is_active IN (0, 1)),
            expected_area_count INTEGER
        );

        CREATE TABLE IF NOT EXISTS areas (
            area_id INTEGER PRIMARY KEY AUTOINCREMENT,
            bid_year_id INTEGER NOT NULL,
            area_code TEXT NOT NULL,
            area_name TEXT,
            expected_user_count INTEGER,
            UNIQUE (bid_year_id, area_code),
            FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id)
        );

        CREATE TABLE IF NOT EXISTS users (
            user_id INTEGER PRIMARY KEY AUTOINCREMENT,
            bid_year_id INTEGER NOT NULL,
            area_id INTEGER NOT NULL,
            initials TEXT NOT NULL,
            name TEXT NOT NULL,
            user_type TEXT NOT NULL,
            crew INTEGER,
            cumulative_natca_bu_date TEXT NOT NULL,
            natca_bu_date TEXT NOT NULL,
            eod_faa_date TEXT NOT NULL,
            service_computation_date TEXT NOT NULL,
            lottery_value INTEGER,
            UNIQUE (bid_year_id, area_id, initials),
            FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
            FOREIGN KEY(area_id) REFERENCES areas(area_id)
        );

        CREATE INDEX IF NOT EXISTS idx_users_by_area
            ON users(bid_year_id, area_id);

        CREATE INDEX IF NOT EXISTS idx_users_by_initials
            ON users(bid_year_id, area_id, initials);
        ",
    )?;

    Ok(())
}

/// Initializes the audit log and snapshot tables.
fn initialize_audit_schema(conn: &Connection) -> Result<(), PersistenceError> {
    conn.execute_batch(
        "
        -- Audit log and derived historical state tables
        -- Phase 23A: Now use canonical IDs with FKs, but area_id can be NULL for CreateBidYear
        -- Phase 23B: bid_year_id can also be NULL for global events (operator management)
        CREATE TABLE IF NOT EXISTS audit_events (
            event_id INTEGER PRIMARY KEY AUTOINCREMENT,
            bid_year_id INTEGER,
            area_id INTEGER,
            year INTEGER NOT NULL,
            area_code TEXT NOT NULL,
            actor_operator_id INTEGER NOT NULL,
            actor_login_name TEXT NOT NULL,
            actor_display_name TEXT NOT NULL,
            actor_json TEXT NOT NULL,
            cause_json TEXT NOT NULL,
            action_json TEXT NOT NULL,
            before_snapshot_json TEXT NOT NULL,
            after_snapshot_json TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(actor_operator_id) REFERENCES operators(operator_id) ON DELETE RESTRICT,
            FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
            FOREIGN KEY(area_id) REFERENCES areas(area_id)
        );

        CREATE INDEX IF NOT EXISTS idx_audit_events_scope
            ON audit_events(bid_year_id, area_id, event_id);

        CREATE TABLE IF NOT EXISTS state_snapshots (
            snapshot_id INTEGER PRIMARY KEY AUTOINCREMENT,
            bid_year_id INTEGER NOT NULL,
            area_id INTEGER NOT NULL,
            event_id INTEGER NOT NULL,
            state_json TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(bid_year_id, area_id, event_id),
            FOREIGN KEY(event_id) REFERENCES audit_events(event_id),
            FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
            FOREIGN KEY(area_id) REFERENCES areas(area_id)
        );

        CREATE INDEX IF NOT EXISTS idx_state_snapshots_scope
            ON state_snapshots(bid_year_id, area_id, event_id DESC);
        ",
    )?;
    Ok(())
}

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

    // Initialize schema components
    initialize_operator_schema(conn)?;
    initialize_canonical_schema(conn)?;
    initialize_audit_schema(conn)?;

    Ok(())
}
