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

use zab_bid::{
    BootstrapMetadata, BootstrapResult, Command, CoreError, State, TransitionResult, apply,
    apply_bootstrap,
};
use zab_bid_audit::{Actor, AuditEvent, Cause};
use zab_bid_domain::{Area, BidYear, Crew, DomainError, Initials, SeniorityData, UserType};

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

/// API request to create a new bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBidYearRequest {
    /// The year value (e.g., 2026).
    pub year: u16,
}

/// API response for a successful bid year creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateBidYearResponse {
    /// The created bid year.
    pub year: u16,
    /// A success message.
    pub message: String,
}

/// API request to create a new area within a bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAreaRequest {
    /// The bid year this area belongs to.
    pub bid_year: u16,
    /// The area identifier.
    pub area_id: String,
}

/// API response for a successful area creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAreaResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area_id: String,
    /// A success message.
    pub message: String,
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
    /// The user's type classification (CPC, CPC-IT, Dev-R, Dev-D).
    pub user_type: String,
    /// The user's crew number (1-7, optional).
    pub crew: Option<u8>,
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
            message: msg.to_string(),
        },
        DomainError::InvalidUserType(msg) => ApiError::InvalidInput {
            field: String::from("user_type"),
            message: msg,
        },
        DomainError::BidYearNotFound(year) => ApiError::DomainRuleViolation {
            rule: String::from("bid_year_exists"),
            message: format!("Bid year {year} not found"),
        },
        DomainError::AreaNotFound { bid_year, area } => ApiError::DomainRuleViolation {
            rule: String::from("area_exists"),
            message: format!("Area '{area}' not found in bid year {bid_year}"),
        },
        DomainError::DuplicateBidYear(year) => ApiError::DomainRuleViolation {
            rule: String::from("unique_bid_year"),
            message: format!("Bid year {year} already exists"),
        },
        DomainError::DuplicateArea { bid_year, area } => ApiError::DomainRuleViolation {
            rule: String::from("unique_area"),
            message: format!("Area '{area}' already exists in bid year {bid_year}"),
        },
        DomainError::InvalidBidYear(msg) => ApiError::InvalidInput {
            field: String::from("bid_year"),
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
    metadata: &BootstrapMetadata,
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
    let initials: Initials = Initials::new(&request.initials);
    let area: Area = Area::new(&request.area);

    // Parse user type
    let user_type: UserType =
        UserType::parse(&request.user_type).map_err(translate_domain_error)?;

    // Parse optional crew
    let crew: Option<Crew> = match request.crew {
        Some(crew_num) => Some(Crew::new(crew_num).map_err(translate_domain_error)?),
        None => None,
    };

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
        user_type,
        crew,
        seniority_data,
    };

    // Apply command via core transition
    let transition_result: TransitionResult = apply(metadata, state, command, actor, cause)
        .map_err(|core_err| match core_err {
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
    metadata: &BootstrapMetadata,
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
    let transition_result: TransitionResult = apply(metadata, state, command, actor, cause)
        .map_err(|core_err| match core_err {
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
    metadata: &BootstrapMetadata,
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
    let transition_result: TransitionResult = apply(metadata, state, command, actor, cause)
        .map_err(|core_err| match core_err {
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
    metadata: &BootstrapMetadata,
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
    let transition_result: TransitionResult = apply(metadata, state, command, actor, cause)
        .map_err(|core_err| match core_err {
            CoreError::DomainViolation(domain_err) => translate_domain_error(domain_err),
        })?;

    Ok(transition_result)
}

/// Creates a new bid year via the API boundary with authorization.
///
/// This function:
/// - Verifies the actor is authorized (Admin role required)
/// - Creates a `CreateBidYear` command
/// - Applies the command to the bootstrap metadata
/// - Returns the bootstrap result on success
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `request` - The API request to create a bid year
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `cause` - The cause or reason for this action
///
/// # Returns
///
/// * `Ok(BootstrapResult)` on success
/// * `Err(ApiError)` if unauthorized or the command fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The bid year already exists
/// - The bid year value is invalid
pub fn create_bid_year(
    metadata: &BootstrapMetadata,
    request: &CreateBidYearRequest,
    authenticated_actor: &AuthenticatedActor,
    cause: Cause,
) -> Result<BootstrapResult, ApiError> {
    // Enforce authorization - only admins can create bid years
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("create_bid_year"),
            required_role: String::from("Admin"),
        });
    }

    // Convert authenticated actor to audit actor for attribution
    let actor: Actor = authenticated_actor.to_audit_actor();

    // Create command
    let command: Command = Command::CreateBidYear { year: request.year };

    // Apply command via core bootstrap
    let bootstrap_result: BootstrapResult = apply_bootstrap(metadata, command, actor, cause)
        .map_err(|core_err| match core_err {
            CoreError::DomainViolation(domain_err) => translate_domain_error(domain_err),
        })?;

    Ok(bootstrap_result)
}

/// Creates a new area via the API boundary with authorization.
///
/// This function:
/// - Verifies the actor is authorized (Admin role required)
/// - Creates a `CreateArea` command
/// - Applies the command to the bootstrap metadata
/// - Returns the bootstrap result on success
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `request` - The API request to create an area
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `cause` - The cause or reason for this action
///
/// # Returns
///
/// * `Ok(BootstrapResult)` on success
/// * `Err(ApiError)` if unauthorized or the command fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The bid year does not exist
/// - The area already exists in the bid year
pub fn create_area(
    metadata: &BootstrapMetadata,
    request: CreateAreaRequest,
    authenticated_actor: &AuthenticatedActor,
    cause: Cause,
) -> Result<BootstrapResult, ApiError> {
    // Enforce authorization - only admins can create areas
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("create_area"),
            required_role: String::from("Admin"),
        });
    }

    // Convert authenticated actor to audit actor for attribution
    let actor: Actor = authenticated_actor.to_audit_actor();

    // Create command
    let command: Command = Command::CreateArea {
        bid_year: BidYear::new(request.bid_year),
        area_id: request.area_id,
    };

    // Apply command via core bootstrap
    let bootstrap_result: BootstrapResult = apply_bootstrap(metadata, command, actor, cause)
        .map_err(|core_err| match core_err {
            CoreError::DomainViolation(domain_err) => translate_domain_error(domain_err),
        })?;

    Ok(bootstrap_result)
}

/// API response for listing bid years.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListBidYearsResponse {
    /// The list of bid years.
    pub bid_years: Vec<u16>,
}

/// API request to list areas for a bid year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAreasRequest {
    /// The bid year to list areas for.
    pub bid_year: u16,
}

/// API response for listing areas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListAreasResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The list of area identifiers.
    pub areas: Vec<String>,
}

/// API request to list users for a bid year and area.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUsersRequest {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area: String,
}

/// API response for listing users.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUsersResponse {
    /// The bid year.
    pub bid_year: u16,
    /// The area identifier.
    pub area: String,
    /// The list of users.
    pub users: Vec<UserInfo>,
}

/// User information for listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserInfo {
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// The user's crew (optional).
    pub crew: Option<u8>,
}

/// Lists all bid years.
///
/// This operation never fails and requires no authorization.
/// Returns an empty list if no bid years have been created.
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
///
/// # Returns
///
/// A response containing all bid years.
#[must_use]
pub fn list_bid_years(metadata: &BootstrapMetadata) -> ListBidYearsResponse {
    let bid_years: Vec<u16> = metadata.bid_years.iter().map(BidYear::year).collect();

    ListBidYearsResponse { bid_years }
}

/// Lists all areas for a given bid year.
///
/// This is a read-only operation that requires no authorization.
/// Returns an empty list if the bid year has no areas.
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `request` - The list areas request
///
/// # Returns
///
/// A response containing all areas for the bid year.
#[must_use]
pub fn list_areas(metadata: &BootstrapMetadata, request: &ListAreasRequest) -> ListAreasResponse {
    let bid_year: BidYear = BidYear::new(request.bid_year);
    let areas: Vec<String> = metadata
        .areas
        .iter()
        .filter(|(by, _)| by.year() == bid_year.year())
        .map(|(_, area)| area.id().to_string())
        .collect();

    ListAreasResponse {
        bid_year: request.bid_year,
        areas,
    }
}

/// Lists all users for a given bid year and area.
///
/// This is a read-only operation that requires no authorization.
/// Returns an empty list if no users exist for the given scope.
///
/// # Arguments
///
/// * `state` - The current state for the bid year and area
///
/// # Returns
///
/// A response containing all users.
#[must_use]
pub fn list_users(state: &State) -> ListUsersResponse {
    let users: Vec<UserInfo> = state
        .users
        .iter()
        .map(|user| UserInfo {
            initials: user.initials.value().to_string(),
            name: user.name.clone(),
            crew: user.crew.as_ref().map(Crew::number),
        })
        .collect();

    ListUsersResponse {
        bid_year: state.bid_year.year(),
        area: state.area.id().to_string(),
        users,
    }
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

    fn create_test_metadata() -> BootstrapMetadata {
        let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
        let bid_year: BidYear = BidYear::new(2026);
        let area: Area = Area::new("North");
        metadata.bid_years.push(bid_year.clone());
        metadata.areas.push((bid_year, area));
        metadata
    }

    fn create_valid_request() -> RegisterUserRequest {
        RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("AB"),
            name: String::from("John Doe"),
            area: String::from("North"),
            user_type: String::from("CPC"),
            crew: Some(1),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(42),
        }
    }

    #[test]
    fn test_valid_api_request_succeeds() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &admin, cause);

        assert!(result.is_ok());
        let api_result: ApiResult<RegisterUserResponse> = result.unwrap();
        assert_eq!(api_result.response.bid_year, 2026);
        assert_eq!(api_result.response.initials, "AB");
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
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &admin, cause);

        assert!(result.is_ok());
        let api_result: ApiResult<RegisterUserResponse> = result.unwrap();
        assert_eq!(api_result.audit_event.action.name, "RegisterUser");
        assert_eq!(api_result.audit_event.actor.id, "admin-123");
        assert_eq!(api_result.audit_event.actor.actor_type, "admin");
        assert_eq!(api_result.audit_event.cause.id, "api-req-456");
    }

    #[test]
    fn test_valid_api_request_returns_new_state() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &admin, cause);

        assert!(result.is_ok());
        let api_result: ApiResult<RegisterUserResponse> = result.unwrap();
        assert_eq!(api_result.new_state.users.len(), 1);
        assert_eq!(api_result.new_state.users[0].initials.value(), "AB");
    }

    #[test]
    fn test_duplicate_initials_returns_api_error() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let mut state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request1: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        // Register first user successfully
        let result1: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request1, &admin, cause.clone());
        assert!(result1.is_ok());
        state = result1.unwrap().new_state;

        // Second registration with same initials in the same area
        let request2: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("AB"), // Duplicate
            name: String::from("Jane Smith"),
            area: String::from("North"), // Same area as first user
            user_type: String::from("CPC"),
            crew: Some(2),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(43),
        };

        let result2: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request2, &admin, cause);

        assert!(result2.is_err());
        let err: ApiError = result2.unwrap_err();
        assert!(matches!(err, ApiError::DomainRuleViolation { .. }));
        if let ApiError::DomainRuleViolation { rule, message } = err {
            assert_eq!(rule, "unique_initials");
            assert!(message.contains("AB"));
            assert!(message.contains("2026"));
        }
    }

    #[test]
    fn test_failed_api_request_does_not_mutate_state() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let mut state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request1: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        // Register first user successfully
        let result1: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request1, &admin, cause.clone());
        assert!(result1.is_ok());
        state = result1.unwrap().new_state;
        let user_count_before: usize = state.users.len();

        // Attempt duplicate registration
        let request2: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("AB"), // Duplicate
            name: String::from("Jane Smith"),
            area: String::from("South"),
            user_type: String::from("CPC"),
            crew: Some(2),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(43),
        };

        let result2: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request2, &admin, cause);

        assert!(result2.is_err());
        // State should remain unchanged
        assert_eq!(state.users.len(), user_count_before);
    }

    #[test]
    fn test_invalid_empty_initials_returns_api_error() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::new(), // Invalid
            name: String::from("John Doe"),
            area: String::from("North"),
            user_type: String::from("CPC"),
            crew: Some(1),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(42),
        };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &admin, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::InvalidInput { .. }));
        if let ApiError::InvalidInput { field, message } = err {
            assert_eq!(field, "initials");
            assert!(message.contains("exactly 2 characters"));
        }
    }

    #[test]
    fn test_invalid_empty_name_returns_api_error() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("AB"),
            name: String::new(), // Invalid
            area: String::from("North"),
            user_type: String::from("CPC"),
            crew: Some(1),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(42),
        };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &admin, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::InvalidInput { .. }));
        if let ApiError::InvalidInput { field, .. } = err {
            assert_eq!(field, "name");
        }
    }

    #[test]
    fn test_invalid_empty_area_returns_api_error() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("AB"),
            name: String::from("John Doe"),
            area: String::new(), // Invalid
            user_type: String::from("CPC"),
            crew: Some(1),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(42),
        };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &admin, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::DomainRuleViolation { .. }));
        if let ApiError::DomainRuleViolation { rule, .. } = err {
            assert_eq!(rule, "area_exists");
        }
    }

    #[test]
    fn test_invalid_crew_number_returns_api_error() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("AB"),
            name: String::from("John Doe"),
            area: String::from("North"),
            user_type: String::from("CPC"),
            crew: Some(99), // Invalid: must be 1-7
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(42),
        };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &admin, cause);

        assert!(result.is_err());
        let err: ApiError = result.unwrap_err();
        assert!(matches!(err, ApiError::InvalidInput { .. }));
        if let ApiError::InvalidInput { field, message: _ } = err {
            assert_eq!(field, "crew");
        }
    }

    #[test]
    fn test_duplicate_initials_in_different_bid_years_allowed() {
        // Need to create metadata with both bid years and areas
        let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
        metadata.bid_years.push(BidYear::new(2026));
        metadata.bid_years.push(BidYear::new(2027));
        metadata
            .areas
            .push((BidYear::new(2026), Area::new("North")));
        metadata
            .areas
            .push((BidYear::new(2027), Area::new("South")));

        let state1: State = State::new(BidYear::new(2026), Area::new("North"));
        let state2: State = State::new(BidYear::new(2027), Area::new("South"));
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        // Register user in 2026
        let request1: RegisterUserRequest = create_valid_request();
        let result1: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state1, request1, &admin, cause.clone());
        assert!(result1.is_ok());

        // Same initials in 2027 (different bid year)
        let request2: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2027,               // Different bid year
            initials: String::from("AB"), // Same initials
            name: String::from("Jane Smith"),
            area: String::from("South"),
            user_type: String::from("CPC"),
            crew: Some(2),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2019-06-01"),
            eod_faa_date: String::from("2020-01-15"),
            service_computation_date: String::from("2020-01-15"),
            lottery_value: Some(43),
        };

        let result2: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state2, request2, &admin, cause);

        assert!(result2.is_ok());
        let api_result: ApiResult<RegisterUserResponse> = result2.unwrap();
        assert_eq!(api_result.new_state.users.len(), 1);
    }

    #[test]
    fn test_api_error_display() {
        let _metadata: BootstrapMetadata = create_test_metadata();
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
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = create_valid_request();
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &admin, cause);

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
        let _metadata: BootstrapMetadata = create_test_metadata();
        let result: Result<AuthenticatedActor, AuthError> =
            authenticate_stub(String::from("user-123"), Role::Admin);
        assert!(result.is_ok());
        let actor: AuthenticatedActor = result.unwrap();
        assert_eq!(actor.id, "user-123");
        assert_eq!(actor.role, Role::Admin);
    }

    #[test]
    fn test_authenticate_stub_fails_with_empty_id() {
        let _metadata: BootstrapMetadata = create_test_metadata();
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
        let _metadata: BootstrapMetadata = create_test_metadata();
        let auth_actor: AuthenticatedActor =
            AuthenticatedActor::new(String::from("admin-1"), Role::Admin);
        let audit_actor: Actor = auth_actor.to_audit_actor();
        assert_eq!(audit_actor.id, "admin-1");
        assert_eq!(audit_actor.actor_type, "admin");
    }

    #[test]
    fn test_authenticated_actor_to_audit_actor_bidder() {
        let _metadata: BootstrapMetadata = create_test_metadata();
        let auth_actor: AuthenticatedActor =
            AuthenticatedActor::new(String::from("bidder-1"), Role::Bidder);
        let audit_actor: Actor = auth_actor.to_audit_actor();
        assert_eq!(audit_actor.id, "bidder-1");
        assert_eq!(audit_actor.actor_type, "bidder");
    }

    #[test]
    fn test_bidder_cannot_register_user() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = create_valid_request();
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &bidder, cause);

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
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = create_valid_request();
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &bidder, cause);

        assert!(result.is_err());
        // Original state is unchanged
        assert_eq!(state.users.len(), 0);
    }

    #[test]
    fn test_unauthorized_action_does_not_emit_audit_event() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let request: RegisterUserRequest = create_valid_request();
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &bidder, cause);

        assert!(result.is_err());
        // No audit event is returned on authorization failure
    }

    #[test]
    fn test_admin_can_create_checkpoint() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> =
            checkpoint(&metadata, &state, &admin, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.audit_event.action.name, "Checkpoint");
        assert_eq!(transition.audit_event.actor.id, "admin-123");
        assert_eq!(transition.audit_event.actor.actor_type, "admin");
    }

    #[test]
    fn test_bidder_cannot_create_checkpoint() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> =
            checkpoint(&metadata, &state, &bidder, cause);

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
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> = finalize(&metadata, &state, &admin, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.audit_event.action.name, "Finalize");
        assert_eq!(transition.audit_event.actor.id, "admin-123");
        assert_eq!(transition.audit_event.actor.actor_type, "admin");
    }

    #[test]
    fn test_bidder_cannot_finalize() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> =
            finalize(&metadata, &state, &bidder, cause);

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
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> =
            rollback(&metadata, &state, 1, &admin, cause);

        assert!(result.is_ok());
        let transition: TransitionResult = result.unwrap();
        assert_eq!(transition.audit_event.action.name, "Rollback");
        assert_eq!(transition.audit_event.actor.id, "admin-123");
        assert_eq!(transition.audit_event.actor.actor_type, "admin");
    }

    #[test]
    fn test_bidder_cannot_rollback() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<TransitionResult, ApiError> =
            rollback(&metadata, &state, 1, &bidder, cause);

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
        let _metadata: BootstrapMetadata = create_test_metadata();
        let auth_err: AuthError = AuthError::Unauthorized {
            action: String::from("test_action"),
            required_role: String::from("Admin"),
        };
        let api_err: ApiError = ApiError::from(auth_err);
        assert!(matches!(api_err, ApiError::Unauthorized { .. }));
    }

    #[test]
    fn test_authentication_error_converts_to_api_error() {
        let _metadata: BootstrapMetadata = create_test_metadata();
        let auth_err: AuthError = AuthError::AuthenticationFailed {
            reason: String::from("invalid token"),
        };
        let api_err: ApiError = ApiError::from(auth_err);
        assert!(matches!(api_err, ApiError::AuthenticationFailed { .. }));
    }

    #[test]
    fn test_auth_error_display_unauthorized() {
        let _metadata: BootstrapMetadata = create_test_metadata();
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
        let _metadata: BootstrapMetadata = create_test_metadata();
        let err: AuthError = AuthError::AuthenticationFailed {
            reason: String::from("invalid credentials"),
        };
        let display: String = format!("{err}");
        assert!(display.contains("Authentication failed"));
        assert!(display.contains("invalid credentials"));
    }

    #[test]
    fn test_api_error_display_unauthorized() {
        let _metadata: BootstrapMetadata = create_test_metadata();
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
        let _metadata: BootstrapMetadata = create_test_metadata();
        let err: ApiError = ApiError::AuthenticationFailed {
            reason: String::from("token expired"),
        };
        let display: String = format!("{err}");
        assert!(display.contains("Authentication failed"));
        assert!(display.contains("token expired"));
    }

    #[test]
    fn test_create_bid_year_succeeds() {
        let metadata: BootstrapMetadata = BootstrapMetadata::new();
        let request: CreateBidYearRequest = CreateBidYearRequest { year: 2026 };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<BootstrapResult, ApiError> =
            create_bid_year(&metadata, &request, &admin, cause);

        assert!(result.is_ok());
        let bootstrap_result: BootstrapResult = result.unwrap();
        assert_eq!(bootstrap_result.new_metadata.bid_years.len(), 1);
        assert_eq!(bootstrap_result.new_metadata.bid_years[0].year(), 2026);
    }

    #[test]
    fn test_create_bid_year_requires_admin() {
        let metadata: BootstrapMetadata = BootstrapMetadata::new();
        let request: CreateBidYearRequest = CreateBidYearRequest { year: 2026 };
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<BootstrapResult, ApiError> =
            create_bid_year(&metadata, &request, &bidder, cause);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Unauthorized { .. }));
    }

    #[test]
    fn test_create_area_succeeds() {
        let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
        metadata.bid_years.push(BidYear::new(2026));

        let request: CreateAreaRequest = CreateAreaRequest {
            bid_year: 2026,
            area_id: String::from("North"),
        };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<BootstrapResult, ApiError> =
            create_area(&metadata, request, &admin, cause);

        assert!(result.is_ok());
        let bootstrap_result: BootstrapResult = result.unwrap();
        assert_eq!(bootstrap_result.new_metadata.areas.len(), 1);
    }

    #[test]
    fn test_create_area_requires_admin() {
        let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
        metadata.bid_years.push(BidYear::new(2026));

        let request: CreateAreaRequest = CreateAreaRequest {
            bid_year: 2026,
            area_id: String::from("North"),
        };
        let bidder: AuthenticatedActor = create_test_bidder();
        let cause: Cause = create_test_cause();

        let result: Result<BootstrapResult, ApiError> =
            create_area(&metadata, request, &bidder, cause);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Unauthorized { .. }));
    }

    #[test]
    fn test_create_area_without_bid_year_fails() {
        let metadata: BootstrapMetadata = BootstrapMetadata::new();
        let request: CreateAreaRequest = CreateAreaRequest {
            bid_year: 2026,
            area_id: String::from("North"),
        };
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let result: Result<BootstrapResult, ApiError> =
            create_area(&metadata, request, &admin, cause);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApiError::DomainRuleViolation { .. }
        ));
    }

    #[test]
    fn test_list_bid_years_empty() {
        let metadata: BootstrapMetadata = BootstrapMetadata::new();
        let response: ListBidYearsResponse = list_bid_years(&metadata);

        assert_eq!(response.bid_years.len(), 0);
    }

    #[test]
    fn test_list_bid_years_with_single_year() {
        let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
        metadata.bid_years.push(BidYear::new(2026));

        let response: ListBidYearsResponse = list_bid_years(&metadata);

        assert_eq!(response.bid_years.len(), 1);
        assert_eq!(response.bid_years[0], 2026);
    }

    #[test]
    fn test_list_bid_years_with_multiple_years() {
        let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
        metadata.bid_years.push(BidYear::new(2026));
        metadata.bid_years.push(BidYear::new(2027));
        metadata.bid_years.push(BidYear::new(2028));

        let response: ListBidYearsResponse = list_bid_years(&metadata);

        assert_eq!(response.bid_years.len(), 3);
        assert!(response.bid_years.contains(&2026));
        assert!(response.bid_years.contains(&2027));
        assert!(response.bid_years.contains(&2028));
    }

    #[test]
    fn test_list_areas_empty() {
        let metadata: BootstrapMetadata = BootstrapMetadata::new();
        let request: ListAreasRequest = ListAreasRequest { bid_year: 2026 };

        let response: ListAreasResponse = list_areas(&metadata, &request);

        assert_eq!(response.bid_year, 2026);
        assert_eq!(response.areas.len(), 0);
    }

    #[test]
    fn test_list_areas_for_bid_year() {
        let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
        metadata.bid_years.push(BidYear::new(2026));
        metadata
            .areas
            .push((BidYear::new(2026), Area::new("North")));
        metadata
            .areas
            .push((BidYear::new(2026), Area::new("South")));

        let request: ListAreasRequest = ListAreasRequest { bid_year: 2026 };
        let response: ListAreasResponse = list_areas(&metadata, &request);

        assert_eq!(response.bid_year, 2026);
        assert_eq!(response.areas.len(), 2);
        assert!(response.areas.contains(&String::from("NORTH")));
        assert!(response.areas.contains(&String::from("SOUTH")));
    }

    #[test]
    fn test_list_areas_isolated_by_bid_year() {
        let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
        metadata.bid_years.push(BidYear::new(2026));
        metadata.bid_years.push(BidYear::new(2027));
        metadata
            .areas
            .push((BidYear::new(2026), Area::new("North")));
        metadata
            .areas
            .push((BidYear::new(2027), Area::new("South")));

        let request_2026: ListAreasRequest = ListAreasRequest { bid_year: 2026 };
        let response_2026: ListAreasResponse = list_areas(&metadata, &request_2026);

        assert_eq!(response_2026.areas.len(), 1);
        assert_eq!(response_2026.areas[0], "NORTH");

        let request_2027: ListAreasRequest = ListAreasRequest { bid_year: 2027 };
        let response_2027: ListAreasResponse = list_areas(&metadata, &request_2027);

        assert_eq!(response_2027.areas.len(), 1);
        assert_eq!(response_2027.areas[0], "SOUTH");
    }

    #[test]
    fn test_list_users_empty() {
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let response: ListUsersResponse = list_users(&state);

        assert_eq!(response.bid_year, 2026);
        assert_eq!(response.area, "NORTH");
        assert_eq!(response.users.len(), 0);
    }

    #[test]
    fn test_list_users_with_users() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let request1: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("AB"),
            name: String::from("Alice Brown"),
            area: String::from("North"),
            user_type: String::from("CPC"),
            crew: Some(1),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2020-03-10"),
            eod_faa_date: String::from("2018-06-01"),
            service_computation_date: String::from("2018-06-01"),
            lottery_value: None,
        };

        let result1: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request1, &admin, cause.clone());
        assert!(result1.is_ok());

        let state_with_user1: State = result1.unwrap().new_state;

        let request2: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("CD"),
            name: String::from("Charlie Davis"),
            area: String::from("North"),
            user_type: String::from("CPC"),
            crew: Some(2),
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2020-03-10"),
            eod_faa_date: String::from("2018-06-01"),
            service_computation_date: String::from("2018-06-01"),
            lottery_value: None,
        };

        let result2: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state_with_user1, request2, &admin, cause);
        assert!(result2.is_ok());

        let final_state: State = result2.unwrap().new_state;
        let response: ListUsersResponse = list_users(&final_state);

        assert_eq!(response.bid_year, 2026);
        assert_eq!(response.area, "NORTH");
        assert_eq!(response.users.len(), 2);

        let ab_user = response.users.iter().find(|u| u.initials == "AB").unwrap();
        assert_eq!(ab_user.name, "Alice Brown");
        assert_eq!(ab_user.crew, Some(1));

        let cd_user = response.users.iter().find(|u| u.initials == "CD").unwrap();
        assert_eq!(cd_user.name, "Charlie Davis");
        assert_eq!(cd_user.crew, Some(2));
    }

    #[test]
    fn test_list_users_with_no_crew() {
        let metadata: BootstrapMetadata = create_test_metadata();
        let state: State = State::new(BidYear::new(2026), Area::new("North"));
        let admin: AuthenticatedActor = create_test_admin();
        let cause: Cause = create_test_cause();

        let request: RegisterUserRequest = RegisterUserRequest {
            bid_year: 2026,
            initials: String::from("EF"),
            name: String::from("Eve Foster"),
            area: String::from("North"),
            user_type: String::from("Dev-R"),
            crew: None,
            cumulative_natca_bu_date: String::from("2019-01-15"),
            natca_bu_date: String::from("2020-03-10"),
            eod_faa_date: String::from("2018-06-01"),
            service_computation_date: String::from("2018-06-01"),
            lottery_value: None,
        };

        let result: Result<ApiResult<RegisterUserResponse>, ApiError> =
            register_user(&metadata, &state, request, &admin, cause);
        assert!(result.is_ok());

        let final_state: State = result.unwrap().new_state;
        let response: ListUsersResponse = list_users(&final_state);

        assert_eq!(response.users.len(), 1);
        assert_eq!(response.users[0].initials, "EF");
        assert_eq!(response.users[0].name, "Eve Foster");
        assert_eq!(response.users[0].crew, None);
    }
}
