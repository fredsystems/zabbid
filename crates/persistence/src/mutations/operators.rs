// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Operator and session mutations.
//!
//! This module contains backend-agnostic mutations for persisting operators
//! and sessions. Most mutations use Diesel DSL, with minimal backend-specific
//! helpers abstracted via the `PersistenceBackend` trait.

use diesel::prelude::*;
use diesel::{MysqlConnection, SqliteConnection};
use tracing::{debug, info};

use crate::backend::PersistenceBackend;
use crate::diesel_schema::{operators, sessions};
use crate::error::PersistenceError;
use crate::queries::operators::{is_operator_referenced_mysql, is_operator_referenced_sqlite};

backend_fn! {
/// Creates a new operator.
///
/// The `login_name` is normalized to uppercase for case-insensitive uniqueness.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `login_name` - The login name (will be normalized)
/// * `display_name` - The display name
/// * `password` - The plain-text password (will be hashed)
/// * `role` - The role (Admin or Bidder)
///
/// # Errors
///
/// Returns an error if the operator cannot be created or if the login name
/// already exists.
pub fn create_operator(
    conn: &mut _,
    login_name: &str,
    display_name: &str,
    password: &str,
    role: &str,
) -> Result<i64, PersistenceError> {
    let normalized_login: String = login_name.to_uppercase();

    info!(
        "Creating operator with login_name: {}, display_name: {}, role: {}",
        normalized_login, display_name, role
    );

    // Hash the password using bcrypt
    let password_hash: String = bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| PersistenceError::Other(format!("Failed to hash password: {e}")))?;

    diesel::insert_into(operators::table)
        .values((
            operators::login_name.eq(&normalized_login),
            operators::display_name.eq(display_name),
            operators::password_hash.eq(&password_hash),
            operators::role.eq(role),
        ))
        .execute(conn)?;

    let operator_id: i64 = conn.get_last_insert_rowid()?;

    info!(operator_id, "Operator created successfully");
    info!("Created operator with ID: {}", operator_id);

    Ok(operator_id)
}
}

backend_fn! {
/// Updates the last login timestamp for an operator.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `operator_id` - The operator ID
///
/// # Errors
///
/// Returns an error if the database update fails.
pub fn update_last_login(conn: &mut _, operator_id: i64) -> Result<(), PersistenceError> {
    debug!("Updating last_login_at for operator ID: {}", operator_id);

    diesel::update(operators::table)
        .filter(operators::operator_id.eq(operator_id))
        .set(operators::last_login_at.eq(diesel::dsl::sql::<
            diesel::sql_types::Nullable<diesel::sql_types::Text>,
        >("CURRENT_TIMESTAMP")))
        .execute(conn)?;

    Ok(())
}
}

backend_fn! {
/// Disables an operator.
///
/// This sets `is_disabled` to true and records the `disabled_at` timestamp.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `operator_id` - The operator ID
///
/// # Errors
///
/// Returns an error if the database update fails.
pub fn disable_operator(conn: &mut _, operator_id: i64) -> Result<(), PersistenceError> {
    info!("Disabling operator ID: {}", operator_id);

    diesel::update(operators::table)
        .filter(operators::operator_id.eq(operator_id))
        .set((
            operators::is_disabled.eq(1),
            operators::disabled_at.eq(diesel::dsl::sql::<
                diesel::sql_types::Nullable<diesel::sql_types::Text>,
            >("CURRENT_TIMESTAMP")),
        ))
        .execute(conn)?;

    Ok(())
}
}

backend_fn! {
/// Re-enables a disabled operator.
///
/// This sets `is_disabled` to false and clears the `disabled_at` timestamp.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `operator_id` - The operator ID
///
/// # Errors
///
/// Returns an error if the database update fails.
pub fn enable_operator(conn: &mut _, operator_id: i64) -> Result<(), PersistenceError> {
    info!("Re-enabling operator ID: {}", operator_id);

    diesel::update(operators::table)
        .filter(operators::operator_id.eq(operator_id))
        .set((
            operators::is_disabled.eq(0),
            operators::disabled_at.eq(None::<String>),
        ))
        .execute(conn)?;

    Ok(())
}
}

/// Deletes an operator if they are not referenced by any audit events (`SQLite` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `operator_id` - The operator ID
///
/// # Errors
///
/// Returns an error if:
/// - The operator is referenced by audit events
/// - The operator does not exist
/// - The database operation fails
pub fn delete_operator_sqlite(
    conn: &mut SqliteConnection,
    operator_id: i64,
) -> Result<(), PersistenceError> {
    info!("Attempting to delete operator ID: {}", operator_id);

    // Check if operator is referenced by audit events
    if is_operator_referenced_sqlite(conn, operator_id)? {
        return Err(PersistenceError::OperatorReferenced { operator_id });
    }

    // Attempt deletion
    let rows_affected: usize = diesel::delete(operators::table)
        .filter(operators::operator_id.eq(operator_id))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::OperatorNotFound(format!(
            "Operator with ID {operator_id} not found"
        )));
    }

    info!("Deleted operator ID: {}", operator_id);
    Ok(())
}

/// Deletes an operator if they are not referenced by any audit events (`MySQL` version).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `operator_id` - The operator ID
///
/// # Errors
///
/// Returns an error if:
/// - The operator is referenced by audit events
/// - The operator does not exist
/// - The database operation fails
pub fn delete_operator_mysql(
    conn: &mut MysqlConnection,
    operator_id: i64,
) -> Result<(), PersistenceError> {
    info!("Attempting to delete operator ID: {}", operator_id);

    // Check if operator is referenced by audit events
    if is_operator_referenced_mysql(conn, operator_id)? {
        return Err(PersistenceError::OperatorReferenced { operator_id });
    }

    // Attempt deletion
    let rows_affected: usize = diesel::delete(operators::table)
        .filter(operators::operator_id.eq(operator_id))
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::OperatorNotFound(format!(
            "Operator with ID {operator_id} not found"
        )));
    }

    info!("Deleted operator ID: {}", operator_id);
    Ok(())
}

backend_fn! {
/// Creates a new session for an operator.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `session_token` - The unique session token
/// * `operator_id` - The operator ID
/// * `expires_at` - The expiration timestamp (ISO 8601 format)
///
/// # Errors
///
/// Returns an error if the session cannot be created.
pub fn create_session(
    conn: &mut _,
    session_token: &str,
    operator_id: i64,
    expires_at: &str,
) -> Result<i64, PersistenceError> {
    debug!(
        "Creating session for operator ID: {} with expiration: {}",
        operator_id, expires_at
    );

    diesel::insert_into(sessions::table)
        .values((
            sessions::session_token.eq(session_token),
            sessions::operator_id.eq(operator_id),
            sessions::expires_at.eq(expires_at),
        ))
        .execute(conn)?;

    let session_id: i64 = conn.get_last_insert_rowid()?;

    debug!(session_id, operator_id, "Session created");
    debug!("Created session with ID: {}", session_id);
    Ok(session_id)
}
}

backend_fn! {
/// Updates the last activity timestamp for a session.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `session_id` - The session ID
///
/// # Errors
///
/// Returns an error if the database update fails.
pub fn update_session_activity(conn: &mut _, session_id: i64) -> Result<(), PersistenceError> {
    debug!("Updating last_activity_at for session ID: {}", session_id);

    diesel::update(sessions::table)
        .filter(sessions::session_id.eq(session_id))
        .set(
            sessions::last_activity_at.eq(diesel::dsl::sql::<diesel::sql_types::Text>(
                "CURRENT_TIMESTAMP",
            )),
        )
        .execute(conn)?;

    Ok(())
}
}

backend_fn! {
/// Deletes a session by token.
///
/// This is used for logout operations.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `session_token` - The session token to delete
///
/// # Errors
///
/// Returns an error if the database delete fails.
pub fn delete_session(conn: &mut _, session_token: &str) -> Result<(), PersistenceError> {
    debug!("Deleting session by token");

    diesel::delete(sessions::table)
        .filter(sessions::session_token.eq(session_token))
        .execute(conn)?;

    Ok(())
}
}

backend_fn! {
/// Deletes all expired sessions.
///
/// This is a cleanup operation that should be run periodically.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database delete fails.
pub fn delete_expired_sessions(conn: &mut _) -> Result<usize, PersistenceError> {
    debug!("Deleting expired sessions");

    let rows_affected: usize = diesel::delete(sessions::table)
        .filter(
            sessions::expires_at.lt(diesel::dsl::sql::<diesel::sql_types::Text>(
                "CURRENT_TIMESTAMP",
            )),
        )
        .execute(conn)?;

    info!("Deleted {} expired sessions", rows_affected);
    Ok(rows_affected)
}
}

backend_fn! {
/// Updates an operator's password.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `operator_id` - The operator ID
/// * `new_password` - The new password (will be hashed)
///
/// # Errors
///
/// Returns an error if the password cannot be hashed or the update fails.
pub fn update_password(
    conn: &mut _,
    operator_id: i64,
    new_password: &str,
) -> Result<(), PersistenceError> {
    info!("Updating password for operator ID: {}", operator_id);

    // Hash the new password using bcrypt
    let password_hash: String = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| PersistenceError::Other(format!("Failed to hash password: {e}")))?;

    diesel::update(operators::table)
        .filter(operators::operator_id.eq(operator_id))
        .set(operators::password_hash.eq(&password_hash))
        .execute(conn)?;

    info!("Password updated for operator ID: {}", operator_id);
    Ok(())
}
}

backend_fn! {
/// Deletes all sessions for a specific operator.
///
/// This is used when an operator's password is changed to invalidate all active sessions.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `operator_id` - The operator ID whose sessions should be deleted
///
/// # Errors
///
/// Returns an error if the database delete fails.
pub fn delete_sessions_for_operator(
    conn: &mut _,
    operator_id: i64,
) -> Result<usize, PersistenceError> {
    info!("Deleting all sessions for operator ID: {}", operator_id);

    let rows_affected: usize = diesel::delete(sessions::table)
        .filter(sessions::operator_id.eq(operator_id))
        .execute(conn)?;

    info!(
        "Deleted {} sessions for operator ID: {}",
        rows_affected, operator_id
    );
    Ok(rows_affected)
}
}
