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
    /// This is a simplified authentication that validates the operator exists
    /// and is not disabled. In a production system, this would validate
    /// credentials (password, token, etc.).
    ///
    /// # Arguments
    ///
    /// * `persistence` - The persistence layer
    /// * `login_name` - The operator login name
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
    ) -> Result<(String, AuthenticatedActor, OperatorData), AuthError> {
        // Retrieve operator by login name
        let operator: OperatorData = persistence
            .get_operator_by_login(login_name)
            .map_err(|e| AuthError::AuthenticationFailed {
                reason: format!("Database error: {e}"),
            })?
            .ok_or_else(|| AuthError::AuthenticationFailed {
                reason: format!("Unknown operator: {login_name}"),
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

        // Generate session token
        let session_token: String = Self::generate_session_token();

        // Calculate expiration time
        let expires_at: OffsetDateTime =
            OffsetDateTime::now_utc() + Self::DEFAULT_SESSION_EXPIRATION;
        let expires_at_str: String = expires_at
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .map_err(|e| AuthError::AuthenticationFailed {
                reason: format!("Failed to format expiration time: {e}"),
            })?;

        // Create session
        persistence
            .create_session(&session_token, operator.operator_id, &expires_at_str)
            .map_err(|e| AuthError::AuthenticationFailed {
                reason: format!("Failed to create session: {e}"),
            })?;

        // Update last login timestamp
        persistence
            .update_last_login(operator.operator_id)
            .map_err(|e| AuthError::AuthenticationFailed {
                reason: format!("Failed to update last login: {e}"),
            })?;

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
        let expires_at: OffsetDateTime = OffsetDateTime::parse(
            &session.expires_at,
            &time::format_description::well_known::Iso8601::DEFAULT,
        )
        .map_err(|e| AuthError::AuthenticationFailed {
            reason: format!("Failed to parse session expiration: {e}"),
        })?;

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
            .expect("Time went backwards")
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
