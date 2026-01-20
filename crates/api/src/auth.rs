// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Authentication and authorization types and services.

use time::{Duration, OffsetDateTime};
use zab_bid_audit::Actor;
use zab_bid_persistence::{OperatorData, PersistenceError, SessionData, SqlitePersistence};

use crate::error::AuthError;

/// Actor roles for authorization.
///
/// Roles determine what actions an authenticated actor may perform.
/// Roles apply only to actors (system operators), never to domain users.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Admin role: system operators with structural and corrective authority.
    ///
    /// Admins may perform:
    /// - creation and modification of bid years, areas, and users
    /// - rollback operations
    /// - checkpoint creation
    /// - round finalization and similar milestone actions
    /// - any other system-level or corrective actions
    Admin,
    /// Bidder role: operators authorized to perform bidding actions.
    ///
    /// Bidders may:
    /// - enter new bids
    /// - modify existing bids
    /// - withdraw or correct bids
    /// - perform bidding actions on behalf of any domain user
    ///
    /// Bidders are not domain users. They are trusted operators entering
    /// data provided by many users.
    Bidder,
}

/// An authenticated actor with an associated role.
///
/// This represents a system operator who has been authenticated and
/// has permission to perform certain actions based on their role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedActor {
    /// The unique identifier for this actor.
    pub id: String,
    /// The role assigned to this actor.
    pub role: Role,
}

impl AuthenticatedActor {
    /// Creates a new authenticated actor.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique identifier for this actor
    /// * `role` - The role assigned to this actor
    #[must_use]
    pub const fn new(id: String, role: Role) -> Self {
        Self { id, role }
    }

    /// Converts this authenticated actor into an audit Actor with operator information.
    ///
    /// This is used when recording audit events to attribute actions
    /// to the authenticated operator.
    ///
    /// # Arguments
    ///
    /// * `operator` - The operator data containing stable snapshot fields
    #[must_use]
    pub fn to_audit_actor(&self, operator: &OperatorData) -> Actor {
        let actor_type: String = match self.role {
            Role::Admin => String::from("admin"),
            Role::Bidder => String::from("bidder"),
        };
        Actor::with_operator(
            self.id.clone(),
            actor_type,
            operator.operator_id,
            operator.login_name.clone(),
            operator.display_name.clone(),
        )
    }
}

/// Authorization service for enforcing role-based access control.
///
/// This service determines whether an authenticated actor has permission
/// to perform a specific action based on their role.
pub struct AuthorizationService;

impl AuthorizationService {
    /// Checks if an actor is authorized to register a user.
    ///
    /// Only Admin actors may register users.
    ///
    /// # Arguments
    ///
    /// * `actor` - The authenticated actor
    ///
    /// # Errors
    ///
    /// Returns an error if the actor does not have the Admin role.
    pub fn authorize_register_user(actor: &AuthenticatedActor) -> Result<(), AuthError> {
        match actor.role {
            Role::Admin => Ok(()),
            Role::Bidder => Err(AuthError::Unauthorized {
                action: String::from("register_user"),
                required_role: String::from("Admin"),
            }),
        }
    }

    /// Checks if an actor is authorized to create a bid year.
    ///
    /// Only Admin actors may create bid years.
    ///
    /// # Arguments
    ///
    /// * `actor` - The authenticated actor
    ///
    /// # Errors
    ///
    /// Returns an error if the actor does not have the Admin role.
    pub fn authorize_create_bid_year(actor: &AuthenticatedActor) -> Result<(), AuthError> {
        match actor.role {
            Role::Admin => Ok(()),
            Role::Bidder => Err(AuthError::Unauthorized {
                action: String::from("create_bid_year"),
                required_role: String::from("Admin"),
            }),
        }
    }

    /// Checks if an actor is authorized to create an area.
    ///
    /// Only Admin actors may create areas.
    ///
    /// # Arguments
    ///
    /// * `actor` - The authenticated actor
    ///
    /// # Errors
    ///
    /// Returns an error if the actor does not have the Admin role.
    pub fn authorize_create_area(actor: &AuthenticatedActor) -> Result<(), AuthError> {
        match actor.role {
            Role::Admin => Ok(()),
            Role::Bidder => Err(AuthError::Unauthorized {
                action: String::from("create_area"),
                required_role: String::from("Admin"),
            }),
        }
    }

    /// Checks if an actor is authorized to reassign a user's crew.
    ///
    /// Both Admin and Bidder actors may reassign crews.
    ///
    /// # Arguments
    ///
    /// * `actor` - The authenticated actor
    ///
    /// # Errors
    ///
    /// Returns an error if the actor does not have permission.
    pub const fn authorize_reassign_crew(_actor: &AuthenticatedActor) -> Result<(), AuthError> {
        // Both Admin and Bidder may reassign crews
        Ok(())
    }

    /// Checks if an actor is authorized to create a checkpoint.
    ///
    /// Only Admin actors may create checkpoints.
    ///
    /// # Arguments
    ///
    /// * `actor` - The authenticated actor
    ///
    /// # Errors
    ///
    /// Returns an error if the actor does not have the Admin role.
    pub fn authorize_checkpoint(actor: &AuthenticatedActor) -> Result<(), AuthError> {
        match actor.role {
            Role::Admin => Ok(()),
            Role::Bidder => Err(AuthError::Unauthorized {
                action: String::from("checkpoint"),
                required_role: String::from("Admin"),
            }),
        }
    }

    /// Checks if an actor is authorized to finalize a round.
    ///
    /// Only Admin actors may finalize rounds.
    ///
    /// # Arguments
    ///
    /// * `actor` - The authenticated actor
    ///
    /// # Errors
    ///
    /// Returns an error if the actor does not have the Admin role.
    pub fn authorize_finalize(actor: &AuthenticatedActor) -> Result<(), AuthError> {
        match actor.role {
            Role::Admin => Ok(()),
            Role::Bidder => Err(AuthError::Unauthorized {
                action: String::from("finalize"),
                required_role: String::from("Admin"),
            }),
        }
    }

    /// Checks if an actor is authorized to rollback to a specific event.
    ///
    /// Only Admin actors may perform rollback operations.
    ///
    /// # Arguments
    ///
    /// * `actor` - The authenticated actor
    ///
    /// # Errors
    ///
    /// Returns an error if the actor does not have the Admin role.
    pub fn authorize_rollback(actor: &AuthenticatedActor) -> Result<(), AuthError> {
        match actor.role {
            Role::Admin => Ok(()),
            Role::Bidder => Err(AuthError::Unauthorized {
                action: String::from("rollback"),
                required_role: String::from("Admin"),
            }),
        }
    }
}

/// Authentication service for session-based authentication (Phase 14).
pub struct AuthenticationService;

impl AuthenticationService {
    /// Default session expiration duration (30 days).
    const DEFAULT_SESSION_EXPIRATION: Duration = Duration::days(30);

    /// Authenticates an operator and creates a session.
    ///
    /// Validates the operator exists, is not disabled, and verifies the password.
    ///
    /// # Arguments
    ///
    /// * `persistence` - The persistence layer
    /// * `login_name` - The operator login name
    /// * `password` - The operator password
    ///
    /// # Returns
    ///
    /// A tuple of (`session_token`, `authenticated_actor`, `operator_data`)
    ///
    /// # Errors
    ///
    /// Returns an error if authentication fails.
    pub fn login(
        persistence: &mut SqlitePersistence,
        login_name: &str,
        password: &str,
    ) -> Result<(String, AuthenticatedActor, OperatorData), AuthError> {
        // Retrieve operator by login name
        let operator: OperatorData = persistence
            .get_operator_by_login(login_name)
            .map_err(|e| {
                tracing::warn!(login_name, error = %e, "Database error during login");
                AuthError::AuthenticationFailed {
                    reason: String::from("invalid_credentials"),
                }
            })?
            .ok_or_else(|| {
                tracing::debug!(login_name, "Unknown operator attempted login");
                AuthError::AuthenticationFailed {
                    reason: String::from("invalid_credentials"),
                }
            })?;

        // Check if operator is disabled
        if operator.is_disabled {
            tracing::info!(login_name = %operator.login_name, operator_id = operator.operator_id, "Disabled operator attempted login");
            return Err(AuthError::AuthenticationFailed {
                reason: String::from("invalid_credentials"),
            });
        }

        // Verify password
        let password_valid: bool =
            persistence.verify_password(password, &operator.password_hash).map_err(|e| {
                tracing::warn!(login_name = %operator.login_name, error = %e, "Password verification error");
                AuthError::AuthenticationFailed {
                    reason: String::from("invalid_credentials"),
                }
            })?;

        if !password_valid {
            tracing::info!(login_name = %operator.login_name, "Invalid password attempt");
            return Err(AuthError::AuthenticationFailed {
                reason: String::from("invalid_credentials"),
            });
        }

        // Parse role
        let role: Role = match operator.role.as_str() {
            "Admin" => Role::Admin,
            "Bidder" => Role::Bidder,
            _ => {
                return Err(AuthError::AuthenticationFailed {
                    reason: format!("Invalid role: {}", operator.role),
                });
            }
        };

        // Generate session token
        let session_token: String = Self::generate_session_token();

        // Calculate expiration time
        let expires_at: OffsetDateTime =
            OffsetDateTime::now_utc() + Self::DEFAULT_SESSION_EXPIRATION;

        // Format with microsecond precision for MySQL compatibility
        // MySQL DATETIME supports up to 6 decimal places (microseconds), not 9 (nanoseconds)
        let expires_at_str: String = format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
            expires_at.year(),
            u8::from(expires_at.month()),
            expires_at.day(),
            expires_at.hour(),
            expires_at.minute(),
            expires_at.second(),
            expires_at.nanosecond() / 1000 // Convert nanoseconds to microseconds
        );

        // Create session
        persistence
            .create_session(&session_token, operator.operator_id, &expires_at_str)
            .map_err(|e| {
                tracing::error!(operator_id = operator.operator_id, error = %e, "Failed to create session");
                AuthError::AuthenticationFailed {
                    reason: String::from("invalid_credentials"),
                }
            })?;

        // Update last login timestamp
        let _ = persistence
            .update_last_login(operator.operator_id)
            .map_err(|e| {
                tracing::warn!(operator_id = operator.operator_id, error = %e, "Failed to update last login timestamp");
                // Don't fail auth if we can't update the timestamp
            });

        let authenticated_actor: AuthenticatedActor =
            AuthenticatedActor::new(operator.login_name.clone(), role);

        Ok((session_token, authenticated_actor, operator))
    }

    /// Validates a session token and returns the authenticated actor.
    ///
    /// # Arguments
    ///
    /// * `persistence` - The persistence layer
    /// * `session_token` - The session token to validate
    ///
    /// # Returns
    ///
    /// A tuple of (`authenticated_actor`, `operator_data`)
    ///
    /// # Errors
    ///
    /// Returns an error if the session is invalid or expired.
    pub fn validate_session(
        persistence: &mut SqlitePersistence,
        session_token: &str,
    ) -> Result<(AuthenticatedActor, OperatorData), AuthError> {
        // Retrieve session
        let session: SessionData = persistence
            .get_session_by_token(session_token)
            .map_err(Self::map_persistence_error)?
            .ok_or_else(|| AuthError::AuthenticationFailed {
                reason: String::from("Invalid session token"),
            })?;

        // Check if session is expired
        // Parse SQL datetime format with optional microseconds
        // MySQL DATETIME stores as "YYYY-MM-DD HH:MM:SS" (no fractional seconds without DATETIME(6))
        // SQLite and MySQL DATETIME(6) store as "YYYY-MM-DD HH:MM:SS.uuuuuu"
        let expires_at: OffsetDateTime = if session.expires_at.contains('.') {
            // Has microseconds
            let format = time::format_description::parse(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]",
            )
            .map_err(|e| AuthError::AuthenticationFailed {
                reason: format!("Failed to create datetime format: {e}"),
            })?;
            time::PrimitiveDateTime::parse(&session.expires_at, &format)
                .map_err(|e| AuthError::AuthenticationFailed {
                    reason: format!("Failed to parse session expiration: {e}"),
                })?
                .assume_utc()
        } else {
            // No microseconds
            let format =
                time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                    .map_err(|e| AuthError::AuthenticationFailed {
                        reason: format!("Failed to create datetime format: {e}"),
                    })?;
            time::PrimitiveDateTime::parse(&session.expires_at, &format)
                .map_err(|e| AuthError::AuthenticationFailed {
                    reason: format!("Failed to parse session expiration: {e}"),
                })?
                .assume_utc()
        };

        if OffsetDateTime::now_utc() > expires_at {
            return Err(AuthError::AuthenticationFailed {
                reason: String::from("Session expired"),
            });
        }

        // Retrieve operator
        let operator: OperatorData = persistence
            .get_operator_by_id(session.operator_id)
            .map_err(Self::map_persistence_error)?
            .ok_or_else(|| AuthError::AuthenticationFailed {
                reason: String::from("Operator not found"),
            })?;

        // Check if operator is disabled
        if operator.is_disabled {
            return Err(AuthError::AuthenticationFailed {
                reason: String::from("Operator is disabled"),
            });
        }

        // Parse role
        let role: Role = match operator.role.as_str() {
            "Admin" => Role::Admin,
            "Bidder" => Role::Bidder,
            _ => {
                return Err(AuthError::AuthenticationFailed {
                    reason: format!("Invalid role: {}", operator.role),
                });
            }
        };

        // Update session activity
        persistence
            .update_session_activity(session.session_id)
            .map_err(Self::map_persistence_error)?;

        let authenticated_actor: AuthenticatedActor =
            AuthenticatedActor::new(operator.login_name.clone(), role);

        Ok((authenticated_actor, operator))
    }

    /// Logs out by deleting the session.
    ///
    /// # Arguments
    ///
    /// * `persistence` - The persistence layer
    /// * `session_token` - The session token to delete
    ///
    /// # Errors
    ///
    /// Returns an error if the logout fails.
    pub fn logout(
        persistence: &mut SqlitePersistence,
        session_token: &str,
    ) -> Result<(), AuthError> {
        persistence
            .delete_session(session_token)
            .map_err(|e| AuthError::AuthenticationFailed {
                reason: format!("Failed to delete session: {e}"),
            })?;

        Ok(())
    }

    /// Generates a session token.
    ///
    /// In a production system, this would use a cryptographically secure
    /// random number generator. For simplicity, we use a timestamp-based
    /// approach here.
    fn generate_session_token() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp: u128 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_nanos();
        format!("session_{timestamp}_{}", rand::random::<u64>())
    }

    /// Maps persistence errors to authentication errors.
    fn map_persistence_error(err: PersistenceError) -> AuthError {
        match err {
            PersistenceError::SessionExpired(msg) | PersistenceError::SessionNotFound(msg) => {
                AuthError::AuthenticationFailed { reason: msg }
            }
            _ => AuthError::AuthenticationFailed {
                reason: format!("Database error: {err}"),
            },
        }
    }
}

/// Stub authentication function (deprecated in Phase 14).
///
/// This is kept for backward compatibility with tests but should not be
/// used in production code.
///
/// # Arguments
///
/// * `actor_id` - The identifier of the actor to authenticate
/// * `role` - The role to assign to the actor
///
/// # Returns
///
/// An authenticated actor if successful.
///
/// # Errors
///
/// Returns an error if authentication fails.
#[deprecated(since = "0.1.0", note = "Use AuthenticationService instead")]
pub fn authenticate_stub(actor_id: String, role: Role) -> Result<AuthenticatedActor, AuthError> {
    if actor_id.is_empty() {
        return Err(AuthError::AuthenticationFailed {
            reason: String::from("Actor ID cannot be empty"),
        });
    }
    Ok(AuthenticatedActor::new(actor_id, role))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use zab_bid_persistence::SqlitePersistence;

    fn create_test_persistence() -> SqlitePersistence {
        SqlitePersistence::new_in_memory().expect("Failed to create test database")
    }

    fn create_test_operator(
        persistence: &mut SqlitePersistence,
        login_name: &str,
        display_name: &str,
        password: &str,
        role: &str,
    ) -> i64 {
        persistence
            .create_operator(login_name, display_name, password, role)
            .expect("Failed to create operator")
    }

    fn create_admin_actor() -> AuthenticatedActor {
        AuthenticatedActor::new(String::from("admin_user"), Role::Admin)
    }

    fn create_bidder_actor() -> AuthenticatedActor {
        AuthenticatedActor::new(String::from("bidder_user"), Role::Bidder)
    }

    /// `PHASE_22.1`: Verify unknown operator returns generic error message
    #[test]
    fn test_login_unknown_operator_returns_generic_error() {
        let mut persistence = create_test_persistence();

        let result = AuthenticationService::login(&mut persistence, "nonexistent", "password");

        assert!(result.is_err());
        let err = result.unwrap_err();
        if let AuthError::AuthenticationFailed { reason } = err {
            assert_eq!(reason, "invalid_credentials");
        } else {
            panic!("Expected AuthenticationFailed error");
        }
    }

    /// `PHASE_22.1`: Verify incorrect password returns generic error message
    #[test]
    fn test_login_wrong_password_returns_generic_error() {
        let mut persistence = create_test_persistence();
        create_test_operator(
            &mut persistence,
            "testuser",
            "Test User",
            "correct_password",
            "Admin",
        );

        let result = AuthenticationService::login(&mut persistence, "testuser", "wrong_password");

        assert!(result.is_err());
        let err = result.unwrap_err();
        if let AuthError::AuthenticationFailed { reason } = err {
            assert_eq!(reason, "invalid_credentials");
        } else {
            panic!("Expected AuthenticationFailed error");
        }
    }

    /// `PHASE_22.1`: Verify disabled operator returns generic error message
    #[test]
    fn test_login_disabled_operator_returns_generic_error() {
        let mut persistence = create_test_persistence();
        let operator_id = create_test_operator(
            &mut persistence,
            "disabled_user",
            "Disabled User",
            "password",
            "Admin",
        );
        persistence
            .disable_operator(operator_id)
            .expect("Failed to disable operator");

        let result = AuthenticationService::login(&mut persistence, "disabled_user", "password");

        assert!(result.is_err());
        let err = result.unwrap_err();
        if let AuthError::AuthenticationFailed { reason } = err {
            assert_eq!(reason, "invalid_credentials");
        } else {
            panic!("Expected AuthenticationFailed error");
        }
    }

    /// `PHASE_22.1`: Verify successful login
    #[test]
    fn test_login_success() {
        let mut persistence = create_test_persistence();
        create_test_operator(
            &mut persistence,
            "validuser",
            "Valid User",
            "validpass",
            "Admin",
        );

        let result = AuthenticationService::login(&mut persistence, "validuser", "validpass");

        assert!(result.is_ok());
        let (_session_token, actor, operator) = result.unwrap();
        assert_eq!(actor.id.to_lowercase(), "validuser");
        assert_eq!(operator.login_name.to_lowercase(), "validuser");
    }

    /// `PHASE_22.1`: Verify all auth failures return same error string
    #[test]
    fn test_all_auth_failures_return_same_error() {
        let mut persistence = create_test_persistence();
        create_test_operator(
            &mut persistence,
            "enabled_user",
            "Enabled User",
            "correct",
            "Admin",
        );
        let disabled_id = create_test_operator(
            &mut persistence,
            "disabled_user",
            "Disabled User",
            "correct",
            "Admin",
        );
        persistence
            .disable_operator(disabled_id)
            .expect("Failed to disable operator");

        // Test unknown operator
        let err1 = AuthenticationService::login(&mut persistence, "unknown", "any").unwrap_err();

        // Test wrong password
        let err2 =
            AuthenticationService::login(&mut persistence, "enabled_user", "wrong").unwrap_err();

        // Test disabled operator
        let err3 =
            AuthenticationService::login(&mut persistence, "disabled_user", "correct").unwrap_err();

        // Extract error messages
        let AuthError::AuthenticationFailed { reason: msg1 } = err1 else {
            panic!("Expected AuthenticationFailed");
        };

        let AuthError::AuthenticationFailed { reason: msg2 } = err2 else {
            panic!("Expected AuthenticationFailed");
        };

        let AuthError::AuthenticationFailed { reason: msg3 } = err3 else {
            panic!("Expected AuthenticationFailed");
        };

        // All three errors must have identical messages
        assert_eq!(msg1, msg2);
        assert_eq!(msg2, msg3);
        assert_eq!(msg1, "invalid_credentials");
    }

    // Authorization Service Tests

    /// `PHASE_27H.6`: Verify admin can register users
    #[test]
    fn test_authorize_register_user_allows_admin() {
        let admin = create_admin_actor();

        let result = AuthorizationService::authorize_register_user(&admin);

        assert!(result.is_ok());
    }

    /// `PHASE_27H.6`: Verify bidder cannot register users
    #[test]
    fn test_authorize_register_user_rejects_bidder() {
        let bidder = create_bidder_actor();

        let result = AuthorizationService::authorize_register_user(&bidder);

        assert!(result.is_err());
        if let AuthError::Unauthorized {
            action,
            required_role,
        } = result.unwrap_err()
        {
            assert_eq!(action, "register_user");
            assert_eq!(required_role, "Admin");
        } else {
            panic!("Expected Unauthorized error");
        }
    }

    /// `PHASE_27H.6`: Verify admin can create bid years
    #[test]
    fn test_authorize_create_bid_year_allows_admin() {
        let admin = create_admin_actor();

        let result = AuthorizationService::authorize_create_bid_year(&admin);

        assert!(result.is_ok());
    }

    /// `PHASE_27H.6`: Verify bidder cannot create bid years
    #[test]
    fn test_authorize_create_bid_year_rejects_bidder() {
        let bidder = create_bidder_actor();

        let result = AuthorizationService::authorize_create_bid_year(&bidder);

        assert!(result.is_err());
        if let AuthError::Unauthorized {
            action,
            required_role,
        } = result.unwrap_err()
        {
            assert_eq!(action, "create_bid_year");
            assert_eq!(required_role, "Admin");
        } else {
            panic!("Expected Unauthorized error");
        }
    }

    /// `PHASE_27H.6`: Verify admin can create areas
    #[test]
    fn test_authorize_create_area_allows_admin() {
        let admin = create_admin_actor();

        let result = AuthorizationService::authorize_create_area(&admin);

        assert!(result.is_ok());
    }

    /// `PHASE_27H.6`: Verify bidder cannot create areas
    #[test]
    fn test_authorize_create_area_rejects_bidder() {
        let bidder = create_bidder_actor();

        let result = AuthorizationService::authorize_create_area(&bidder);

        assert!(result.is_err());
        if let AuthError::Unauthorized {
            action,
            required_role,
        } = result.unwrap_err()
        {
            assert_eq!(action, "create_area");
            assert_eq!(required_role, "Admin");
        } else {
            panic!("Expected Unauthorized error");
        }
    }

    /// `PHASE_27H.6`: Verify admin can reassign crew
    #[test]
    fn test_authorize_reassign_crew_allows_admin() {
        let admin = create_admin_actor();

        let result = AuthorizationService::authorize_reassign_crew(&admin);

        assert!(result.is_ok());
    }

    /// `PHASE_27H.6`: Verify bidder can reassign crew
    #[test]
    fn test_authorize_reassign_crew_allows_bidder() {
        let bidder = create_bidder_actor();

        let result = AuthorizationService::authorize_reassign_crew(&bidder);

        assert!(result.is_ok());
    }

    /// `PHASE_27H.6`: Verify admin can create checkpoint
    #[test]
    fn test_authorize_checkpoint_allows_admin() {
        let admin = create_admin_actor();

        let result = AuthorizationService::authorize_checkpoint(&admin);

        assert!(result.is_ok());
    }

    /// `PHASE_27H.6`: Verify bidder cannot create checkpoint
    #[test]
    fn test_authorize_checkpoint_rejects_bidder() {
        let bidder = create_bidder_actor();

        let result = AuthorizationService::authorize_checkpoint(&bidder);

        assert!(result.is_err());
        if let AuthError::Unauthorized {
            action,
            required_role,
        } = result.unwrap_err()
        {
            assert_eq!(action, "checkpoint");
            assert_eq!(required_role, "Admin");
        } else {
            panic!("Expected Unauthorized error");
        }
    }

    /// `PHASE_27H.6`: Verify admin can finalize round
    #[test]
    fn test_authorize_finalize_allows_admin() {
        let admin = create_admin_actor();

        let result = AuthorizationService::authorize_finalize(&admin);

        assert!(result.is_ok());
    }

    /// `PHASE_27H.6`: Verify bidder cannot finalize round
    #[test]
    fn test_authorize_finalize_rejects_bidder() {
        let bidder = create_bidder_actor();

        let result = AuthorizationService::authorize_finalize(&bidder);

        assert!(result.is_err());
        if let AuthError::Unauthorized {
            action,
            required_role,
        } = result.unwrap_err()
        {
            assert_eq!(action, "finalize");
            assert_eq!(required_role, "Admin");
        } else {
            panic!("Expected Unauthorized error");
        }
    }

    /// `PHASE_27H.6`: Verify admin can perform rollback
    #[test]
    fn test_authorize_rollback_allows_admin() {
        let admin = create_admin_actor();

        let result = AuthorizationService::authorize_rollback(&admin);

        assert!(result.is_ok());
    }

    /// `PHASE_27H.6`: Verify bidder cannot perform rollback
    #[test]
    fn test_authorize_rollback_rejects_bidder() {
        let bidder = create_bidder_actor();

        let result = AuthorizationService::authorize_rollback(&bidder);

        assert!(result.is_err());
        if let AuthError::Unauthorized {
            action,
            required_role,
        } = result.unwrap_err()
        {
            assert_eq!(action, "rollback");
            assert_eq!(required_role, "Admin");
        } else {
            panic!("Expected Unauthorized error");
        }
    }
}
