// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Operator and session persistence functions.

use diesel::sql_types::{BigInt, Integer, Nullable, Text};
use diesel::{RunQueryDsl, SqliteConnection, sql_query};
use tracing::{debug, info};

use crate::data_models::{OperatorData, SessionData};
use crate::error::PersistenceError;

// Helper structs for Diesel query results
#[derive(diesel::QueryableByName)]
struct LastInsertRowid {
    #[diesel(sql_type = BigInt)]
    last_insert_rowid: i64,
}

#[derive(diesel::QueryableByName)]
struct OperatorRow {
    #[diesel(sql_type = BigInt)]
    operator_id: i64,
    #[diesel(sql_type = Text)]
    login_name: String,
    #[diesel(sql_type = Text)]
    display_name: String,
    #[diesel(sql_type = Text)]
    password_hash: String,
    #[diesel(sql_type = Text)]
    role: String,
    #[diesel(sql_type = Integer)]
    is_disabled: i32,
    #[diesel(sql_type = Text)]
    created_at: String,
    #[diesel(sql_type = Nullable<Text>)]
    disabled_at: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    last_login_at: Option<String>,
}

#[derive(diesel::QueryableByName)]
struct SessionRow {
    #[diesel(sql_type = BigInt)]
    session_id: i64,
    #[diesel(sql_type = BigInt)]
    operator_id: i64,
    #[diesel(sql_type = Text)]
    session_token: String,
    #[diesel(sql_type = Text)]
    created_at: String,
    #[diesel(sql_type = Text)]
    expires_at: String,
    #[diesel(sql_type = Text)]
    last_activity_at: String,
}

#[derive(diesel::QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

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
    conn: &mut SqliteConnection,
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

    sql_query(
        "INSERT INTO operators (login_name, display_name, password_hash, role)
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind::<Text, _>(&normalized_login)
    .bind::<Text, _>(display_name)
    .bind::<Text, _>(&password_hash)
    .bind::<Text, _>(role)
    .execute(conn)?;

    let operator_id: i64 = sql_query("SELECT last_insert_rowid() as last_insert_rowid")
        .get_result::<LastInsertRowid>(conn)?
        .last_insert_rowid;

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
    conn: &mut SqliteConnection,
    login_name: &str,
) -> Result<Option<OperatorData>, PersistenceError> {
    let normalized_login: String = login_name.to_uppercase();

    debug!("Looking up operator by login_name: {}", normalized_login);

    let result: Result<OperatorRow, diesel::result::Error> = sql_query(
        "SELECT operator_id, login_name, display_name, password_hash, role, is_disabled,
                created_at, disabled_at, last_login_at
         FROM operators
         WHERE login_name = ?1",
    )
    .bind::<diesel::sql_types::Text, _>(&normalized_login)
    .get_result::<OperatorRow>(conn);

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
    conn: &mut SqliteConnection,
    operator_id: i64,
) -> Result<Option<OperatorData>, PersistenceError> {
    debug!("Looking up operator by ID: {}", operator_id);

    let result: Result<OperatorRow, diesel::result::Error> = sql_query(
        "SELECT operator_id, login_name, display_name, password_hash, role, is_disabled,
                created_at, disabled_at, last_login_at
         FROM operators
         WHERE operator_id = ?1",
    )
    .bind::<diesel::sql_types::BigInt, _>(operator_id)
    .get_result::<OperatorRow>(conn);

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
pub fn update_last_login(
    conn: &mut SqliteConnection,
    operator_id: i64,
) -> Result<(), PersistenceError> {
    debug!("Updating last_login_at for operator ID: {}", operator_id);

    sql_query("UPDATE operators SET last_login_at = CURRENT_TIMESTAMP WHERE operator_id = ?1")
        .bind::<diesel::sql_types::BigInt, _>(operator_id)
        .execute(conn)?;

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
pub fn disable_operator(
    conn: &mut SqliteConnection,
    operator_id: i64,
) -> Result<(), PersistenceError> {
    info!("Disabling operator ID: {}", operator_id);

    sql_query(
        "UPDATE operators
         SET is_disabled = 1, disabled_at = CURRENT_TIMESTAMP
         WHERE operator_id = ?1",
    )
    .bind::<diesel::sql_types::BigInt, _>(operator_id)
    .execute(conn)?;

    Ok(())
}

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
pub fn enable_operator(
    conn: &mut SqliteConnection,
    operator_id: i64,
) -> Result<(), PersistenceError> {
    info!("Re-enabling operator ID: {}", operator_id);

    sql_query(
        "UPDATE operators
         SET is_disabled = 0, disabled_at = NULL
         WHERE operator_id = ?1",
    )
    .bind::<diesel::sql_types::BigInt, _>(operator_id)
    .execute(conn)?;

    Ok(())
}

/// Deletes an operator.
///
/// This operation will fail if the operator is referenced by any audit events,
/// enforced by the foreign key constraint (ON DELETE RESTRICT).
///
/// # Arguments
///
/// * `conn` - The database connection
/// * `operator_id` - The operator ID
///
/// # Errors
///
/// Returns `PersistenceError::OperatorReferenced` if the operator is referenced
/// by audit events. Returns other errors if the database delete fails.
pub fn delete_operator(
    conn: &mut SqliteConnection,
    operator_id: i64,
) -> Result<(), PersistenceError> {
    info!("Attempting to delete operator ID: {}", operator_id);

    // Check if operator is referenced by audit events
    if is_operator_referenced(conn, operator_id)? {
        return Err(PersistenceError::OperatorReferenced { operator_id });
    }

    // Attempt deletion
    let rows_affected: usize = sql_query("DELETE FROM operators WHERE operator_id = ?1")
        .bind::<diesel::sql_types::BigInt, _>(operator_id)
        .execute(conn)?;

    if rows_affected == 0 {
        return Err(PersistenceError::OperatorNotFound(format!(
            "Operator with ID {operator_id} not found"
        )));
    }

    info!("Deleted operator ID: {}", operator_id);
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
    conn: &mut SqliteConnection,
    session_token: &str,
    operator_id: i64,
    expires_at: &str,
) -> Result<i64, PersistenceError> {
    debug!(
        "Creating session for operator ID: {} with expiration: {}",
        operator_id, expires_at
    );

    sql_query(
        "INSERT INTO sessions (session_token, operator_id, expires_at)
         VALUES (?1, ?2, ?3)",
    )
    .bind::<diesel::sql_types::Text, _>(session_token)
    .bind::<diesel::sql_types::BigInt, _>(operator_id)
    .bind::<diesel::sql_types::Text, _>(expires_at)
    .execute(conn)?;

    let session_id: i64 = sql_query("SELECT last_insert_rowid() as last_insert_rowid")
        .get_result::<LastInsertRowid>(conn)?
        .last_insert_rowid;

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
    conn: &mut SqliteConnection,
    session_token: &str,
) -> Result<Option<SessionData>, PersistenceError> {
    debug!("Looking up session by token");

    let result: Result<SessionRow, diesel::result::Error> = sql_query(
        "SELECT session_id, session_token, operator_id, created_at,
                last_activity_at, expires_at
         FROM sessions
         WHERE session_token = ?1",
    )
    .bind::<diesel::sql_types::Text, _>(session_token)
    .get_result::<SessionRow>(conn);

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
pub fn update_session_activity(
    conn: &mut SqliteConnection,
    session_id: i64,
) -> Result<(), PersistenceError> {
    debug!("Updating last_activity_at for session ID: {}", session_id);

    sql_query("UPDATE sessions SET last_activity_at = CURRENT_TIMESTAMP WHERE session_id = ?1")
        .bind::<diesel::sql_types::BigInt, _>(session_id)
        .execute(conn)?;

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
pub fn delete_session(
    conn: &mut SqliteConnection,
    session_token: &str,
) -> Result<(), PersistenceError> {
    debug!("Deleting session by token");

    sql_query("DELETE FROM sessions WHERE session_token = ?1")
        .bind::<diesel::sql_types::Text, _>(session_token)
        .execute(conn)?;

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
pub fn delete_expired_sessions(conn: &mut SqliteConnection) -> Result<usize, PersistenceError> {
    debug!("Deleting expired sessions");

    let rows_affected: usize =
        sql_query("DELETE FROM sessions WHERE expires_at < CURRENT_TIMESTAMP").execute(conn)?;

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
    conn: &mut SqliteConnection,
    operator_id: i64,
) -> Result<bool, PersistenceError> {
    debug!(
        "Checking if operator ID {} is referenced in audit events",
        operator_id
    );

    let count: i64 =
        sql_query("SELECT COUNT(*) as count FROM audit_events WHERE actor_operator_id = ?1")
            .bind::<diesel::sql_types::BigInt, _>(operator_id)
            .get_result::<CountRow>(conn)?
            .count;

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
pub fn list_operators(conn: &mut SqliteConnection) -> Result<Vec<OperatorData>, PersistenceError> {
    debug!("Listing all operators");

    let rows: Vec<OperatorRow> = sql_query(
        "SELECT operator_id, login_name, display_name, password_hash, role, is_disabled,
                created_at, disabled_at, last_login_at
         FROM operators
         ORDER BY login_name ASC",
    )
    .load::<OperatorRow>(conn)?;

    let operators: Vec<OperatorData> = rows
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
    conn: &mut SqliteConnection,
    operator_id: i64,
    new_password: &str,
) -> Result<(), PersistenceError> {
    info!("Updating password for operator ID: {}", operator_id);

    // Hash the new password using bcrypt
    let password_hash: String = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| PersistenceError::Other(format!("Failed to hash password: {e}")))?;

    sql_query("UPDATE operators SET password_hash = ?1 WHERE operator_id = ?2")
        .bind::<diesel::sql_types::Text, _>(&password_hash)
        .bind::<diesel::sql_types::BigInt, _>(operator_id)
        .execute(conn)?;

    info!("Password updated for operator ID: {}", operator_id);
    Ok(())
}

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
    conn: &mut SqliteConnection,
    operator_id: i64,
) -> Result<usize, PersistenceError> {
    info!("Deleting all sessions for operator ID: {}", operator_id);

    let rows_affected: usize = sql_query("DELETE FROM sessions WHERE operator_id = ?1")
        .bind::<diesel::sql_types::BigInt, _>(operator_id)
        .execute(conn)?;

    info!(
        "Deleted {} sessions for operator ID: {}",
        rows_affected, operator_id
    );
    Ok(rows_affected)
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
pub fn count_operators(conn: &mut SqliteConnection) -> Result<i64, PersistenceError> {
    debug!("Counting operators");

    let count: i64 = sql_query("SELECT COUNT(*) as count FROM operators")
        .get_result::<CountRow>(conn)?
        .count;

    debug!("Total operators: {}", count);
    Ok(count)
}

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
pub fn count_active_admin_operators(conn: &mut SqliteConnection) -> Result<i64, PersistenceError> {
    debug!("Counting active admin operators");

    let count: i64 = sql_query(
        "SELECT COUNT(*) as count FROM operators WHERE role = 'Admin' AND is_disabled = 0",
    )
    .get_result::<CountRow>(conn)?
    .count;

    debug!("Active admin operators: {}", count);
    Ok(count)
}
