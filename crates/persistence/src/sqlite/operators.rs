// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Operator and session persistence functions.

use rusqlite::{Connection, OptionalExtension, params};
use tracing::{debug, info};

use crate::data_models::{OperatorData, SessionData};
use crate::error::PersistenceError;

/// Creates a new operator.
///
/// The `login_name` is normalized to uppercase for case-insensitive uniqueness.
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `login_name` - The login name (will be normalized)
/// * `display_name` - The display name
/// * `role` - The role (Admin or Bidder)
///
/// # Errors
///
/// Returns an error if the operator cannot be created or if the login name
/// already exists.
pub fn create_operator(
    conn: &Connection,
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

    conn.execute(
        "INSERT INTO operators (login_name, display_name, password_hash, role)
         VALUES (?1, ?2, ?3, ?4)",
        params![normalized_login, display_name, password_hash, role],
    )?;

    let operator_id: i64 = conn.last_insert_rowid();
    info!("Created operator with ID: {}", operator_id);

    Ok(operator_id)
}

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
    conn: &Connection,
    login_name: &str,
) -> Result<Option<OperatorData>, PersistenceError> {
    let normalized_login: String = login_name.to_uppercase();

    debug!("Looking up operator by login_name: {}", normalized_login);

    let result: Option<OperatorData> = conn
        .query_row(
            "SELECT operator_id, login_name, display_name, password_hash, role, is_disabled,
                    created_at, disabled_at, last_login_at
             FROM operators
             WHERE login_name = ?1",
            params![normalized_login],
            |row| {
                Ok(OperatorData {
                    operator_id: row.get(0)?,
                    login_name: row.get(1)?,
                    display_name: row.get(2)?,
                    password_hash: row.get(3)?,
                    role: row.get(4)?,
                    is_disabled: row.get::<_, i32>(5)? != 0,
                    created_at: row.get(6)?,
                    disabled_at: row.get(7)?,
                    last_login_at: row.get(8)?,
                })
            },
        )
        .optional()?;

    Ok(result)
}

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
    conn: &Connection,
    operator_id: i64,
) -> Result<Option<OperatorData>, PersistenceError> {
    debug!("Looking up operator by ID: {}", operator_id);

    let result: Option<OperatorData> = conn
        .query_row(
            "SELECT operator_id, login_name, display_name, password_hash, role, is_disabled,
                    created_at, disabled_at, last_login_at
             FROM operators
             WHERE operator_id = ?1",
            params![operator_id],
            |row| {
                Ok(OperatorData {
                    operator_id: row.get(0)?,
                    login_name: row.get(1)?,
                    display_name: row.get(2)?,
                    password_hash: row.get(3)?,
                    role: row.get(4)?,
                    is_disabled: row.get::<_, i32>(5)? != 0,
                    created_at: row.get(6)?,
                    disabled_at: row.get(7)?,
                    last_login_at: row.get(8)?,
                })
            },
        )
        .optional()?;

    Ok(result)
}

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
pub fn update_last_login(conn: &Connection, operator_id: i64) -> Result<(), PersistenceError> {
    debug!("Updating last_login_at for operator ID: {}", operator_id);

    conn.execute(
        "UPDATE operators SET last_login_at = CURRENT_TIMESTAMP WHERE operator_id = ?1",
        params![operator_id],
    )?;

    Ok(())
}

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
pub fn disable_operator(conn: &Connection, operator_id: i64) -> Result<(), PersistenceError> {
    info!("Disabling operator ID: {}", operator_id);

    conn.execute(
        "UPDATE operators
         SET is_disabled = 1, disabled_at = CURRENT_TIMESTAMP
         WHERE operator_id = ?1",
        params![operator_id],
    )?;

    Ok(())
}

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
    conn: &Connection,
    session_token: &str,
    operator_id: i64,
    expires_at: &str,
) -> Result<i64, PersistenceError> {
    debug!(
        "Creating session for operator ID: {} with expiration: {}",
        operator_id, expires_at
    );

    conn.execute(
        "INSERT INTO sessions (session_token, operator_id, expires_at)
         VALUES (?1, ?2, ?3)",
        params![session_token, operator_id, expires_at],
    )?;

    let session_id: i64 = conn.last_insert_rowid();
    debug!("Created session with ID: {}", session_id);

    Ok(session_id)
}

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
    conn: &Connection,
    session_token: &str,
) -> Result<Option<SessionData>, PersistenceError> {
    debug!("Looking up session by token");

    let result: Option<SessionData> = conn
        .query_row(
            "SELECT session_id, session_token, operator_id, created_at,
                    last_activity_at, expires_at
             FROM sessions
             WHERE session_token = ?1",
            params![session_token],
            |row| {
                Ok(SessionData {
                    session_id: row.get(0)?,
                    session_token: row.get(1)?,
                    operator_id: row.get(2)?,
                    created_at: row.get(3)?,
                    last_activity_at: row.get(4)?,
                    expires_at: row.get(5)?,
                })
            },
        )
        .optional()?;

    Ok(result)
}

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
pub fn update_session_activity(conn: &Connection, session_id: i64) -> Result<(), PersistenceError> {
    debug!("Updating last_activity_at for session ID: {}", session_id);

    conn.execute(
        "UPDATE sessions SET last_activity_at = CURRENT_TIMESTAMP WHERE session_id = ?1",
        params![session_id],
    )?;

    Ok(())
}

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
pub fn delete_session(conn: &Connection, session_token: &str) -> Result<(), PersistenceError> {
    debug!("Deleting session by token");

    conn.execute(
        "DELETE FROM sessions WHERE session_token = ?1",
        params![session_token],
    )?;

    Ok(())
}

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
pub fn delete_expired_sessions(conn: &Connection) -> Result<usize, PersistenceError> {
    debug!("Deleting expired sessions");

    let rows_affected: usize = conn.execute(
        "DELETE FROM sessions WHERE expires_at < CURRENT_TIMESTAMP",
        [],
    )?;

    info!("Deleted {} expired sessions", rows_affected);
    Ok(rows_affected)
}

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
pub fn is_operator_referenced(
    conn: &Connection,
    operator_id: i64,
) -> Result<bool, PersistenceError> {
    debug!(
        "Checking if operator ID {} is referenced in audit events",
        operator_id
    );

    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audit_events WHERE actor_operator_id = ?1",
        params![operator_id],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

/// Lists all operators.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database query fails.
pub fn list_operators(conn: &Connection) -> Result<Vec<OperatorData>, PersistenceError> {
    debug!("Listing all operators");

    let mut stmt = conn.prepare(
        "SELECT operator_id, login_name, display_name, password_hash, role, is_disabled,
                created_at, disabled_at, last_login_at
         FROM operators
         ORDER BY login_name ASC",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(OperatorData {
            operator_id: row.get(0)?,
            login_name: row.get(1)?,
            display_name: row.get(2)?,
            password_hash: row.get(3)?,
            role: row.get(4)?,
            is_disabled: row.get::<_, i32>(5)? != 0,
            created_at: row.get(6)?,
            disabled_at: row.get(7)?,
            last_login_at: row.get(8)?,
        })
    })?;

    let mut operators: Vec<OperatorData> = Vec::new();
    for row_result in rows {
        operators.push(row_result?);
    }

    Ok(operators)
}

/// Verifies a password against a stored hash.
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

/// Counts the total number of operators.
///
/// # Arguments
///
/// * `conn` - The database connection
///
/// # Errors
///
/// Returns an error if the database query fails.
pub fn count_operators(conn: &Connection) -> Result<i64, PersistenceError> {
    debug!("Counting operators");

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM operators", [], |row| row.get(0))?;

    debug!("Total operators: {}", count);
    Ok(count)
}
