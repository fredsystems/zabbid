// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

#![deny(
    clippy::pedantic,
    clippy::cargo,
    clippy::nursery,
    clippy::style,
    clippy::correctness,
    clippy::all
)]

use zab_bid::{Command, CoreError, State, TransitionResult, apply};
use zab_bid_audit::{Actor, AuditEvent, Cause};
use zab_bid_domain::{Area, BidYear, Crew, DomainError, Initials, SeniorityData};

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

    /// Converts this authenticated actor into an audit Actor.
    ///
    /// This is used when recording audit events to attribute actions
    /// to the authenticated operator.
    #[must_use]
    pub fn to_audit_actor(&self) -> Actor {
        let actor_type: String = match self.role {
            Role::Admin => String::from("admin"),
            Role::Bidder => String::from("bidder"),
        };
        Actor::new(self.id.clone(), actor_type)
    }
}

/// Authentication and authorization errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    /// Authentication failed.
    AuthenticationFailed {
        /// The reason authentication failed.
        reason: String,
    },
    /// Authorization failed.
    Unauthorized {
        /// The action that was attempted.
        action: String,
        /// The role required for this action.
        required_role: String,
    },
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthenticationFailed { reason } => {
                write!(f, "Authentication failed: {reason}")
            }
            Self::Unauthorized {
                action,
                required_role,
            } => {
                write!(f, "Unauthorized: '{action}' requires {required_role} role")
            }
        }
    }
}

impl std::error::Error for AuthError {}

/// Stub authentication function.
///
/// This is a minimal placeholder for Phase 5. It does NOT implement
/// real authentication - that is explicitly deferred to a later phase.
///
/// In a real system, this would validate credentials, check tokens,
/// or integrate with an identity provider.
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
pub fn authenticate_stub(actor_id: String, role: Role) -> Result<AuthenticatedActor, AuthError> {
    if actor_id.is_empty() {
        return Err(AuthError::AuthenticationFailed {
            reason: String::from("Actor ID cannot be empty"),
        });
    }
    Ok(AuthenticatedActor::new(actor_id, role))
}

/// API request to register a new user for a bid year.
///
/// This DTO is distinct from domain types and represents the API contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterUserRequest {
    /// The bid year (e.g., 2026).
    pub bid_year: u16,
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// The user's area identifier.
    pub area: String,
    /// The user's crew identifier.
    pub crew: String,
    /// Cumulative NATCA bargaining unit date (ISO 8601).
    pub cumulative_natca_bu_date: String,
    /// NATCA bargaining unit date (ISO 8601).
    pub natca_bu_date: String,
    /// Entry on Duty / FAA date (ISO 8601).
    pub eod_faa_date: String,
    /// Service Computation Date (ISO 8601).
    pub service_computation_date: String,
    /// Optional lottery value.
    pub lottery_value: Option<u32>,
}

/// API response for a successful user registration.
///
/// This DTO is distinct from domain types and represents the API contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterUserResponse {
    /// The bid year the user was registered for.
    pub bid_year: u16,
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// A success message.
    pub message: String,
}

/// API-level errors.
///
/// These are distinct from domain/core errors and represent the API contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    /// Authentication failed.
    AuthenticationFailed {
        /// The reason authentication failed.
        reason: String,
    },
    /// Authorization failed - the actor does not have permission.
    Unauthorized {
        /// The action that was attempted.
        action: String,
        /// The role required for this action.
        required_role: String,
    },
    /// A domain rule was violated.
    DomainRuleViolation {
        /// The rule that was violated.
        rule: String,
        /// A human-readable description of the violation.
        message: String,
    },
    /// Invalid input was provided.
    InvalidInput {
        /// The field that was invalid.
        field: String,
        /// A human-readable description of the error.
        message: String,
    },
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthenticationFailed { reason } => {
                write!(f, "Authentication failed: {reason}")
            }
            Self::Unauthorized {
                action,
                required_role,
            } => {
                write!(f, "Unauthorized: '{action}' requires {required_role} role")
            }
            Self::DomainRuleViolation { rule, message } => {
                write!(f, "Domain rule violation ({rule}): {message}")
            }
            Self::InvalidInput { field, message } => {
                write!(f, "Invalid input for field '{field}': {message}")
            }
        }
    }
}

impl std::error::Error for ApiError {}

impl From<AuthError> for ApiError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::AuthenticationFailed { reason } => Self::AuthenticationFailed { reason },
            AuthError::Unauthorized {
                action,
                required_role,
            } => Self::Unauthorized {
                action,
                required_role,
            },
        }
    }
}

/// Translates a domain error into an API error.
///
/// This translation is explicit and ensures domain errors are not leaked directly.
fn translate_domain_error(err: DomainError) -> ApiError {
    match err {
        DomainError::DuplicateInitials { bid_year, initials } => ApiError::DomainRuleViolation {
            rule: String::from("unique_initials"),
            message: format!(
                "User with initials '{}' already exists in bid year {}",
                initials.value(),
                bid_year.year()
            ),
        },
        DomainError::InvalidInitials(msg) => ApiError::InvalidInput {
            field: String::from("initials"),
            message: msg,
        },
        DomainError::InvalidName(msg) => ApiError::InvalidInput {
            field: String::from("name"),
            message: msg,
        },
        DomainError::InvalidArea(msg) => ApiError::InvalidInput {
            field: String::from("area"),
            message: msg,
        },
        DomainError::InvalidCrew(msg) => ApiError::InvalidInput {
            field: String::from("crew"),
            message: msg,
        },
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

/// The result of an API operation that includes both the response and the audit event.
///
/// This ensures that successful API operations always produce an audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiResult<T> {
    /// The API response.
    pub response: T,
    /// The audit event generated by this operation.
    pub audit_event: AuditEvent,
    /// The new state after the operation.
    pub new_state: State,
}

/// Registers a new user via the API boundary with authorization.
///
/// This function:
/// - Verifies the actor is authorized (Admin role required)
/// - Translates the API request into a core command
/// - Applies the command to the current state
/// - Translates any errors to API errors
/// - Returns the API response with audit event on success
///
/// # Arguments
///
/// * `state` - The current system state
/// * `request` - The API request to register a user
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `cause` - The cause or reason for this action
///
/// # Returns
///
/// * `Ok(ApiResult<RegisterUserResponse>)` on success
/// * `Err(ApiError)` if unauthorized, the request is invalid, or a domain rule is violated
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - Any field validation fails
/// - The initials are already in use within the bid year
pub fn register_user(
    state: &State,
    request: RegisterUserRequest,
    authenticated_actor: &AuthenticatedActor,
    cause: Cause,
) -> Result<ApiResult<RegisterUserResponse>, ApiError> {
    // Enforce authorization before executing command
    AuthorizationService::authorize_register_user(authenticated_actor)?;

    // Convert authenticated actor to audit actor for attribution
    let actor: Actor = authenticated_actor.to_audit_actor();
    // Translate API request into domain types
    let bid_year: BidYear = BidYear::new(request.bid_year);
    let initials: Initials = Initials::new(request.initials.clone());
    let area: Area = Area::new(request.area.clone());
    let crew: Crew = Crew::new(request.crew.clone());
    let seniority_data: SeniorityData = SeniorityData::new(
        request.cumulative_natca_bu_date,
        request.natca_bu_date,
        request.eod_faa_date,
        request.service_computation_date,
        request.lottery_value,
    );

    // Create core command
    let command: Command = Command::RegisterUser {
        bid_year: bid_year.clone(),
        initials: initials.clone(),
        name: request.name.clone(),
        area,
        crew,
        seniority_data,
    };

    // Apply command via core transition
    let transition_result: TransitionResult =
        apply(state, command, actor, cause).map_err(|core_err| match core_err {
            CoreError::DomainViolation(domain_err) => translate_domain_error(domain_err),
        })?;

    // Translate to API response
    let response: RegisterUserResponse = RegisterUserResponse {
        bid_year: bid_year.year(),
        initials: initials.value().to_string(),
        name: request.name,
        message: format!(
            "Successfully registered user '{}' for bid year {}",
            initials.value(),
            bid_year.year()
        ),
    };

    Ok(ApiResult {
        response,
        audit_event: transition_result.audit_event,
        new_state: transition_result.new_state,
    })
}

/// Creates a checkpoint via the API boundary with authorization.
///
/// This function:
/// - Verifies the actor is authorized (Admin role required)
/// - Creates a checkpoint command
/// - Applies the command to the current state
/// - Returns the transition result on success
///
/// # Arguments
///
/// * `state` - The current system state
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `cause` - The cause or reason for this action
///
/// # Returns
///
/// * `Ok(TransitionResult)` on success
/// * `Err(ApiError)` if unauthorized or the command fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The command execution fails
pub fn checkpoint(
    state: &State,
    authenticated_actor: &AuthenticatedActor,
    cause: Cause,
) -> Result<TransitionResult, ApiError> {
    // Enforce authorization before executing command
    AuthorizationService::authorize_checkpoint(authenticated_actor)?;

    // Convert authenticated actor to audit actor for attribution
    let actor: Actor = authenticated_actor.to_audit_actor();

    // Create and apply checkpoint command
    let command: Command = Command::Checkpoint;
    let transition_result: TransitionResult =
        apply(state, command, actor, cause).map_err(|core_err| match core_err {
            CoreError::DomainViolation(domain_err) => translate_domain_error(domain_err),
        })?;

    Ok(transition_result)
}

/// Finalizes a round via the API boundary with authorization.
///
/// This function:
/// - Verifies the actor is authorized (Admin role required)
/// - Creates a finalize command
/// - Applies the command to the current state
/// - Returns the transition result on success
///
/// # Arguments
///
/// * `state` - The current system state
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `cause` - The cause or reason for this action
///
/// # Returns
///
/// * `Ok(TransitionResult)` on success
/// * `Err(ApiError)` if unauthorized or the command fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The command execution fails
pub fn finalize(
    state: &State,
    authenticated_actor: &AuthenticatedActor,
    cause: Cause,
) -> Result<TransitionResult, ApiError> {
    // Enforce authorization before executing command
    AuthorizationService::authorize_finalize(authenticated_actor)?;

    // Convert authenticated actor to audit actor for attribution
    let actor: Actor = authenticated_actor.to_audit_actor();

    // Create and apply finalize command
    let command: Command = Command::Finalize;
    let transition_result: TransitionResult =
        apply(state, command, actor, cause).map_err(|core_err| match core_err {
            CoreError::DomainViolation(domain_err) => translate_domain_error(domain_err),
        })?;

    Ok(transition_result)
}

/// Rolls back to a specific event via the API boundary with authorization.
///
/// This function:
/// - Verifies the actor is authorized (Admin role required)
/// - Creates a rollback command
/// - Applies the command to the current state
/// - Returns the transition result on success
///
/// # Arguments
///
/// * `state` - The current system state
/// * `target_event_id` - The event ID to rollback to
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `cause` - The cause or reason for this action
///
/// # Returns
///
/// * `Ok(TransitionResult)` on success
/// * `Err(ApiError)` if unauthorized or the command fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The command execution fails
pub fn rollback(
    state: &State,
    target_event_id: i64,
    authenticated_actor: &AuthenticatedActor,
    cause: Cause,
) -> Result<TransitionResult, ApiError> {
    // Enforce authorization before executing command
    AuthorizationService::authorize_rollback(authenticated_actor)?;

    // Convert authenticated actor to audit actor for attribution
    let actor: Actor = authenticated_actor.to_audit_actor();

    // Create and apply rollback command
    let command: Command = Command::RollbackToEventId { target_event_id };
    let transition_result: TransitionResult =
        apply(state, command, actor, cause).map_err(|core_err| match core_err {
            CoreError::DomainViolation(domain_err) => translate_domain_error(domain_err),
        })?;

    Ok(transition_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_admin() -> AuthenticatedActor {
        AuthenticatedActor::new(String::from("admin-123"), Role::Admin)
    }

    fn create_test_bidder() -> AuthenticatedActor {
        AuthenticatedActor::new(String::from("bidder-456"), Role::Bidder)
    }

    fn create_test_cause() -> Cause {
        Cause::new(String::from("api-req-456"), String::from("API request"))
    }

    fn create_valid_request() -> RegisterUserRequest {
        RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("ABC"),
            name: String::from("John Doe"),
            area: String::from("North"),
            crew: String::from("A"),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(42),
        }
    }

    #[test]
    fn test_valid_api_request_succeeds() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &admin, cause);

        assert!(result.is_ok());
        let api_result: ApiResult<RegisterUserResponse> = result.unwrap();
        assert_eq!(api_result.response.bid_year, 2026);
        assert_eq!(api_result.response.initials, "ABC");
        assert_eq!(api_result.response.name, "John Doe");
        assert!(
            api_result
                .response
                .message
                .contains("Successfully registered")
        );
    }

    #[test]
    fn test_valid_api_request_emits_audit_event() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &admin, cause);

        assert!(result.is_ok());
        let api_result: ApiResult<RegisterUserResponse> = result.unwrap();
        assert_eq!(api_result.audit_event.action.name, "RegisterUser");
        assert_eq!(api_result.audit_event.actor.id, "admin-123");
        assert_eq!(api_result.audit_event.actor.actor_type, "admin");
        assert_eq!(api_result.audit_event.cause.id, "api-req-456");
    }

    #[test]
    fn test_valid_api_request_returns_new_state() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &admin, cause);

        assert!(result.is_ok());
        let api_result: ApiResult<RegisterUserResponse> = result.unwrap();
        assert_eq!(api_result.new_state.users.len(), 1);
        assert_eq!(api_result.new_state.users[0].initials.value(), "ABC");
    }

    #[test]
    fn test_duplicate_initials_returns_api_error() {
        let mut state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request1: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        // Register first user successfully
        let result1: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request1, &admin, cause.clone());
        assert!(result1.is_ok());
        state = result1.unwrap().new_state;

        // Second registration with same initials
        let request2: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("ABC"), // Duplicate
            name: String::from("Jane Smith"),
            area: String::from("South"),
            crew: String::from("B"),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(43),
        };

        let result2: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request2, &admin, cause);

        assert!(result2.is_err());
        let err: ApiError = result2.unwrap_err();
        assert!(matches!(err, ApiError::DomainRuleViolation { .. }));
        if let ApiError::DomainRuleViolation { rule, message } = err {
            assert_eq!(rule, "unique_initials");
            assert!(message.contains("ABC"));
            assert!(message.contains("2026"));
        }
    }

    #[test]
    fn test_failed_api_request_does_not_mutate_state() {
        let mut state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request1: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        // Register first user successfully
        let result1: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request1, &admin, cause.clone());
        assert!(result1.is_ok());
        state = result1.unwrap().new_state;
        let user_count_before: usize = state.users.len();

        // Attempt duplicate registration
        let request2: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("ABC"), // Duplicate
            name: String::from("Jane Smith"),
            area: String::from("South"),
            crew: String::from("B"),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(43),
        };

        let result2: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request2, &admin, cause);

        assert!(result2.is_err());
        // State should remain unchanged
        assert_eq!(state.users.len(), user_count_before);
    }

    #[test]
    fn test_invalid_empty_initials_returns_api_error() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::new(), // Invalid
            name: String::from("John Doe"),
            area: String::from("North"),
            crew: String::from("A"),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(42),
        };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &admin, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::InvalidInput { .. }));
        if let ApiError::InvalidInput { field, message } = err {
            assert_eq!(field, "initials");
            assert!(message.contains("empty"));
        }
    }

    #[test]
    fn test_invalid_empty_name_returns_api_error() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("ABC"),
            name: String::new(), // Invalid
            area: String::from("North"),
            crew: String::from("A"),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(42),
        };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &admin, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::InvalidInput { .. }));
        if let ApiError::InvalidInput { field, .. } = err {
            assert_eq!(field, "name");
        }
    }

    #[test]
    fn test_invalid_empty_area_returns_api_error() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("ABC"),
            name: String::from("John Doe"),
            area: String::new(), // Invalid
            crew: String::from("A"),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(42),
        };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &admin, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::InvalidInput { .. }));
        if let ApiError::InvalidInput { field, .. } = err {
            assert_eq!(field, "area");
        }
    }

    #[test]
    fn test_invalid_empty_crew_returns_api_error() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("ABC"),
            name: String::from("John Doe"),
            area: String::from("North"),
            crew: String::new(), // Invalid
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(42),
        };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &admin, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::InvalidInput { .. }));
        if let ApiError::InvalidInput { field, .. } = err {
            assert_eq!(field, "crew");
        }
    }

    #[test]
    fn test_duplicate_initials_in_different_bid_years_allowed() {
        let state1: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let state2: State = State::new(BidYear::new(2027), Area::new(String::from("South")));
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        // Register user in 2026
        let request1: RegisterUserRequest = create_valid_request();
        let result1: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state1, request1, &admin, cause.clone());
        assert!(result1.is_ok());

        // Same initials in 2027 (different bid year)
        let request2: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2027,                // Different bid year
            initials: String::from("ABC"), // Same initials
            name: String::from("Jane Smith"),
            area: String::from("South"),
            crew: String::from("B"),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(43),
        };

        let result2: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state2, request2, &admin, cause);

        assert!(result2.is_ok());
        let api_result: ApiResult<RegisterUserResponse> = result2.unwrap();
        assert_eq!(api_result.new_state.users.len(), 1);
    }

    #[test]
    fn test_api_error_display() {
        let err1: ApiError = ApiError::DomainRuleViolation {
            rule: String::from("test_rule"),
            message: String::from("test message"),
        };
        assert_eq!(
            format!("{err1}"),
            "Domain rule violation (test_rule): test message"
        );

        let err2: ApiError = ApiError::InvalidInput {
            field: String::from("test_field"),
            message: String::from("test error"),
        };
        assert_eq!(
            format!("{err2}"),
            "Invalid input for field 'test_field': test error"
        );
    }

    #[test]
    fn test_successful_api_call_updates_state() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &admin, cause);

        assert!(result.is_ok());
        let api_result: ApiResult<RegisterUserResponse> = result.unwrap();

        // New state has the user
        assert_eq!(api_result.new_state.users.len(), 1);
        assert_eq!(api_result.new_state.users[0].name, "John Doe");

        // Original state is unchanged
        assert_eq!(state.users.len(), 0);
    }

    // Phase 5: Authorization Tests

    #[test]
    fn test_authenticate_stub_succeeds_with_valid_id() {
        let result: Result<AuthenticatedActor, AuthError> =
            authenticate_stub(String::from("user-123"), Role::Admin);
        assert!(result.is_ok());
        let actor: AuthenticatedActor = result.unwrap();
        assert_eq!(actor.id, "user-123");
        assert_eq!(actor.role, Role::Admin);
    }

    #[test]
    fn test_authenticate_stub_fails_with_empty_id() {
        let result: Result<AuthenticatedActor, AuthError> =
            authenticate_stub(String::new(), Role::Admin);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AuthError::AuthenticationFailed { .. }
        ));
    }

    #[test]
    fn test_authenticated_actor_to_audit_actor_admin() {
        let auth_actor: AuthenticatedActor =
            AuthenticatedActor::new(String::from("admin-1"), Role::Admin);
        let audit_actor: Actor = auth_actor.to_audit_actor();
        assert_eq!(audit_actor.id, "admin-1");
        assert_eq!(audit_actor.actor_type, "admin");
    }

    #[test]
    fn test_authenticated_actor_to_audit_actor_bidder() {
        let auth_actor: AuthenticatedActor =
            AuthenticatedActor::new(String::from("bidder-1"), Role::Bidder);
        let audit_actor: Actor = auth_actor.to_audit_actor();
        assert_eq!(audit_actor.id, "bidder-1");
        assert_eq!(audit_actor.actor_type, "bidder");
    }

    #[test]
    fn test_bidder_cannot_register_user() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = create_valid_request();
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &bidder, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::Unauthorized { .. }));
        if let ApiError::Unauthorized {
            action,
            required_role,
        } = err
        {
            assert_eq!(action, "register_user");
            assert_eq!(required_role, "Admin");
        }
    }

    #[test]
    fn test_unauthorized_action_does_not_mutate_state() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = create_valid_request();
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &bidder, cause);

        assert!(result.is_err());
        // Original state is unchanged
        assert_eq!(state.users.len(), 0);
    }

    #[test]
    fn test_unauthorized_action_does_not_emit_audit_event() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let request: RegisterUserRequest = create_valid_request();
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&state, request, &bidder, cause);

        assert!(result.is_err());
        // No audit event is returned on authorization failure
    }

    #[test]
    fn test_admin_can_create_checkpoint() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> = checkpoint(&state, &admin, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.audit_event.action.name, "Checkpoint");
        assert_eq!(transition.audit_event.actor.id, "admin-123");
        assert_eq!(transition.audit_event.actor.actor_type, "admin");
    }

    #[test]
    fn test_bidder_cannot_create_checkpoint() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> = checkpoint(&state, &bidder, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::Unauthorized { .. }));
        if let ApiError::Unauthorized {
            action,
            required_role,
        } = err
        {
            assert_eq!(action, "checkpoint");
            assert_eq!(required_role, "Admin");
        }
    }

    #[test]
    fn test_admin_can_finalize() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> = finalize(&state, &admin, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.audit_event.action.name, "Finalize");
        assert_eq!(transition.audit_event.actor.id, "admin-123");
        assert_eq!(transition.audit_event.actor.actor_type, "admin");
    }

    #[test]
    fn test_bidder_cannot_finalize() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> = finalize(&state, &bidder, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::Unauthorized { .. }));
        if let ApiError::Unauthorized {
            action,
            required_role,
        } = err
        {
            assert_eq!(action, "finalize");
            assert_eq!(required_role, "Admin");
        }
    }

    #[test]
    fn test_admin_can_rollback() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> = rollback(&state, 1, &admin, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.audit_event.action.name, "Rollback");
        assert_eq!(transition.audit_event.actor.id, "admin-123");
        assert_eq!(transition.audit_event.actor.actor_type, "admin");
    }

    #[test]
    fn test_bidder_cannot_rollback() {
        let state: State = State::new(BidYear::new(2026), Area::new(String::from("North")));
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> = rollback(&state, 1, &bidder, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::Unauthorized { .. }));
        if let ApiError::Unauthorized {
            action,
            required_role,
        } = err
        {
            assert_eq!(action, "rollback");
            assert_eq!(required_role, "Admin");
        }
    }

    #[test]
    fn test_authorization_error_converts_to_api_error() {
        let auth_err: AuthError = AuthError::Unauthorized {
            action: String::from("test_action"),
            required_role: String::from("Admin"),
        };
        let api_err: ApiError = ApiError::from(auth_err);
        assert!(matches!(api_err, ApiError::Unauthorized { .. }));
    }

    #[test]
    fn test_authentication_error_converts_to_api_error() {
        let auth_err: AuthError = AuthError::AuthenticationFailed {
            reason: String::from("invalid token"),
        };
        let api_err: ApiError = ApiError::from(auth_err);
        assert!(matches!(api_err, ApiError::AuthenticationFailed { .. }));
    }

    #[test]
    fn test_auth_error_display_unauthorized() {
        let err: AuthError = AuthError::Unauthorized {
            action: String::from("test_action"),
            required_role: String::from("Admin"),
        };
        let display: String = format!("{err}");
        assert!(display.contains("Unauthorized"));
        assert!(display.contains("test_action"));
        assert!(display.contains("Admin"));
    }

    #[test]
    fn test_auth_error_display_authentication_failed() {
        let err: AuthError = AuthError::AuthenticationFailed {
            reason: String::from("invalid credentials"),
        };
        let display: String = format!("{err}");
        assert!(display.contains("Authentication failed"));
        assert!(display.contains("invalid credentials"));
    }

    #[test]
    fn test_api_error_display_unauthorized() {
        let err: ApiError = ApiError::Unauthorized {
            action: String::from("register_user"),
            required_role: String::from("Admin"),
        };
        let display: String = format!("{err}");
        assert!(display.contains("Unauthorized"));
        assert!(display.contains("register_user"));
        assert!(display.contains("Admin"));
    }

    #[test]
    fn test_api_error_display_authentication_failed() {
        let err: ApiError = ApiError::AuthenticationFailed {
            reason: String::from("token expired"),
        };
        let display: String = format!("{err}");
        assert!(display.contains("Authentication failed"));
        assert!(display.contains("token expired"));
    }
}
