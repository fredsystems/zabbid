// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Operator and session queries.
//!
//! This module contains backend-agnostic queries for retrieving operators
//! and sessions. All queries use Diesel DSL and work across all supported
//! database backends.

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use tracing::debug;

use crate::data_models::{OperatorData, SessionData};
use crate::diesel_schema::{audit_events, operators, sessions};
use crate::error::PersistenceError;

/// Diesel Queryable struct for operator rows.
#[derive(Queryable, Selectable)]
#[diesel(table_name = operators)]
struct OperatorRow {
    operator_id: i64,
    login_name: String,
    display_name: String,
    password_hash: String,
    role: String,
    is_disabled: i32,
    created_at: String,
    disabled_at: Option<String>,
    last_login_at: Option<String>,
}

/// Diesel Queryable struct for session rows.
#[derive(Queryable, Selectable)]
#[diesel(table_name = sessions)]
struct SessionRow {
    session_id: i64,
    session_token: String,
    operator_id: i64,
    created_at: String,
    last_activity_at: String,
    expires_at: String,
}

backend_fn! {
/// Retrieves an operator by login name.
///
/// The `login_name` is normalized to uppercase for case-insensitive lookup.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `login_name` - The login name to search for
///
/// # Errors
///
/// Returns an error if the database query fails.
/// Returns `Ok(None)` if the operator is not found.
pub fn get_operator_by_login(
    conn: &mut _,
    login_name: &str,
) -> Result<Option<OperatorData>, PersistenceError> {
    let normalized_login: String = login_name.to_uppercase();

    debug!("Looking up operator by login_name: {}", normalized_login);

    let result: Result<OperatorRow, diesel::result::Error> = operators::table
        .filter(operators::login_name.eq(&normalized_login))
        .select(OperatorRow::as_select())
        .first(conn);

    match result {
        Ok(row) => Ok(Some(OperatorData {
            operator_id: row.operator_id,
            login_name: row.login_name,
            display_name: row.display_name,
            password_hash: row.password_hash,
            role: row.role,
            is_disabled: row.is_disabled != 0,
            created_at: row.created_at,
            disabled_at: row.disabled_at,
            last_login_at: row.last_login_at,
        })),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
/// Retrieves an operator by ID.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `operator_id` - The operator ID
///
/// # Errors
///
/// Returns an error if the database query fails.
/// Returns `Ok(None)` if the operator is not found.
pub fn get_operator_by_id(
    conn: &mut _,
    operator_id: i64,
) -> Result<Option<OperatorData>, PersistenceError> {
    debug!("Looking up operator by ID: {}", operator_id);

    let result: Result<OperatorRow, diesel::result::Error> = operators::table
        .filter(operators::operator_id.eq(operator_id))
        .select(OperatorRow::as_select())
        .first(conn);

    match result {
        Ok(row) => Ok(Some(OperatorData {
            operator_id: row.operator_id,
            login_name: row.login_name,
            display_name: row.display_name,
            password_hash: row.password_hash,
            role: row.role,
            is_disabled: row.is_disabled != 0,
            created_at: row.created_at,
            disabled_at: row.disabled_at,
            last_login_at: row.last_login_at,
        })),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
/// Retrieves a session by token.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `session_token` - The session token
///
/// # Errors
///
/// Returns an error if the database query fails.
/// Returns `Ok(None)` if the session is not found.
pub fn get_session_by_token(
    conn: &mut _,
    session_token: &str,
) -> Result<Option<SessionData>, PersistenceError> {
    debug!("Looking up session by token");

    let result: Result<SessionRow, diesel::result::Error> = sessions::table
        .filter(sessions::session_token.eq(session_token))
        .select(SessionRow::as_select())
        .first(conn);

    match result {
        Ok(row) => Ok(Some(SessionData {
            session_id: row.session_id,
            session_token: row.session_token,
            operator_id: row.operator_id,
            created_at: row.created_at,
            last_activity_at: row.last_activity_at,
            expires_at: row.expires_at,
        })),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(PersistenceError::from(e)),
    }
}
}

backend_fn! {
/// Checks if an operator is referenced by any audit events.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `operator_id` - The operator ID to check
///
/// # Errors
///
/// Returns an error if the database query fails.
pub fn is_operator_referenced(conn: &mut _, operator_id: i64) -> Result<bool, PersistenceError> {
    use diesel::dsl::count;

    debug!(
        "Checking if operator ID {} is referenced in audit events",
        operator_id
    );

    let count: i64 = audit_events::table
        .filter(audit_events::actor_operator_id.eq(operator_id))
        .select(count(audit_events::event_id))
        .first(conn)?;

    Ok(count > 0)
}
}

backend_fn! {
/// Lists all operators.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database query fails.
pub fn list_operators(conn: &mut _) -> Result<Vec<OperatorData>, PersistenceError> {
    debug!("Listing all operators");

    let rows: Vec<OperatorRow> = operators::table
        .select(OperatorRow::as_select())
        .order_by(operators::login_name.asc())
        .load(conn)?;

    let operators_list: Vec<OperatorData> = rows
        .into_iter()
        .map(|row| OperatorData {
            operator_id: row.operator_id,
            login_name: row.login_name,
            display_name: row.display_name,
            password_hash: row.password_hash,
            role: row.role,
            is_disabled: row.is_disabled != 0,
            created_at: row.created_at,
            disabled_at: row.disabled_at,
            last_login_at: row.last_login_at,
        })
        .collect();

    Ok(operators_list)
}
}

backend_fn! {
/// Counts the total number of operators.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database query fails.
pub fn count_operators(conn: &mut _) -> Result<i64, PersistenceError> {
    use diesel::dsl::count;

    debug!("Counting operators");

    let count: i64 = operators::table
        .select(count(operators::operator_id))
        .first(conn)?;

    debug!("Total operators: {}", count);
    Ok(count)
}
}

backend_fn! {
/// Counts the number of active admin operators.
///
/// An active admin operator is one where:
/// - `role` is 'Admin'
/// - `is_disabled` is false
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database query fails.
pub fn count_active_admin_operators(conn: &mut _) -> Result<i64, PersistenceError> {
    use diesel::dsl::count;

    debug!("Counting active admin operators");

    let count: i64 = operators::table
        .filter(operators::role.eq("Admin"))
        .filter(operators::is_disabled.eq(0))
        .select(count(operators::operator_id))
        .first(conn)?;

    debug!("Active admin operators: {}", count);
    Ok(count)
}
}

/// Verifies a password against a stored hash.
///
/// This is a backend-agnostic utility function that uses bcrypt.
///
/// # Arguments
///
/// * `password` - The plain text password to verify
/// * `password_hash` - The stored bcrypt hash
///
/// # Errors
///
/// Returns an error if password verification fails.
pub fn verify_password(password: &str, password_hash: &str) -> Result<bool, PersistenceError> {
    bcrypt::verify(password, password_hash)
        .map_err(|e| PersistenceError::Other(format!("Failed to verify password: {e}")))
}
