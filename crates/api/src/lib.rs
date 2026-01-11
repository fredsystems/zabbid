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

#[cfg(test)]
mod tests;

use zab_bid::{
    BootstrapMetadata, BootstrapResult, Command, CoreError, State, TransitionResult, apply,
    apply_bootstrap, validate_area_exists, validate_bid_year_exists,
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
    /// A requested resource was not found.
    ResourceNotFound {
        /// The type of resource that was not found.
        resource_type: String,
        /// A human-readable description of what was not found.
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
            Self::ResourceNotFound {
                resource_type,
                message,
            } => {
                write!(f, "{resource_type} not found: {message}")
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
        DomainError::BidYearNotFound(year) => ApiError::ResourceNotFound {
            resource_type: String::from("Bid year"),
            message: format!("Bid year {year} does not exist"),
        },
        DomainError::AreaNotFound { bid_year, area } => ApiError::ResourceNotFound {
            resource_type: String::from("Area"),
            message: format!("Area '{area}' does not exist in bid year {bid_year}"),
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
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `request` - The list areas request
///
/// # Returns
///
/// * `Ok(ListAreasResponse)` containing all areas for the bid year
/// * `Err(ApiError)` if the bid year does not exist
///
/// # Errors
///
/// Returns an error if the bid year has not been created.
pub fn list_areas(
    metadata: &BootstrapMetadata,
    request: &ListAreasRequest,
) -> Result<ListAreasResponse, ApiError> {
    let bid_year: BidYear = BidYear::new(request.bid_year);

    // Validate bid year exists before querying
    validate_bid_year_exists(metadata, &bid_year).map_err(translate_domain_error)?;

    let areas: Vec<String> = metadata
        .areas
        .iter()
        .filter(|(by, _)| by.year() == bid_year.year())
        .map(|(_, area)| area.id().to_string())
        .collect();

    Ok(ListAreasResponse {
        bid_year: request.bid_year,
        areas,
    })
}

/// Lists all users for a given bid year and area.
///
/// This is a read-only operation that requires no authorization.
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `bid_year` - The bid year to list users for
/// * `area` - The area to list users for
/// * `state` - The current state for the bid year and area
///
/// # Returns
///
/// * `Ok(ListUsersResponse)` containing all users for the scope
/// * `Err(ApiError)` if the bid year or area does not exist
///
/// # Errors
///
/// Returns an error if:
/// - The bid year has not been created
/// - The area has not been created in the bid year
pub fn list_users(
    metadata: &BootstrapMetadata,
    bid_year: &BidYear,
    area: &Area,
    state: &State,
) -> Result<ListUsersResponse, ApiError> {
    // Validate bid year and area exist before processing
    validate_area_exists(metadata, bid_year, area).map_err(translate_domain_error)?;

    let users: Vec<UserInfo> = state
        .users
        .iter()
        .map(|user| UserInfo {
            initials: user.initials.value().to_string(),
            name: user.name.clone(),
            crew: user.crew.as_ref().map(Crew::number),
        })
        .collect();

    Ok(ListUsersResponse {
        bid_year: state.bid_year.year(),
        area: state.area.id().to_string(),
        users,
    })
}

/// Gets the current state for a given bid year and area.
///
/// This is a read-only operation that requires no authorization.
/// This function validates that the bid year and area exist before
/// attempting to load state from persistence.
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `bid_year` - The bid year to get state for
/// * `area` - The area to get state for
/// * `state` - The current state (if it exists)
///
/// # Returns
///
/// * `Ok(State)` - The current state for the scope
/// * `Err(ApiError)` if the bid year or area does not exist
///
/// # Errors
///
/// Returns an error if:
/// - The bid year has not been created
/// - The area has not been created in the bid year
pub fn get_current_state(
    metadata: &BootstrapMetadata,
    bid_year: &BidYear,
    area: &Area,
    state: State,
) -> Result<State, ApiError> {
    // Validate bid year and area exist before returning state
    validate_area_exists(metadata, bid_year, area).map_err(translate_domain_error)?;

    Ok(state)
}

/// Gets the historical state for a given bid year and area at a specific timestamp.
///
/// This is a read-only operation that requires no authorization.
/// This function validates that the bid year and area exist before
/// attempting to load historical state from persistence.
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `bid_year` - The bid year to get state for
/// * `area` - The area to get state for
/// * `state` - The historical state (if it exists)
///
/// # Returns
///
/// * `Ok(State)` - The historical state for the scope at the timestamp
/// * `Err(ApiError)` if the bid year or area does not exist
///
/// # Errors
///
/// Returns an error if:
/// - The bid year has not been created
/// - The area has not been created in the bid year
pub fn get_historical_state(
    metadata: &BootstrapMetadata,
    bid_year: &BidYear,
    area: &Area,
    state: State,
) -> Result<State, ApiError> {
    // Validate bid year and area exist before returning state
    validate_area_exists(metadata, bid_year, area).map_err(translate_domain_error)?;

    Ok(state)
}
