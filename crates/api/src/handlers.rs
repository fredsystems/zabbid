// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! API handler functions for state-changing and read-only operations.

use num_traits::cast::ToPrimitive;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use zab_bid::{
    BootstrapMetadata, BootstrapResult, Command, State, TransitionResult, apply, apply_bootstrap,
    validate_area_exists, validate_bid_year_exists,
};
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::{
    Area, BidSchedule, BidYear, BidYearLifecycle, CanonicalBidYear, Crew, DomainError, Initials,
    LeaveAccrualResult, LeaveAvailabilityResult, LeaveUsage, RoundGroup, SeniorityData, UserType,
    calculate_leave_accrual, calculate_leave_availability,
};
use zab_bid_persistence::{OperatorData, SqlitePersistence};

use crate::auth::{AuthenticatedActor, AuthenticationService, AuthorizationService, Role};
use crate::csv_preview::{CsvRowResult, preview_csv_users as preview_csv_users_impl};
use crate::error::{ApiError, AuthError, translate_core_error, translate_domain_error};
use crate::password_policy::PasswordPolicy;
use crate::request_response::{
    AdjustBidOrderRequest, AdjustBidOrderResponse, AdjustBidWindowRequest, AdjustBidWindowResponse,
    AreaCompletenessInfo, AssignAreaRoundGroupRequest, AssignAreaRoundGroupResponse,
    BidOrderPositionInfo, BidScheduleInfo, BidStatusHistoryInfo, BidStatusInfo,
    BidYearCompletenessInfo, BidYearInfo, BlockingReason, BulkUpdateBidStatusRequest,
    BulkUpdateBidStatusResponse, ChangePasswordRequest, ChangePasswordResponse,
    ConfirmReadyToBidRequest, ConfirmReadyToBidResponse, CreateAreaRequest, CreateBidYearRequest,
    CreateOperatorRequest, CreateOperatorResponse, CsvImportRowResult, CsvImportRowStatus,
    CsvRowPreview, CsvRowStatus, DeleteOperatorRequest, DeleteOperatorResponse,
    DisableOperatorRequest, DisableOperatorResponse, EnableOperatorRequest, EnableOperatorResponse,
    GetActiveBidYearResponse, GetBidOrderPreviewResponse, GetBidScheduleResponse,
    GetBidStatusForAreaRequest, GetBidStatusForAreaResponse, GetBidStatusRequest,
    GetBidStatusResponse, GetBidYearReadinessResponse, GetBootstrapCompletenessResponse,
    GetLeaveAvailabilityResponse, GlobalCapabilities, ImportCsvUsersRequest,
    ImportCsvUsersResponse, ListAreasRequest, ListAreasResponse, ListBidYearsResponse,
    ListOperatorsResponse, ListUsersResponse, LoginRequest, LoginResponse, OperatorCapabilities,
    OperatorInfo, OverrideAreaAssignmentRequest, OverrideAreaAssignmentResponse,
    OverrideBidOrderRequest, OverrideBidOrderResponse, OverrideBidWindowRequest,
    OverrideBidWindowResponse, OverrideEligibilityRequest, OverrideEligibilityResponse,
    PreviewCsvUsersRequest, PreviewCsvUsersResponse, ReadinessDetailsInfo,
    RecalculateBidWindowsRequest, RecalculateBidWindowsResponse, RegisterUserRequest,
    ResetPasswordRequest, ResetPasswordResponse, ReviewNoBidUserResponse, SeniorityInputsInfo,
    SetActiveBidYearRequest, SetActiveBidYearResponse, SetBidScheduleRequest,
    SetBidScheduleResponse, SetExpectedAreaCountRequest, SetExpectedAreaCountResponse,
    SetExpectedUserCountRequest, SetExpectedUserCountResponse, TransitionBidStatusRequest,
    TransitionBidStatusResponse, TransitionToBiddingActiveRequest,
    TransitionToBiddingActiveResponse, TransitionToBiddingClosedRequest,
    TransitionToBiddingClosedResponse, TransitionToBootstrapCompleteRequest,
    TransitionToBootstrapCompleteResponse, TransitionToCanonicalizedRequest,
    TransitionToCanonicalizedResponse, UpdateAreaRequest, UpdateAreaResponse,
    UpdateBidYearMetadataRequest, UpdateBidYearMetadataResponse, UpdateUserRequest,
    UpdateUserResponse, UserCapabilities, UserInfo, WhoAmIResponse,
};
use zab_bid_persistence::PersistenceError;

/// Internal result type for user registration before ID population.
///
/// This is not an HTTP response type. The server layer must populate
/// the canonical IDs after persistence before constructing the final
/// `RegisterUserResponse`.
#[derive(Debug, Clone)]
pub struct RegisterUserResult {
    /// The bid year the user was registered for (display value).
    pub bid_year: u16,
    /// The user's initials.
    pub initials: String,
    /// The user's name.
    pub name: String,
    /// A success message.
    pub message: String,
}

/// Resolves the active bid year from persistence.
///
/// This function ensures that exactly one bid year is active.
///
/// # Arguments
///
/// * `persistence` - The persistence layer to query
///
/// # Returns
///
/// * `Ok(BidYear)` - The active bid year
/// * `Err(ApiError)` - If no active bid year exists
///
/// # Errors
///
/// Returns an error if:
/// - No active bid year is set
/// - Database query fails
fn resolve_active_bid_year(persistence: &mut SqlitePersistence) -> Result<BidYear, ApiError> {
    let year: u16 = persistence.get_active_bid_year().map_err(|e| match e {
        zab_bid_persistence::PersistenceError::NotFound(_) => {
            translate_domain_error(zab_bid_domain::DomainError::NoActiveBidYear)
        }
        _ => ApiError::Internal {
            message: format!("Failed to query active bid year: {e}"),
        },
    })?;

    Ok(BidYear::new(year))
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
/// * `metadata` - The current bootstrap metadata
/// * `state` - The current system state
/// * `request` - The API request to register a user
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `cause` - The cause or reason for this action
///
/// # Returns
///
/// * `Ok(ApiResult<RegisterUserResult>)` on success with internal result
/// * `Err(ApiError)` if unauthorized, the request is invalid, or a domain rule is violated
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - Any field validation fails
/// - The initials are already in use within the bid year
pub fn register_user(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    state: &State,
    request: RegisterUserRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<ApiResult<RegisterUserResult>, ApiError> {
    // Enforce authorization before executing command
    AuthorizationService::authorize_register_user(authenticated_actor)?;

    // Resolve the active bid year from canonical state
    let bid_year: BidYear = resolve_active_bid_year(persistence)?;

    // Enforce lifecycle constraints: user registration blocked after Canonicalized
    // Get bid_year_id from metadata (if bid year has no ID, assume Draft state and allow)
    if let Some(bid_year_id) = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == bid_year.year())
        .and_then(BidYear::bid_year_id)
    {
        let lifecycle_state_str: String =
            persistence
                .get_lifecycle_state(bid_year_id)
                .map_err(|e| ApiError::Internal {
                    message: format!("Failed to get lifecycle state: {e}"),
                })?;

        let lifecycle_state: BidYearLifecycle = lifecycle_state_str
            .parse()
            .map_err(translate_domain_error)?;

        if lifecycle_state.is_locked() {
            return Err(ApiError::DomainRuleViolation {
                rule: String::from("user_registration_lifecycle"),
                message: format!(
                    "Cannot register user in state '{lifecycle_state}': structural changes locked after confirmation"
                ),
            });
        }
    }

    // Convert authenticated actor to audit actor with operator information
    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    // Translate API request into domain types
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
        initials: initials.clone(),
        name: request.name.clone(),
        area,
        user_type,
        crew,
        seniority_data,
    };

    // Apply command via core transition
    let transition_result: TransitionResult =
        apply(metadata, state, &bid_year, command, actor, cause).map_err(translate_core_error)?;

    // Return internal result (IDs will be populated by server layer after persistence)
    let result: RegisterUserResult = RegisterUserResult {
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
        response: result,
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
/// * `metadata` - The current bootstrap metadata
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
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    state: &State,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<TransitionResult, ApiError> {
    // Enforce authorization before executing command
    AuthorizationService::authorize_checkpoint(authenticated_actor)?;

    // Resolve the active bid year from canonical state
    let active_bid_year: BidYear = resolve_active_bid_year(persistence)?;

    // Convert authenticated actor to audit actor with operator information
    let actor: Actor = authenticated_actor.to_audit_actor(operator);

    // Create and apply checkpoint command
    let command: Command = Command::Checkpoint;
    let transition_result: TransitionResult =
        apply(metadata, state, &active_bid_year, command, actor, cause)
            .map_err(translate_core_error)?;

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
/// * `metadata` - The current bootstrap metadata
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
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    state: &State,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<TransitionResult, ApiError> {
    // Enforce authorization before executing command
    AuthorizationService::authorize_finalize(authenticated_actor)?;

    // Resolve the active bid year from canonical state
    let active_bid_year: BidYear = resolve_active_bid_year(persistence)?;

    // Convert authenticated actor to audit actor with operator information
    let actor: Actor = authenticated_actor.to_audit_actor(operator);

    // Create and apply finalize command
    let command: Command = Command::Finalize;
    let transition_result: TransitionResult =
        apply(metadata, state, &active_bid_year, command, actor, cause)
            .map_err(translate_core_error)?;

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
/// * `metadata` - The current bootstrap metadata
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
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    state: &State,
    target_event_id: i64,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<TransitionResult, ApiError> {
    // Enforce authorization before executing command
    AuthorizationService::authorize_rollback(authenticated_actor)?;

    // Resolve the active bid year from canonical state
    let active_bid_year: BidYear = resolve_active_bid_year(persistence)?;

    // Convert authenticated actor to audit actor with operator information
    let actor: Actor = authenticated_actor.to_audit_actor(operator);

    // Create and apply rollback command
    let command: Command = Command::RollbackToEventId { target_event_id };
    let transition_result: TransitionResult =
        apply(metadata, state, &active_bid_year, command, actor, cause)
            .map_err(translate_core_error)?;

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
    operator: &OperatorData,
    cause: Cause,
) -> Result<BootstrapResult, ApiError> {
    // Enforce authorization - only admins can create bid years
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("create_bid_year"),
            required_role: String::from("Admin"),
        });
    }

    // Convert authenticated actor to audit actor with operator information
    let actor: Actor = authenticated_actor.to_audit_actor(operator);

    // Create command with canonical metadata
    let command: Command = Command::CreateBidYear {
        year: request.year,
        start_date: request.start_date,
        num_pay_periods: request.num_pay_periods,
    };

    // Apply command via core bootstrap
    // Create a placeholder bid year for CreateBidYear command (it doesn't need an active bid year)
    let placeholder_bid_year = BidYear::new(request.year);
    let bootstrap_result: BootstrapResult =
        apply_bootstrap(metadata, &placeholder_bid_year, command, actor, cause)
            .map_err(translate_core_error)?;

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
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &CreateAreaRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<BootstrapResult, ApiError> {
    // Enforce authorization - only admins can create areas
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("create_area"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve the active bid year from canonical state
    let active_bid_year: BidYear = resolve_active_bid_year(persistence)?;

    // Enforce lifecycle constraints: area creation blocked after Canonicalized
    // Get bid_year_id from metadata (if bid year has no ID, assume Draft state and allow)
    if let Some(bid_year_id) = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == active_bid_year.year())
        .and_then(BidYear::bid_year_id)
    {
        let lifecycle_state_str: String =
            persistence
                .get_lifecycle_state(bid_year_id)
                .map_err(|e| ApiError::Internal {
                    message: format!("Failed to get lifecycle state: {e}"),
                })?;

        let lifecycle_state: BidYearLifecycle = lifecycle_state_str
            .parse()
            .map_err(translate_domain_error)?;

        if lifecycle_state.is_locked() {
            return Err(ApiError::DomainRuleViolation {
                rule: String::from("area_creation_lifecycle"),
                message: format!(
                    "Cannot create area in state '{lifecycle_state}': structural changes locked after confirmation"
                ),
            });
        }
    }

    // Convert authenticated actor to audit actor with operator information
    let actor: Actor = authenticated_actor.to_audit_actor(operator);

    // Create command
    let command: Command = Command::CreateArea {
        area_id: request.area_id.clone(),
    };

    // Apply command via core bootstrap
    let bootstrap_result: BootstrapResult =
        apply_bootstrap(metadata, &active_bid_year, command, actor, cause)
            .map_err(translate_core_error)?;

    Ok(bootstrap_result)
}

/// Lists all bid years with their canonical metadata.
///
/// This operation never fails and requires no authorization.
/// Returns an empty list if no bid years have been created.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata with bid year IDs
/// * `canonical_bid_years` - The list of canonical bid years from persistence
///
/// # Returns
///
/// A response containing all bid years with canonical metadata and IDs.
///
/// # Errors
///
/// Returns an error if end date derivation fails due to date arithmetic overflow.
pub fn list_bid_years(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    canonical_bid_years: &[CanonicalBidYear],
) -> Result<ListBidYearsResponse, ApiError> {
    let bid_years: Result<Vec<BidYearInfo>, ApiError> = canonical_bid_years
        .iter()
        .map(|c| {
            let end_date: time::Date = c.end_date().map_err(translate_domain_error)?;

            // Extract bid_year_id from metadata by matching the year
            let bid_year_id: i64 = metadata
                .bid_years
                .iter()
                .find(|by| by.year() == c.year())
                .and_then(zab_bid_domain::BidYear::bid_year_id)
                .ok_or_else(|| ApiError::Internal {
                    message: format!(
                        "Bid year {} exists in canonical data but has no ID in metadata",
                        c.year()
                    ),
                })?;

            // Fetch lifecycle state from persistence
            let lifecycle_state: String = persistence
                .get_lifecycle_state(bid_year_id)
                .unwrap_or_else(|_| String::from("Draft"));

            // Fetch metadata (label and notes) from persistence
            let (label, notes) = persistence
                .get_bid_year_metadata(bid_year_id)
                .unwrap_or((None, None));

            // Fetch bid schedule from persistence
            let bid_schedule = persistence.get_bid_schedule(bid_year_id).ok().and_then(
                |(tz, sd, wst, wet, bpd)| {
                    // Only construct BidScheduleInfo if all fields are present
                    if let (
                        Some(timezone),
                        Some(start_date),
                        Some(window_start_time),
                        Some(window_end_time),
                        Some(bidders_per_day),
                    ) = (tz, sd, wst, wet, bpd)
                    {
                        Some(BidScheduleInfo {
                            timezone,
                            start_date,
                            window_start_time,
                            window_end_time,
                            bidders_per_day: bidders_per_day.cast_unsigned(),
                        })
                    } else {
                        None
                    }
                },
            );

            Ok(BidYearInfo {
                bid_year_id,
                year: c.year(),
                start_date: c.start_date(),
                num_pay_periods: c.num_pay_periods(),
                end_date,
                area_count: 0,       // Will be populated by server layer
                total_user_count: 0, // Will be populated by server layer
                lifecycle_state,
                label,
                notes,
                bid_schedule,
            })
        })
        .collect();

    Ok(ListBidYearsResponse {
        bid_years: bid_years?,
    })
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
    // Resolve bid_year_id to BidYear from metadata
    let bid_year: &BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(request.bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {} not found", request.bid_year_id),
        })?;

    let areas: Vec<crate::request_response::AreaInfo> = metadata
        .areas
        .iter()
        .filter(|(by, _)| by.year() == bid_year.year())
        .map(|(_, area)| {
            // Extract area_id - all persisted areas must have IDs
            let area_id: i64 = area.area_id().ok_or_else(|| ApiError::Internal {
                message: format!(
                    "Area '{}' in bid year {} has no ID",
                    area.area_code(),
                    bid_year.year()
                ),
            })?;

            Ok(crate::request_response::AreaInfo {
                area_id,
                area_code: area.area_code().to_string(),
                area_name: area.area_name().map(String::from),
                user_count: 0, // Will be populated by server layer with actual counts
                is_system_area: area.is_system_area(),
                round_group_id: area.round_group_id(),
                round_group_name: None, // Will be populated by server layer with actual names
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(ListAreasResponse {
        bid_year_id: request.bid_year_id,
        bid_year: bid_year.year(),
        areas,
    })
}

/// Checks if an area is a system area and returns an error if it is.
///
/// # Errors
///
/// Returns an error if the area is a system area.
fn validate_not_system_area(
    persistence: &mut SqlitePersistence,
    area_id: i64,
    area_code: &str,
) -> Result<(), ApiError> {
    let is_system = persistence
        .is_system_area(area_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to check system area status: {e}"),
        })?;

    if is_system {
        return Err(translate_domain_error(
            DomainError::CannotRenameSystemArea {
                area_code: area_code.to_string(),
            },
        ));
    }

    Ok(())
}

/// Validates that the lifecycle state allows area metadata editing.
///
/// # Errors
///
/// Returns an error if the lifecycle state is >= Canonicalized.
fn validate_lifecycle_allows_area_edit(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
    bid_year: u16,
) -> Result<(), ApiError> {
    let lifecycle_state_str =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    let lifecycle_state = zab_bid_domain::BidYearLifecycle::from_str(&lifecycle_state_str)
        .map_err(|_| ApiError::Internal {
            message: format!("Invalid lifecycle state: {lifecycle_state_str}"),
        })?;

    if matches!(
        lifecycle_state,
        zab_bid_domain::BidYearLifecycle::Canonicalized
            | zab_bid_domain::BidYearLifecycle::BiddingActive
            | zab_bid_domain::BidYearLifecycle::BiddingClosed
    ) {
        return Err(translate_domain_error(
            DomainError::CannotEditAreaAfterCanonicalization {
                bid_year,
                lifecycle_state: lifecycle_state_str,
            },
        ));
    }

    Ok(())
}

/// Updates area metadata (display name only).
///
/// Phase 26C: Enables editing of area display names with lifecycle-aware gating.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `request` - The update area request
/// * `authenticated_actor` - The authenticated actor (must be Admin)
/// * `operator` - The operator data
///
/// # Returns
///
/// * `Ok(UpdateAreaResponse)` on success
/// * `Err(ApiError)` if authorization fails, lifecycle state prevents editing,
///   or the area is a system area
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an Admin
/// - The area is a system area (immutable)
/// - The bid year lifecycle state is >= Canonicalized
/// - The area does not exist
pub fn update_area(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &UpdateAreaRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<UpdateAreaResponse, ApiError> {
    // Enforce authorization - only admins can update areas
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("update_area"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve area from metadata
    let area = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(request.area_id))
        .map(|(_, a)| a)
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("Area"),
            message: format!("Area with ID {} not found", request.area_id),
        })?;

    // Get the bid year for this area
    let bid_year = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(request.area_id))
        .map(|(by, _)| by)
        .ok_or_else(|| ApiError::Internal {
            message: format!("Area {} has no associated bid year", request.area_id),
        })?;

    // Get bid_year_id for lifecycle check
    let bid_year_id = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == bid_year.year())
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| ApiError::Internal {
            message: format!("Bid year {} has no ID", bid_year.year()),
        })?;

    // Validate this is not a system area
    validate_not_system_area(persistence, request.area_id, area.area_code())?;

    // Validate lifecycle state allows editing
    validate_lifecycle_allows_area_edit(persistence, bid_year_id, bid_year.year())?;

    // Update the area name in the canonical table
    persistence
        .update_area_name(request.area_id, request.area_name.as_deref())
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update area name: {e}"),
        })?;

    // Create audit event for the metadata change
    let actor = authenticated_actor.to_audit_actor(operator);
    let cause = Cause::new(
        String::from("operator_action"),
        String::from("Area metadata update via admin interface"),
    );

    let before = StateSnapshot::new(format!(
        "area_name={}",
        area.area_name().unwrap_or("(none)")
    ));
    let after = StateSnapshot::new(format!(
        "area_name={}",
        request.area_name.as_deref().unwrap_or("(none)")
    ));

    let action = Action::new(
        String::from("UpdateAreaMetadata"),
        Some(format!(
            "Updated display name for area '{}' to '{}'",
            area.area_code(),
            request.area_name.as_deref().unwrap_or("(none)")
        )),
    );

    let audit_event = AuditEvent::new(
        actor,
        cause,
        action,
        before,
        after,
        bid_year.clone(),
        area.clone(),
    );

    persistence
        .persist_audit_event(&audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(UpdateAreaResponse {
        bid_year_id,
        bid_year: bid_year.year(),
        area_id: request.area_id,
        area_code: area.area_code().to_string(),
        area_name: request.area_name.clone(),
        message: format!(
            "Area '{}' display name updated successfully",
            area.area_code()
        ),
    })
}

/// Assigns a round group to an area.
///
/// This operation allows admins to assign or clear a round group for a non-system area.
/// Assignment is only permitted before the bid year is canonicalized.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `area_id` - The canonical area identifier
/// * `request` - The assignment request
/// * `authenticated_actor` - The authenticated actor
/// * `operator` - The operator data
///
/// # Returns
///
/// * `Ok(AssignAreaRoundGroupResponse)` on success
/// * `Err(ApiError)` if validation fails or the operation is not permitted
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an admin
/// - The area does not exist
/// - The area is a system area
/// - The lifecycle state does not allow editing
/// - The round group does not exist (when assigning)
/// - The round group is in a different bid year
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
pub fn assign_area_round_group(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    area_id: i64,
    request: &AssignAreaRoundGroupRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<AssignAreaRoundGroupResponse, ApiError> {
    // Enforce authorization - only admins can assign round groups
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("assign_area_round_group"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve area from metadata
    let area = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(area_id))
        .map(|(_, a)| a)
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("Area"),
            message: format!("Area with ID {area_id} not found"),
        })?;

    // Get the bid year for this area
    let bid_year = metadata
        .areas
        .iter()
        .find(|(_, a)| a.area_id() == Some(area_id))
        .map(|(by, _)| by)
        .ok_or_else(|| ApiError::Internal {
            message: format!("Area {area_id} has no associated bid year"),
        })?;

    // Get bid_year_id for lifecycle check
    let bid_year_id = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == bid_year.year())
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| ApiError::Internal {
            message: format!("Bid year {} has no ID", bid_year.year()),
        })?;

    // Validate this is not a system area
    validate_not_system_area(persistence, area_id, area.area_code())?;

    // Validate lifecycle state allows editing
    validate_lifecycle_allows_area_edit(persistence, bid_year_id, bid_year.year())?;

    // Get current round_group_id before update for audit trail
    let current_round_group_id =
        persistence
            .get_area_round_group_id(area_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get current round group assignment: {e}"),
            })?;

    // If assigning a round group (not clearing), validate it exists and is in same bid year
    if let Some(round_group_id) = request.round_group_id {
        let round_group = persistence
            .get_round_group(round_group_id)
            .map_err(|e| match e {
                PersistenceError::NotFound(_) => ApiError::ResourceNotFound {
                    resource_type: String::from("Round Group"),
                    message: format!("Round group with ID {round_group_id} not found"),
                },
                _ => ApiError::Internal {
                    message: format!("Failed to validate round group: {e}"),
                },
            })?;

        // Ensure round group is in the same bid year
        let rg_bid_year_id =
            round_group
                .bid_year()
                .bid_year_id()
                .ok_or_else(|| ApiError::Internal {
                    message: format!("Round group {round_group_id} has no bid year ID"),
                })?;

        if rg_bid_year_id != bid_year_id {
            return Err(ApiError::InvalidInput {
                field: String::from("round_group_id"),
                message: format!(
                    "Round group {round_group_id} belongs to a different bid year (expected {bid_year_id}, got {rg_bid_year_id})"
                ),
            });
        }
    }

    // Update the area's round group assignment
    persistence
        .update_area_round_group(area_id, request.round_group_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update area round group: {e}"),
        })?;

    // Create audit event for the assignment
    let actor = authenticated_actor.to_audit_actor(operator);
    let cause = Cause::new(
        String::from("operator_action"),
        String::from("Area round group assignment via admin interface"),
    );

    let before = StateSnapshot::new(format!(
        "round_group_id={}",
        current_round_group_id.map_or_else(|| String::from("null"), |id: i64| id.to_string())
    ));
    let after = StateSnapshot::new(format!(
        "round_group_id={}",
        request
            .round_group_id
            .map_or_else(|| String::from("null"), |id| id.to_string())
    ));

    let action_description = request.round_group_id.map_or_else(
        || {
            format!(
                "Cleared round group assignment for area '{}'",
                area.area_code()
            )
        },
        |rg_id| {
            format!(
                "Assigned round group {rg_id} to area '{}'",
                area.area_code()
            )
        },
    );

    let action = Action::new(
        String::from("AssignAreaRoundGroup"),
        Some(action_description),
    );

    let audit_event = AuditEvent::new(
        actor,
        cause,
        action,
        before,
        after,
        bid_year.clone(),
        area.clone(),
    );

    persistence
        .persist_audit_event(&audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    let message = request.round_group_id.map_or_else(
        || {
            format!(
                "Successfully cleared round group assignment for area '{}'",
                area.area_code()
            )
        },
        |rg_id| {
            format!(
                "Successfully assigned round group {rg_id} to area '{}'",
                area.area_code()
            )
        },
    );

    Ok(AssignAreaRoundGroupResponse {
        bid_year_id,
        bid_year: bid_year.year(),
        area_id,
        area_code: area.area_code().to_string(),
        round_group_id: request.round_group_id,
        message,
    })
}

/// Lists all users in a given bid year and area with leave balances and capabilities.
///
/// This is a read-only operation. No authorization check is performed.
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `canonical_bid_years` - The list of canonical bid years
/// * `bid_year` - The bid year to list users for
/// * `area` - The area to list users for
/// * `state` - The current state for this scope
/// * `authenticated_actor` - The authenticated actor (for capability computation)
/// * `actor_operator` - The authenticated operator's data (for capability computation)
///
/// # Returns
///
/// * `Ok(ListUsersResponse)` containing all users for the scope with capabilities
/// * `Err(ApiError)` if the bid year or area does not exist
///
/// # Errors
///
/// Returns an error if:
/// - The bid year has not been created
/// - The area has not been created in the bid year
///
/// Phase 26A: Added `lifecycle_state` parameter for lifecycle-aware capability computation.
/// This brings the parameter count to 8, which exceeds clippy's default limit of 7.
/// Grouping these into a struct would add complexity without improving clarity.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn list_users(
    metadata: &BootstrapMetadata,
    canonical_bid_years: &[CanonicalBidYear],
    bid_year: &BidYear,
    area: &Area,
    state: &State,
    authenticated_actor: &AuthenticatedActor,
    actor_operator: &OperatorData,
    lifecycle_state: zab_bid_domain::BidYearLifecycle,
) -> Result<ListUsersResponse, ApiError> {
    // Validate bid year and area exist before processing
    validate_area_exists(metadata, bid_year, area).map_err(translate_domain_error)?;

    // Extract bid_year_id from metadata
    let bid_year_id: i64 = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == bid_year.year())
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| ApiError::Internal {
            message: format!(
                "Bid year {} exists but has no ID in metadata",
                bid_year.year()
            ),
        })?;

    // Extract area_id from metadata
    let area_id: i64 = metadata
        .areas
        .iter()
        .filter(|(by, _)| by.year() == bid_year.year())
        .find(|(_, a)| a.area_code() == area.id())
        .and_then(|(_, a)| a.area_id())
        .ok_or_else(|| ApiError::Internal {
            message: format!(
                "Area '{}' in bid year {} exists but has no ID in metadata",
                area.id(),
                bid_year.year()
            ),
        })?;

    // Find the canonical bid year metadata for leave calculations
    let canonical_bid_year: &CanonicalBidYear = canonical_bid_years
        .iter()
        .find(|c| c.year() == bid_year.year())
        .ok_or_else(|| {
            translate_domain_error(zab_bid_domain::DomainError::InvalidBidYear(format!(
                "Bid year {} not found",
                bid_year.year()
            )))
        })?;

    let users: Result<Vec<UserInfo>, ApiError> = state
        .users
        .iter()
        .map(|user| {
            // Verify user_id is present (data integrity check)
            let user_id: i64 = user.user_id.ok_or_else(|| ApiError::Internal {
                message: format!(
                    "User '{}' loaded from database is missing user_id (data integrity violation)",
                    user.initials.value()
                ),
            })?;

            // Calculate leave accrual for this user
            let leave_accrual_result: LeaveAccrualResult =
                calculate_leave_accrual(user, canonical_bid_year).unwrap_or_else(|_| {
                    LeaveAccrualResult {
                        total_hours: 0,
                        total_days: 0,
                        rounded_up: false,
                        breakdown: vec![],
                    }
                });

            let earned_hours: u16 = leave_accrual_result.total_hours;
            let earned_days: u16 = leave_accrual_result.total_days;

            // Calculate availability
            // For Phase 11, we don't have bid records yet, so usage is empty
            let availability: LeaveAvailabilityResult =
                calculate_leave_availability(&leave_accrual_result, std::iter::empty())
                    .unwrap_or_else(|_| LeaveAvailabilityResult {
                        earned_hours,
                        earned_days,
                        used_hours: 0,
                        remaining_hours: i32::from(earned_hours),
                        remaining_days: i32::from(earned_days),
                        is_exhausted: false,
                        is_overdrawn: false,
                    });

            // Compute user capabilities
            let capabilities: UserCapabilities = crate::capabilities::compute_user_capabilities(
                authenticated_actor,
                actor_operator,
                lifecycle_state,
            )
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to compute user capabilities: {e}"),
            })?;

            Ok(UserInfo {
                user_id,
                bid_year_id,
                area_id,
                initials: user.initials.value().to_string(),
                name: user.name.clone(),
                crew: user.crew.as_ref().map(Crew::number),
                user_type: user.user_type.as_str().to_string(),
                cumulative_natca_bu_date: user.seniority_data.cumulative_natca_bu_date.clone(),
                natca_bu_date: user.seniority_data.natca_bu_date.clone(),
                eod_faa_date: user.seniority_data.eod_faa_date.clone(),
                service_computation_date: user.seniority_data.service_computation_date.clone(),
                lottery_value: user.seniority_data.lottery_value,
                earned_hours,
                earned_days,
                remaining_hours: availability.remaining_hours,
                remaining_days: availability.remaining_days,
                is_exhausted: availability.is_exhausted,
                is_overdrawn: availability.is_overdrawn,
                excluded_from_bidding: user.excluded_from_bidding,
                excluded_from_leave_calculation: user.excluded_from_leave_calculation,
                no_bid_reviewed: user.no_bid_reviewed,
                capabilities,
            })
        })
        .collect();

    Ok(ListUsersResponse {
        bid_year_id,
        bid_year: state.bid_year.year(),
        area_id,
        area_code: state.area.id().to_string(),
        users: users?,
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

/// Gets leave availability for a specific user.
///
/// This is a read-only operation that:
/// - Validates the bid year and area exist
/// - Finds the specified user
/// - Calculates leave accrual using Phase 9 logic
/// - Retrieves leave usage records (currently none exist in persistence)
/// - Calculates remaining leave availability
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `canonical_bid_year` - The canonical bid year for accrual calculation
/// * `area` - The area
/// * `initials` - The user's initials
/// * `state` - The current state
///
/// # Returns
///
/// * `Ok(GetLeaveAvailabilityResponse)` - The leave availability information
/// * `Err(ApiError)` if the bid year, area, or user does not exist
///
/// # Errors
///
/// Returns an error if:
/// - The bid year does not exist
/// - The area does not exist in the bid year
/// - The user does not exist in the area
/// - Leave accrual calculation fails
/// - Leave availability calculation fails
pub fn get_leave_availability(
    metadata: &BootstrapMetadata,
    canonical_bid_year: &CanonicalBidYear,
    area: &Area,
    initials: &Initials,
    state: &State,
) -> Result<GetLeaveAvailabilityResponse, ApiError> {
    let bid_year: BidYear = BidYear::new(canonical_bid_year.year());

    // Validate bid year and area exist
    validate_area_exists(metadata, &bid_year, area).map_err(translate_domain_error)?;

    // Extract bid_year_id from metadata
    let bid_year_id: i64 = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == bid_year.year())
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| ApiError::Internal {
            message: format!(
                "Bid year {} exists but has no ID in metadata",
                bid_year.year()
            ),
        })?;

    // Find the user
    let user = state
        .users
        .iter()
        .find(|u| u.initials == *initials)
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("User"),
            message: format!(
                "User with initials '{}' not found in bid year {} area {}",
                initials.value(),
                bid_year.year(),
                area.id()
            ),
        })?;

    // Verify user_id is present (data integrity check)
    let user_id: i64 = user.user_id.ok_or_else(|| ApiError::Internal {
        message: format!(
            "User '{}' loaded from database is missing user_id (data integrity violation)",
            user.initials.value()
        ),
    })?;

    // Calculate leave accrual using Phase 9
    let accrual =
        calculate_leave_accrual(user, canonical_bid_year).map_err(translate_domain_error)?;

    // Retrieve leave usage records
    // Note: For Phase 10, no persistence for leave usage exists yet.
    // We pass an empty iterator, which means all earned leave is available.
    let usage_records: Vec<LeaveUsage> = Vec::new();

    // Calculate leave availability
    let availability: LeaveAvailabilityResult =
        calculate_leave_availability(&accrual, usage_records).map_err(translate_domain_error)?;

    // Build explanation
    let explanation: String = format!(
        "Leave accrual calculated for user '{}' in bid year {}. \
         Earned: {} hours ({} days). Used: {} hours. \
         Remaining: {} hours ({} days).{}{}",
        initials.value(),
        bid_year.year(),
        availability.earned_hours,
        availability.earned_days,
        availability.used_hours,
        availability.remaining_hours,
        availability.remaining_days,
        if availability.is_exhausted {
            " Leave fully exhausted."
        } else {
            ""
        },
        if availability.is_overdrawn {
            " Leave balance is overdrawn."
        } else {
            ""
        }
    );

    Ok(GetLeaveAvailabilityResponse {
        bid_year_id,
        bid_year: bid_year.year(),
        user_id,
        initials: initials.value().to_string(),
        earned_hours: availability.earned_hours,
        earned_days: availability.earned_days,
        used_hours: availability.used_hours,
        remaining_hours: availability.remaining_hours,
        remaining_days: availability.remaining_days,
        is_exhausted: availability.is_exhausted,
        is_overdrawn: availability.is_overdrawn,
        explanation,
    })
}

/// Gets a comprehensive bootstrap status summary.
///
/// This is a read-only operation that provides aggregated information
/// about all bid years and areas in the system.
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `area_counts` - Area counts per bid year
/// * `user_counts_by_year` - Total user counts per bid year
/// * `user_counts_by_area` - User counts per (`bid_year`, `area_id`)
///
/// # Returns
///
/// * `Ok(BootstrapStatusResponse)` containing all system status information
///
/// # Errors
///
/// This function does not currently return errors, but the return type supports
/// future error conditions.
///
/// This endpoint is useful for operators to get a complete picture of the
/// system state in a single API call.
pub fn get_bootstrap_status(
    metadata: &BootstrapMetadata,
    area_counts: &[(u16, usize)],
    user_counts_by_year: &[(u16, usize)],
    user_counts_by_area: &[(u16, String, usize)],
) -> Result<crate::request_response::BootstrapStatusResponse, ApiError> {
    use crate::request_response::{AreaStatusInfo, BidYearStatusInfo, BootstrapStatusResponse};

    // Build bid year summaries
    let bid_years: Vec<BidYearStatusInfo> = metadata
        .bid_years
        .iter()
        .map(|bid_year| {
            let year: u16 = bid_year.year();
            let bid_year_id: i64 = bid_year.bid_year_id().ok_or_else(|| ApiError::Internal {
                message: format!("Bid year {year} has no ID in metadata"),
            })?;
            let area_count: usize = area_counts
                .iter()
                .find(|(y, _)| *y == year)
                .map_or(0, |(_, count)| *count);
            let total_user_count: usize = user_counts_by_year
                .iter()
                .find(|(y, _)| *y == year)
                .map_or(0, |(_, count)| *count);

            Ok(BidYearStatusInfo {
                bid_year_id,
                year,
                area_count,
                total_user_count,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    // Build area summaries
    let areas: Vec<AreaStatusInfo> = metadata
        .areas
        .iter()
        .map(|(bid_year, area)| {
            let year: u16 = bid_year.year();
            let bid_year_id: i64 = metadata
                .bid_years
                .iter()
                .find(|by| by.year() == year)
                .and_then(zab_bid_domain::BidYear::bid_year_id)
                .ok_or_else(|| ApiError::Internal {
                    message: format!("Bid year {year} has no ID in metadata"),
                })?;
            let area_code: String = area.area_code().to_string();
            let area_id: i64 = area.area_id().ok_or_else(|| ApiError::Internal {
                message: format!("Area '{area_code}' in bid year {year} has no ID in metadata"),
            })?;
            let user_count: usize = user_counts_by_area
                .iter()
                .find(|(y, a, _)| *y == year && a == &area_code)
                .map_or(0, |(_, _, count)| *count);

            Ok(AreaStatusInfo {
                bid_year_id,
                bid_year: year,
                area_id,
                area_code,
                user_count,
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(BootstrapStatusResponse { bid_years, areas })
}

// ========================================================================
// Authentication Handlers (Phase 14)
// ========================================================================

/// Authenticates an operator and creates a session.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The login request
///
/// # Returns
///
/// * `Ok(LoginResponse)` on success with session token
/// * `Err(ApiError)` if authentication fails
///
/// # Errors
///
/// Returns an error if:
/// - The operator does not exist
/// - The operator is disabled
/// - Database operations fail
pub fn login(
    persistence: &mut SqlitePersistence,
    request: &LoginRequest,
) -> Result<LoginResponse, ApiError> {
    let (session_token, _authenticated_actor, operator): (
        String,
        AuthenticatedActor,
        OperatorData,
    ) = AuthenticationService::login(persistence, &request.login_name, &request.password)?;

    // Get session expiration from the session we just created
    let session: Option<zab_bid_persistence::SessionData> = persistence
        .get_session_by_token(&session_token)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to retrieve session: {e}"),
        })?;

    let expires_at: String = session
        .ok_or_else(|| ApiError::Internal {
            message: String::from("Session not found after creation"),
        })?
        .expires_at;

    Ok(LoginResponse {
        session_token,
        login_name: operator.login_name,
        display_name: operator.display_name,
        role: operator.role,
        expires_at,
    })
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
pub fn logout(persistence: &mut SqlitePersistence, session_token: &str) -> Result<(), ApiError> {
    AuthenticationService::logout(persistence, session_token)?;
    Ok(())
}

/// Returns the current operator's information with global capabilities.
///
/// # Arguments
///
/// * `persistence` - The persistence layer (for computing capabilities)
/// * `actor` - The authenticated actor
/// * `operator` - The operator data from the validated session
///
/// # Returns
///
/// * `Ok(WhoAmIResponse)` with operator information and capabilities
///
/// # Errors
///
/// Returns an error if capability computation fails.
pub fn whoami(
    _persistence: &mut SqlitePersistence,
    actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<WhoAmIResponse, ApiError> {
    let capabilities: GlobalCapabilities =
        crate::capabilities::compute_global_capabilities(actor, operator).map_err(|e| {
            ApiError::Internal {
                message: format!("Failed to compute global capabilities: {e}"),
            }
        })?;

    Ok(WhoAmIResponse {
        login_name: operator.login_name.clone(),
        display_name: operator.display_name.clone(),
        role: operator.role.clone(),
        is_disabled: operator.is_disabled,
        capabilities,
    })
}

/// Creates a new operator.
///
/// Only Admin actors may create operators.
/// Emits an audit event on success.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The create operator request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data for audit attribution
/// * `cause` - The cause for this action
///
/// # Returns
///
/// * `Ok(CreateOperatorResponse)` on success
/// * `Err(ApiError)` if unauthorized or creation fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The login name already exists
/// - The role is invalid
/// - Database operations fail
pub fn create_operator(
    persistence: &mut SqlitePersistence,
    request: CreateOperatorRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<CreateOperatorResponse, ApiError> {
    // Enforce authorization before executing command
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("create_operator"),
            required_role: String::from("Admin"),
        });
    }

    // Validate role
    if request.role != "Admin" && request.role != "Bidder" {
        return Err(ApiError::InvalidInput {
            field: String::from("role"),
            message: format!(
                "Invalid role: {}. Must be 'Admin' or 'Bidder'",
                request.role
            ),
        });
    }

    // Validate password policy
    let policy: PasswordPolicy = PasswordPolicy::default();
    policy.validate(
        &request.password,
        &request.password_confirmation,
        &request.login_name,
        &request.display_name,
    )?;

    // Create operator with validated password
    let operator_id: i64 = persistence
        .create_operator(
            &request.login_name,
            &request.display_name,
            &request.password,
            &request.role,
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to create operator: {e}"),
        })?;

    // Create audit event for operator lifecycle change
    let actor: Actor = Actor::with_operator(
        operator.operator_id.to_string(),
        String::from("operator"),
        operator.operator_id,
        operator.login_name.clone(),
        operator.display_name.clone(),
    );

    let action: Action = Action::new(
        String::from("CreateOperator"),
        Some(format!(
            "Created operator {} ({}) with role {}",
            request.login_name, request.display_name, request.role
        )),
    );

    let before: StateSnapshot = StateSnapshot::new(String::from("operator_does_not_exist"));
    let after: StateSnapshot = StateSnapshot::new(format!(
        "operator_id={},login_name={},role={}",
        operator_id, request.login_name, request.role
    ));

    // Phase 23B: Use global event for operator management
    let audit_event: AuditEvent = AuditEvent::new_global(actor, cause, action, before, after);

    // Persist audit event
    persistence
        .persist_audit_event(&audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(CreateOperatorResponse {
        operator_id,
        login_name: request.login_name,
        display_name: request.display_name,
        role: request.role,
    })
}

/// Lists all operators with per-operator capabilities.
///
/// Only Admin actors may list operators.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `actor_operator` - The authenticated operator's data
///
/// # Returns
///
/// * `Ok(ListOperatorsResponse)` with the list of operators and their capabilities
/// * `Err(ApiError)` if unauthorized or query fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - Database operations fail
pub fn list_operators(
    persistence: &mut SqlitePersistence,
    authenticated_actor: &AuthenticatedActor,
    actor_operator: &OperatorData,
) -> Result<ListOperatorsResponse, ApiError> {
    // Enforce authorization before executing command
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("list_operators"),
            required_role: String::from("Admin"),
        });
    }

    let operators: Vec<OperatorData> =
        persistence
            .list_operators()
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to list operators: {e}"),
            })?;

    let operator_infos: Result<Vec<OperatorInfo>, ApiError> = operators
        .into_iter()
        .map(|op| {
            let capabilities: OperatorCapabilities =
                crate::capabilities::compute_operator_capabilities(
                    authenticated_actor,
                    actor_operator,
                    &op,
                    persistence,
                )
                .map_err(|e| ApiError::Internal {
                    message: format!("Failed to compute operator capabilities: {e}"),
                })?;

            Ok(OperatorInfo {
                operator_id: op.operator_id,
                login_name: op.login_name,
                display_name: op.display_name,
                role: op.role,
                is_disabled: op.is_disabled,
                created_at: op.created_at,
                last_login_at: op.last_login_at,
                capabilities,
            })
        })
        .collect();

    Ok(ListOperatorsResponse {
        operators: operator_infos?,
    })
}

/// Disables an operator.
///
/// Only Admin actors may disable operators.
/// Emits an audit event on success.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The disable operator request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data for audit attribution
/// * `cause` - The cause for this action
///
/// # Returns
///
/// * `Ok(DisableOperatorResponse)` on success
/// * `Err(ApiError)` if unauthorized or operation fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The operator does not exist
/// - Database operations fail
pub fn disable_operator(
    persistence: &mut SqlitePersistence,
    request: DisableOperatorRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<DisableOperatorResponse, ApiError> {
    // Enforce authorization before executing command
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("disable_operator"),
            required_role: String::from("Admin"),
        });
    }

    // Get target operator to verify existence and get details for audit
    let target_operator: OperatorData = persistence
        .get_operator_by_id(request.operator_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get operator: {e}"),
        })?
        .ok_or_else(|| {
            let operator_id = request.operator_id;
            ApiError::ResourceNotFound {
                resource_type: String::from("Operator"),
                message: format!("Operator with ID {operator_id} not found"),
            }
        })?;

    // Enforce invariant: cannot disable the last active admin
    // Only check if the target is an active admin
    if target_operator.role == "Admin" && !target_operator.is_disabled {
        let active_admin_count: i64 =
            persistence
                .count_active_admin_operators()
                .map_err(|e| ApiError::Internal {
                    message: format!("Failed to count active admins: {e}"),
                })?;

        if active_admin_count <= 1 {
            return Err(ApiError::DomainRuleViolation {
                rule: String::from("last_active_admin"),
                message: String::from("Operation would leave the system without an active admin"),
            });
        }
    }

    // Perform the disable operation
    persistence
        .disable_operator(request.operator_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to disable operator: {e}"),
        })?;

    // Create audit event for operator lifecycle change
    let actor: Actor = Actor::with_operator(
        operator.operator_id.to_string(),
        String::from("operator"),
        operator.operator_id,
        operator.login_name.clone(),
        operator.display_name.clone(),
    );

    let action: Action = Action::new(
        String::from("DisableOperator"),
        Some(format!(
            "Disabled operator {} ({})",
            target_operator.login_name, target_operator.display_name
        )),
    );

    let operator_id = request.operator_id;
    let before: StateSnapshot =
        StateSnapshot::new(format!("operator_id={operator_id},is_disabled=false"));
    let after: StateSnapshot =
        StateSnapshot::new(format!("operator_id={operator_id},is_disabled=true"));

    // Phase 23B: Use global event for operator management
    let audit_event: AuditEvent = AuditEvent::new_global(actor, cause, action, before, after);

    // Persist audit event
    persistence
        .persist_audit_event(&audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    let login_name = &target_operator.login_name;
    Ok(DisableOperatorResponse {
        message: format!("Operator {login_name} has been disabled"),
    })
}

/// Re-enables a disabled operator.
///
/// Only Admin actors may re-enable operators.
/// Emits an audit event on success.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The enable operator request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data for audit attribution
/// * `cause` - The cause for this action
///
/// # Returns
///
/// * `Ok(EnableOperatorResponse)` on success
/// * `Err(ApiError)` if unauthorized or operation fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The operator does not exist
/// - Database operations fail
pub fn enable_operator(
    persistence: &mut SqlitePersistence,
    request: EnableOperatorRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<EnableOperatorResponse, ApiError> {
    // Enforce authorization before executing command
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("enable_operator"),
            required_role: String::from("Admin"),
        });
    }

    // Get target operator to verify existence and get details for audit
    let target_operator: OperatorData = persistence
        .get_operator_by_id(request.operator_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get operator: {e}"),
        })?
        .ok_or_else(|| {
            let operator_id = request.operator_id;
            ApiError::ResourceNotFound {
                resource_type: String::from("Operator"),
                message: format!("Operator with ID {operator_id} not found"),
            }
        })?;

    // Perform the enable operation
    persistence
        .enable_operator(request.operator_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to enable operator: {e}"),
        })?;

    // Create audit event for operator lifecycle change
    let actor: Actor = Actor::with_operator(
        operator.operator_id.to_string(),
        String::from("operator"),
        operator.operator_id,
        operator.login_name.clone(),
        operator.display_name.clone(),
    );

    let action: Action = Action::new(
        String::from("EnableOperator"),
        Some(format!(
            "Re-enabled operator {} ({})",
            target_operator.login_name, target_operator.display_name
        )),
    );

    let operator_id = request.operator_id;
    let before: StateSnapshot =
        StateSnapshot::new(format!("operator_id={operator_id},is_disabled=true"));
    let after: StateSnapshot =
        StateSnapshot::new(format!("operator_id={operator_id},is_disabled=false"));

    // Phase 23B: Use global event for operator management
    let audit_event: AuditEvent = AuditEvent::new_global(actor, cause, action, before, after);

    // Persist audit event
    persistence
        .persist_audit_event(&audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    let login_name = &target_operator.login_name;
    Ok(EnableOperatorResponse {
        message: format!("Operator {login_name} has been re-enabled"),
    })
}

/// Deletes an operator.
///
/// Only Admin actors may delete operators.
/// Operators can only be deleted if they are not referenced by any audit events.
/// Emits an audit event on success.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The delete operator request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data for audit attribution
/// * `cause` - The cause for this action
///
/// # Returns
///
/// * `Ok(DeleteOperatorResponse)` on success
/// * `Err(ApiError)` if unauthorized, operator is referenced, or operation fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The operator does not exist
/// - The operator is referenced by audit events
/// - Database operations fail
pub fn delete_operator(
    persistence: &mut SqlitePersistence,
    request: DeleteOperatorRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<DeleteOperatorResponse, ApiError> {
    // Enforce authorization before executing command
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("delete_operator"),
            required_role: String::from("Admin"),
        });
    }

    // Get target operator to verify existence and get details for audit
    let target_operator: OperatorData = persistence
        .get_operator_by_id(request.operator_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get operator: {e}"),
        })?
        .ok_or_else(|| {
            let operator_id = request.operator_id;
            ApiError::ResourceNotFound {
                resource_type: String::from("Operator"),
                message: format!("Operator with ID {operator_id} not found"),
            }
        })?;

    // Enforce invariant: cannot delete the last active admin
    // Only check if the target is an active admin
    if target_operator.role == "Admin" && !target_operator.is_disabled {
        let active_admin_count: i64 =
            persistence
                .count_active_admin_operators()
                .map_err(|e| ApiError::Internal {
                    message: format!("Failed to count active admins: {e}"),
                })?;

        if active_admin_count <= 1 {
            return Err(ApiError::DomainRuleViolation {
                rule: String::from("last_active_admin"),
                message: String::from("Operation would leave the system without an active admin"),
            });
        }
    }

    // Perform the delete operation (will fail if operator is referenced)
    persistence
        .delete_operator(request.operator_id)
        .map_err(|e| match e {
            PersistenceError::OperatorReferenced { operator_id } => ApiError::DomainRuleViolation {
                rule: String::from("operator_not_referenced"),
                message: format!(
                    "Cannot delete operator {operator_id}: referenced by audit events"
                ),
            },
            _ => ApiError::Internal {
                message: format!("Failed to delete operator: {e}"),
            },
        })?;

    // Create audit event for operator lifecycle change
    let actor: Actor = Actor::with_operator(
        operator.operator_id.to_string(),
        String::from("operator"),
        operator.operator_id,
        operator.login_name.clone(),
        operator.display_name.clone(),
    );

    let action: Action = Action::new(
        String::from("DeleteOperator"),
        Some(format!(
            "Deleted operator {} ({})",
            target_operator.login_name, target_operator.display_name
        )),
    );

    let operator_id = request.operator_id;
    let login_name = &target_operator.login_name;
    let before: StateSnapshot =
        StateSnapshot::new(format!("operator_id={operator_id},login_name={login_name}"));
    let after: StateSnapshot = StateSnapshot::new(String::from("operator_deleted"));

    // Phase 23B: Use global event for operator management
    let audit_event: AuditEvent = AuditEvent::new_global(actor, cause, action, before, after);

    // Persist audit event
    persistence
        .persist_audit_event(&audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    let login_name = &target_operator.login_name;
    Ok(DeleteOperatorResponse {
        message: format!("Operator {login_name} has been deleted"),
    })
}

/// Changes an operator's own password.
///
/// Any authenticated operator may change their own password.
/// Validates the current password, enforces password policy, and invalidates all sessions.
/// Emits an audit event on success.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The change password request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data for audit attribution
/// * `cause` - The cause for this action
///
/// # Returns
///
/// * `Ok(ChangePasswordResponse)` on success
/// * `Err(ApiError)` if validation fails or operation fails
///
/// # Errors
///
/// Returns an error if:
/// - Current password is incorrect
/// - New password does not meet policy requirements
/// - Password confirmation does not match
/// - Database operations fail
pub fn change_password(
    persistence: &mut SqlitePersistence,
    request: &ChangePasswordRequest,
    _authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<ChangePasswordResponse, ApiError> {
    // Verify current password
    let password_valid: bool = persistence
        .verify_password(&request.current_password, &operator.password_hash)
        .map_err(|e| ApiError::Internal {
            message: format!("Password verification failed: {e}"),
        })?;

    if !password_valid {
        return Err(ApiError::AuthenticationFailed {
            reason: String::from("Current password is incorrect"),
        });
    }

    // Validate new password policy
    let policy: PasswordPolicy = PasswordPolicy::default();
    policy.validate(
        &request.new_password,
        &request.new_password_confirmation,
        &operator.login_name,
        &operator.display_name,
    )?;

    // Update password
    persistence
        .update_password(operator.operator_id, &request.new_password)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update password: {e}"),
        })?;

    // Invalidate all sessions for this operator
    persistence
        .delete_sessions_for_operator(operator.operator_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to invalidate sessions: {e}"),
        })?;

    // Create audit event for password change
    let actor: Actor = Actor::with_operator(
        operator.operator_id.to_string(),
        String::from("operator"),
        operator.operator_id,
        operator.login_name.clone(),
        operator.display_name.clone(),
    );

    let action: Action = Action::new(
        String::from("ChangePassword"),
        Some(format!(
            "Operator {} changed their own password",
            operator.login_name
        )),
    );

    let operator_id = operator.operator_id;
    let before: StateSnapshot = StateSnapshot::new(format!("operator_id={operator_id}"));
    let after: StateSnapshot =
        StateSnapshot::new(format!("operator_id={operator_id},password_changed"));

    // Phase 23B: Use global event for operator management
    let audit_event: AuditEvent = AuditEvent::new_global(actor, cause, action, before, after);

    // Persist audit event
    persistence
        .persist_audit_event(&audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(ChangePasswordResponse {
        message: String::from("Password changed successfully. All sessions have been invalidated."),
    })
}

/// Resets another operator's password (admin only).
///
/// Only Admin actors may reset other operators' passwords.
/// Does not require the old password, enforces password policy, and invalidates all sessions.
/// Emits an audit event on success.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The reset password request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data for audit attribution (the admin)
/// * `cause` - The cause for this action
///
/// # Returns
///
/// * `Ok(ResetPasswordResponse)` on success
/// * `Err(ApiError)` if unauthorized or operation fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The target operator does not exist
/// - New password does not meet policy requirements
/// - Password confirmation does not match
/// - Database operations fail
pub fn reset_password(
    persistence: &mut SqlitePersistence,
    request: &ResetPasswordRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<ResetPasswordResponse, ApiError> {
    // Enforce authorization before executing command
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("reset_password"),
            required_role: String::from("Admin"),
        });
    }

    // Get target operator to verify existence and get details for validation and audit
    let target_operator: OperatorData = persistence
        .get_operator_by_id(request.operator_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get operator: {e}"),
        })?
        .ok_or_else(|| {
            let operator_id = request.operator_id;
            ApiError::ResourceNotFound {
                resource_type: String::from("Operator"),
                message: format!("Operator with ID {operator_id} not found"),
            }
        })?;

    // Validate new password policy
    let policy: PasswordPolicy = PasswordPolicy::default();
    policy.validate(
        &request.new_password,
        &request.new_password_confirmation,
        &target_operator.login_name,
        &target_operator.display_name,
    )?;

    // Update password
    persistence
        .update_password(request.operator_id, &request.new_password)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update password: {e}"),
        })?;

    // Invalidate all sessions for the target operator
    persistence
        .delete_sessions_for_operator(request.operator_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to invalidate sessions: {e}"),
        })?;

    // Create audit event for password reset
    let actor: Actor = Actor::with_operator(
        operator.operator_id.to_string(),
        String::from("operator"),
        operator.operator_id,
        operator.login_name.clone(),
        operator.display_name.clone(),
    );

    let action: Action = Action::new(
        String::from("ResetPassword"),
        Some(format!(
            "Admin {} reset password for operator {}",
            operator.login_name, target_operator.login_name
        )),
    );

    let operator_id = request.operator_id;
    let target_login = &target_operator.login_name;
    let before: StateSnapshot = StateSnapshot::new(format!(
        "operator_id={operator_id},login_name={target_login}"
    ));
    let after: StateSnapshot = StateSnapshot::new(format!(
        "operator_id={operator_id},login_name={target_login},password_reset"
    ));

    // Phase 23B: Use global event for operator management
    let audit_event: AuditEvent = AuditEvent::new_global(actor, cause, action, before, after);

    // Persist audit event
    persistence
        .persist_audit_event(&audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(ResetPasswordResponse {
        message: format!(
            "Password reset successfully for operator {}. All sessions have been invalidated.",
            target_operator.login_name
        ),
        operator_id: request.operator_id,
    })
}

// ========================================================================
// Bootstrap Authentication (Phase 15)
// ========================================================================

/// Checks whether the system is in bootstrap mode.
///
/// Bootstrap mode is active when no operators exist in the database.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
///
/// # Returns
///
/// * `Ok(BootstrapAuthStatusResponse)` indicating bootstrap status
/// * `Err(ApiError)` if the query fails
///
/// # Errors
///
/// Returns an error if database operations fail.
pub fn check_bootstrap_status(
    persistence: &mut SqlitePersistence,
) -> Result<crate::BootstrapAuthStatusResponse, ApiError> {
    let operator_count: i64 = persistence
        .count_operators()
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to count operators: {e}"),
        })?;

    Ok(crate::BootstrapAuthStatusResponse {
        is_bootstrap_mode: operator_count == 0,
    })
}

/// Performs bootstrap login with hardcoded credentials.
///
/// This function only succeeds when:
/// - No operators exist in the database (bootstrap mode)
/// - Username is exactly "admin"
/// - Password is exactly "admin"
///
/// The returned token is a temporary bootstrap session, not a real operator session.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The bootstrap login request
///
/// # Returns
///
/// * `Ok(BootstrapLoginResponse)` with a bootstrap token
/// * `Err(ApiError)` if bootstrap mode is not active or credentials are invalid
///
/// # Errors
///
/// Returns an error if:
/// - Operators already exist (not in bootstrap mode)
/// - Credentials are not exactly "admin" / "admin"
/// - Database operations fail
///
/// # Panics
///
/// Panics if the system time is before the Unix epoch.
pub fn bootstrap_login(
    persistence: &mut SqlitePersistence,
    request: &crate::BootstrapLoginRequest,
) -> Result<crate::BootstrapLoginResponse, ApiError> {
    // Check if we're in bootstrap mode
    let operator_count: i64 = persistence
        .count_operators()
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to count operators: {e}"),
        })?;

    if operator_count > 0 {
        return Err(ApiError::Unauthorized {
            action: String::from("bootstrap_login"),
            required_role: String::from("Bootstrap mode (no operators exist)"),
        });
    }

    // Verify hardcoded credentials
    if request.username != "admin" || request.password != "admin" {
        return Err(ApiError::from(AuthError::AuthenticationFailed {
            reason: String::from("Invalid bootstrap credentials"),
        }));
    }

    // Generate a bootstrap token (simple, temporary)
    let timestamp: u128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_nanos();
    let bootstrap_token: String = format!("bootstrap_{timestamp}_{}", rand::random::<u64>());

    Ok(crate::BootstrapLoginResponse {
        bootstrap_token,
        is_bootstrap: true,
    })
}

/// Creates the first admin operator during bootstrap.
///
/// This function only succeeds when:
/// - No operators exist in the database (bootstrap mode)
/// - A valid bootstrap token is provided
///
/// After successful creation, the bootstrap session is terminated and
/// the system transitions out of bootstrap mode.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The create first admin request
///
/// # Returns
///
/// * `Ok(CreateFirstAdminResponse)` on success
/// * `Err(ApiError)` if not in bootstrap mode or creation fails
///
/// # Errors
///
/// Returns an error if:
/// - Operators already exist (not in bootstrap mode)
/// - Login name already exists
/// - Password validation fails
/// - Database operations fail
pub fn create_first_admin(
    persistence: &mut SqlitePersistence,
    request: crate::CreateFirstAdminRequest,
) -> Result<crate::CreateFirstAdminResponse, ApiError> {
    // Check if we're in bootstrap mode
    let operator_count: i64 = persistence
        .count_operators()
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to count operators: {e}"),
        })?;

    if operator_count > 0 {
        return Err(ApiError::Unauthorized {
            action: String::from("create_first_admin"),
            required_role: String::from("Bootstrap mode (no operators exist)"),
        });
    }

    // Validate password policy
    let policy: PasswordPolicy = PasswordPolicy::default();
    policy.validate(
        &request.password,
        &request.password_confirmation,
        &request.login_name,
        &request.display_name,
    )?;

    // Create the first admin operator
    let operator_id: i64 = persistence
        .create_operator(
            &request.login_name,
            &request.display_name,
            &request.password,
            "Admin",
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to create first admin: {e}"),
        })?;

    Ok(crate::CreateFirstAdminResponse {
        operator_id,
        login_name: request.login_name,
        display_name: request.display_name,
        message: String::from("First admin operator created successfully"),
    })
}

// ========================================================================
// Phase 18: Bootstrap Workflow Completion Handlers
// ========================================================================

/// Sets the active bid year.
#[allow(dead_code)]
///
/// Only admins can set the active bid year.
/// Exactly one bid year may be active at a time.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `request` - The set active bid year request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
/// * `cause` - The cause or reason for this action
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The bid year does not exist
/// - Database operations fail
pub fn set_active_bid_year(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &SetActiveBidYearRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<SetActiveBidYearResponse, ApiError> {
    // Enforce authorization - only admins can set active bid year
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("set_active_bid_year"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve bid_year_id to BidYear from metadata
    let bid_year: &BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(request.bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {} not found", request.bid_year_id),
        })?;

    let year: u16 = bid_year.year();

    // Apply the command
    let command = Command::SetActiveBidYear { year };
    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    let result: BootstrapResult =
        apply_bootstrap(metadata, bid_year, command, actor, cause).map_err(translate_core_error)?;

    // Persist the active bid year setting
    persistence
        .set_active_bid_year(bid_year)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to set active bid year: {e}"),
        })?;

    // Persist audit event
    persistence
        .persist_audit_event(&result.audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(SetActiveBidYearResponse {
        bid_year_id: request.bid_year_id,
        year,
        message: format!("Bid year {year} is now active"),
    })
}

/// Transitions a bid year from `Draft` to `BootstrapComplete`.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `request` - The transition request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
/// * `cause` - The cause or reason for this action
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The bid year does not exist
/// - Bootstrap is not complete
/// - The transition is invalid
pub fn transition_to_bootstrap_complete(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &TransitionToBootstrapCompleteRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<TransitionToBootstrapCompleteResponse, ApiError> {
    // Enforce authorization - only admins can transition lifecycle states
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("transition_to_bootstrap_complete"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve bid_year_id to BidYear from metadata
    let bid_year: &BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(request.bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {} not found", request.bid_year_id),
        })?;

    let year: u16 = bid_year.year();

    // Load current lifecycle state
    let current_state_str: String = persistence
        .get_lifecycle_state(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get lifecycle state: {e}"),
        })?;

    let current_state: zab_bid_domain::BidYearLifecycle =
        current_state_str.parse().map_err(translate_domain_error)?;

    let target_state = zab_bid_domain::BidYearLifecycle::BootstrapComplete;

    // Validate transition
    if !current_state.can_transition_to(target_state) {
        return Err(translate_domain_error(
            DomainError::InvalidStateTransition {
                current: current_state.as_str().to_string(),
                target: target_state.as_str().to_string(),
            },
        ));
    }

    // Check bootstrap completeness
    let completeness_response: GetBootstrapCompletenessResponse =
        get_bootstrap_completeness(persistence, metadata)?;
    if !completeness_response.is_ready_for_bidding {
        return Err(translate_domain_error(DomainError::BootstrapIncomplete));
    }

    // Phase 25B: Check for users in No Bid area
    let users_in_no_bid: usize = persistence
        .count_users_in_system_area(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to check No Bid area: {e}"),
        })?;

    if users_in_no_bid > 0 {
        let sample_initials: Vec<String> = persistence
            .list_users_in_system_area(request.bid_year_id, 5)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to list users in No Bid area: {e}"),
            })?;

        return Err(translate_domain_error(DomainError::UsersInNoBidArea {
            bid_year: year,
            user_count: users_in_no_bid,
            sample_initials,
        }));
    }

    // Apply the command
    let command = Command::TransitionToBootstrapComplete { year };
    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    let result: BootstrapResult =
        apply_bootstrap(metadata, bid_year, command, actor, cause).map_err(translate_core_error)?;

    // Persist the lifecycle state change
    persistence
        .update_lifecycle_state(request.bid_year_id, target_state.as_str())
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update lifecycle state: {e}"),
        })?;

    // Persist audit event
    persistence
        .persist_audit_event(&result.audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(TransitionToBootstrapCompleteResponse {
        bid_year_id: request.bid_year_id,
        year,
        lifecycle_state: target_state.as_str().to_string(),
        message: format!("Bid year {year} transitioned to {}", target_state.as_str()),
    })
}

/// Transitions a bid year from `BootstrapComplete` to `Canonicalized`.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `request` - The transition request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
/// * `cause` - The cause or reason for this action
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The bid year does not exist
/// - The transition is invalid
pub fn transition_to_canonicalized(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &TransitionToCanonicalizedRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<TransitionToCanonicalizedResponse, ApiError> {
    // Enforce authorization - only admins can transition lifecycle states
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("transition_to_canonicalized"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve bid_year_id to BidYear from metadata
    let bid_year: &BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(request.bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {} not found", request.bid_year_id),
        })?;

    let year: u16 = bid_year.year();

    // Load current lifecycle state
    let current_state_str: String = persistence
        .get_lifecycle_state(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get lifecycle state: {e}"),
        })?;

    let current_state: zab_bid_domain::BidYearLifecycle =
        current_state_str.parse().map_err(translate_domain_error)?;

    let target_state = zab_bid_domain::BidYearLifecycle::Canonicalized;

    // Validate transition
    if !current_state.can_transition_to(target_state) {
        return Err(translate_domain_error(
            DomainError::InvalidStateTransition {
                current: current_state.as_str().to_string(),
                target: target_state.as_str().to_string(),
            },
        ));
    }

    // Check for users in No Bid area (Phase 25B enforcement)
    let users_in_no_bid: usize = persistence
        .count_users_in_system_area(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to check No Bid area: {e}"),
        })?;

    if users_in_no_bid > 0 {
        let sample_initials: Vec<String> = persistence
            .list_users_in_system_area(request.bid_year_id, 5)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to list users in No Bid area: {e}"),
            })?;

        return Err(translate_domain_error(DomainError::UsersInNoBidArea {
            bid_year: year,
            user_count: users_in_no_bid,
            sample_initials,
        }));
    }

    // Apply the command to get the audit event
    let command = Command::TransitionToCanonicalized { year };
    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    let result: BootstrapResult =
        apply_bootstrap(metadata, bid_year, command, actor, cause).map_err(translate_core_error)?;

    // Perform canonicalization (within implicit transaction via persistence layer)
    persistence
        .canonicalize_bid_year(request.bid_year_id, &result.audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to canonicalize bid year: {e}"),
        })?;

    // Update lifecycle state
    persistence
        .update_lifecycle_state(request.bid_year_id, target_state.as_str())
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update lifecycle state: {e}"),
        })?;

    Ok(TransitionToCanonicalizedResponse {
        bid_year_id: request.bid_year_id,
        year,
        lifecycle_state: target_state.as_str().to_string(),
        message: format!("Bid year {year} transitioned to {}", target_state.as_str()),
    })
}

/// Transitions a bid year from `Canonicalized` to `BiddingActive`.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `request` - The transition request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
/// * `cause` - The cause or reason for this action
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The bid year does not exist
/// - Another bid year is already `BiddingActive`
/// - The transition is invalid
pub fn transition_to_bidding_active(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &TransitionToBiddingActiveRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<TransitionToBiddingActiveResponse, ApiError> {
    // Enforce authorization - only admins can transition lifecycle states
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("transition_to_bidding_active"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve bid_year_id to BidYear from metadata
    let bid_year: &BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(request.bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {} not found", request.bid_year_id),
        })?;

    let year: u16 = bid_year.year();

    // Load current lifecycle state
    let current_state_str: String = persistence
        .get_lifecycle_state(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get lifecycle state: {e}"),
        })?;

    let current_state: zab_bid_domain::BidYearLifecycle =
        current_state_str.parse().map_err(translate_domain_error)?;

    let target_state = zab_bid_domain::BidYearLifecycle::BiddingActive;

    // Validate transition
    if !current_state.can_transition_to(target_state) {
        return Err(translate_domain_error(
            DomainError::InvalidStateTransition {
                current: current_state.as_str().to_string(),
                target: target_state.as_str().to_string(),
            },
        ));
    }

    // Check if another bid year is already BiddingActive
    if let Some(active_year) =
        persistence
            .get_bidding_active_year()
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to check for active bid year: {e}"),
            })?
        && active_year != year
    {
        return Err(translate_domain_error(
            DomainError::AnotherBidYearAlreadyActive { active_year },
        ));
    }

    // Apply the command
    let command = Command::TransitionToBiddingActive { year };
    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    let result: BootstrapResult =
        apply_bootstrap(metadata, bid_year, command, actor, cause).map_err(translate_core_error)?;

    // Persist the lifecycle state change
    persistence
        .update_lifecycle_state(request.bid_year_id, target_state.as_str())
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update lifecycle state: {e}"),
        })?;

    // Persist audit event
    persistence
        .persist_audit_event(&result.audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(TransitionToBiddingActiveResponse {
        bid_year_id: request.bid_year_id,
        year,
        lifecycle_state: target_state.as_str().to_string(),
        message: format!("Bid year {year} transitioned to {}", target_state.as_str()),
    })
}

/// Transitions a bid year from `BiddingActive` to `BiddingClosed`.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `request` - The transition request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
/// * `cause` - The cause or reason for this action
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The bid year does not exist
/// - The transition is invalid
pub fn transition_to_bidding_closed(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &TransitionToBiddingClosedRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<TransitionToBiddingClosedResponse, ApiError> {
    // Enforce authorization - only admins can transition lifecycle states
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("transition_to_bidding_closed"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve bid_year_id to BidYear from metadata
    let bid_year: &BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(request.bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {} not found", request.bid_year_id),
        })?;

    let year: u16 = bid_year.year();

    // Load current lifecycle state
    let current_state_str: String = persistence
        .get_lifecycle_state(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get lifecycle state: {e}"),
        })?;

    let current_state: zab_bid_domain::BidYearLifecycle =
        current_state_str.parse().map_err(translate_domain_error)?;

    let target_state = zab_bid_domain::BidYearLifecycle::BiddingClosed;

    // Validate transition
    if !current_state.can_transition_to(target_state) {
        return Err(translate_domain_error(
            DomainError::InvalidStateTransition {
                current: current_state.as_str().to_string(),
                target: target_state.as_str().to_string(),
            },
        ));
    }

    // Apply the command
    let command = Command::TransitionToBiddingClosed { year };
    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    let result: BootstrapResult =
        apply_bootstrap(metadata, bid_year, command, actor, cause).map_err(translate_core_error)?;

    // Persist the lifecycle state change
    persistence
        .update_lifecycle_state(request.bid_year_id, target_state.as_str())
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update lifecycle state: {e}"),
        })?;

    // Persist audit event
    persistence
        .persist_audit_event(&result.audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(TransitionToBiddingClosedResponse {
        bid_year_id: request.bid_year_id,
        year,
        lifecycle_state: target_state.as_str().to_string(),
        message: format!("Bid year {year} transitioned to {}", target_state.as_str()),
    })
}

/// Updates the metadata (label and notes) for a bid year.
///
/// This is an admin-only operation that can be performed in any lifecycle state.
/// Metadata changes are audited.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `request` - The update metadata request
/// * `authenticated_actor` - The authenticated actor
/// * `operator` - The operator data
/// * `cause` - The cause of the action
///
/// # Returns
///
/// * `Ok(UpdateBidYearMetadataResponse)` if successful
/// * `Err(ApiError)` if unauthorized, validation fails, or persistence fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an admin
/// - The bid year does not exist
/// - Label exceeds 100 characters
/// - Notes exceed 2000 characters
/// - Database operations fail
pub fn update_bid_year_metadata(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &UpdateBidYearMetadataRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<UpdateBidYearMetadataResponse, ApiError> {
    // Enforce authorization - only admins can update bid year metadata
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("update bid year metadata"),
            required_role: String::from("Admin"),
        });
    }

    // Validate label length
    if let Some(ref label) = request.label
        && label.len() > 100
    {
        return Err(ApiError::InvalidInput {
            field: String::from("label"),
            message: String::from("Label must not exceed 100 characters"),
        });
    }

    // Validate notes length
    if let Some(ref notes) = request.notes
        && notes.len() > 2000
    {
        return Err(ApiError::InvalidInput {
            field: String::from("notes"),
            message: String::from("Notes must not exceed 2000 characters"),
        });
    }

    // Retrieve the bid year to get the year value
    let bid_year: &zab_bid_domain::BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(request.bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {} not found", request.bid_year_id),
        })?;

    let year: u16 = bid_year.year();

    // Retrieve current metadata for audit before/after
    let (old_label, old_notes) = persistence
        .get_bid_year_metadata(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to retrieve current metadata: {e}"),
        })?;

    // Update the metadata in the database
    persistence
        .update_bid_year_metadata(
            request.bid_year_id,
            request.label.as_deref(),
            request.notes.as_deref(),
        )
        .map_err(|e| match e {
            PersistenceError::NotFound(_) => ApiError::ResourceNotFound {
                resource_type: String::from("BidYear"),
                message: format!("Bid year with ID {} not found", request.bid_year_id),
            },
            _ => ApiError::Internal {
                message: format!("Failed to update bid year metadata: {e}"),
            },
        })?;

    // Create audit event
    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    let action: Action = Action {
        name: String::from("UpdateBidYearMetadata"),
        details: Some(format!(
            "Updated metadata for bid year {}: label: {:?} -> {:?}, notes: {:?} -> {:?}",
            year, old_label, request.label, old_notes, request.notes
        )),
    };

    let before_snapshot: String = format!(
        r#"{{"label":{},"notes":{}}}"#,
        old_label.as_ref().map_or_else(
            || "null".to_string(),
            |s| format!("\"{}\"", s.replace('"', "\\\""))
        ),
        old_notes.as_ref().map_or_else(
            || "null".to_string(),
            |s| format!("\"{}\"", s.replace('"', "\\\""))
        )
    );

    let after_snapshot: String = format!(
        r#"{{"label":{},"notes":{}}}"#,
        request.label.as_ref().map_or_else(
            || "null".to_string(),
            |s| format!("\"{}\"", s.replace('"', "\\\""))
        ),
        request.notes.as_ref().map_or_else(
            || "null".to_string(),
            |s| format!("\"{}\"", s.replace('"', "\\\""))
        )
    );

    let before: StateSnapshot = StateSnapshot::new(before_snapshot);
    let after: StateSnapshot = StateSnapshot::new(after_snapshot);

    let audit_event: AuditEvent = AuditEvent::new_global(actor, cause, action, before, after);

    // Persist audit event
    persistence
        .persist_audit_event(&audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(UpdateBidYearMetadataResponse {
        bid_year_id: request.bid_year_id,
        year,
        label: request.label.clone(),
        notes: request.notes.clone(),
        message: format!("Metadata updated for bid year {year}"),
    })
}

/// Sets the bid schedule for a bid year.
///
/// Phase 29C: Configures when and how bidding occurs.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `request` - The set bid schedule request
/// * `authenticated_actor` - The authenticated operator
/// * `operator` - The operator data
/// * `cause` - The cause of this action
///
/// # Returns
///
/// * `Ok(SetBidScheduleResponse)` if the bid schedule was set successfully
/// * `Err(ApiError)` if validation fails or the bid year is locked
///
/// # Errors
///
/// Returns an error if:
/// - The operator is not an admin
/// - The bid year is in a locked lifecycle state
/// - Validation of the bid schedule fails
/// - Database operations fail
#[allow(dead_code, clippy::too_many_lines)]
pub fn set_bid_schedule(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &SetBidScheduleRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<SetBidScheduleResponse, ApiError> {
    const TIME_FORMAT: &[time::format_description::FormatItem<'_>] =
        time::macros::format_description!("[hour]:[minute]:[second]");

    // Enforce authorization - only admins can set bid schedule
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("set bid schedule"),
            required_role: String::from("Admin"),
        });
    }

    // Retrieve the bid year
    let bid_year: &zab_bid_domain::BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(request.bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {} not found", request.bid_year_id),
        })?;

    let year: u16 = bid_year.year();

    // Check lifecycle state - bid schedule is only editable in Draft and BootstrapComplete
    let lifecycle_state: BidYearLifecycle = persistence
        .get_lifecycle_state(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get lifecycle state: {e}"),
        })
        .and_then(|s| {
            s.parse::<BidYearLifecycle>()
                .map_err(translate_domain_error)
        })?;

    if lifecycle_state.is_locked() {
        return Err(ApiError::InvalidInput {
            field: String::from("lifecycle_state"),
            message: format!("Cannot modify bid schedule: bid year is in {lifecycle_state} state"),
        });
    }

    // Parse and validate the bid schedule fields
    let start_date: time::Date = time::Date::parse(
        &request.start_date,
        &time::format_description::well_known::Iso8601::DEFAULT,
    )
    .map_err(|_| ApiError::InvalidInput {
        field: String::from("start_date"),
        message: format!("Invalid date format: {}", request.start_date),
    })?;

    let window_start_time: time::Time = time::Time::parse(&request.window_start_time, TIME_FORMAT)
        .map_err(|_| ApiError::InvalidInput {
            field: String::from("window_start_time"),
            message: format!("Invalid time format: {}", request.window_start_time),
        })?;

    let window_end_time: time::Time = time::Time::parse(&request.window_end_time, TIME_FORMAT)
        .map_err(|_| ApiError::InvalidInput {
            field: String::from("window_end_time"),
            message: format!("Invalid time format: {}", request.window_end_time),
        })?;

    // Create and validate BidSchedule domain object
    let _bid_schedule: BidSchedule = BidSchedule::new(
        request.timezone.clone(),
        start_date,
        window_start_time,
        window_end_time,
        request.bidders_per_day,
    )
    .map_err(translate_domain_error)?;

    // Retrieve old bid schedule for audit
    let old_schedule = persistence.get_bid_schedule(request.bid_year_id).ok();

    // Update the bid schedule in the database
    persistence
        .update_bid_schedule(
            request.bid_year_id,
            Some(&request.timezone),
            Some(&request.start_date),
            Some(&request.window_start_time),
            Some(&request.window_end_time),
            Some(request.bidders_per_day.cast_signed()),
        )
        .map_err(|e| match e {
            PersistenceError::NotFound(_) => ApiError::ResourceNotFound {
                resource_type: String::from("BidYear"),
                message: format!("Bid year with ID {} not found", request.bid_year_id),
            },
            _ => ApiError::Internal {
                message: format!("Failed to update bid schedule: {e}"),
            },
        })?;

    // Create audit event
    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    let action: Action = Action {
        name: String::from("SetBidSchedule"),
        details: Some(format!(
            "Set bid schedule for bid year {year}: timezone={}, start_date={}, window={}–{}, bidders_per_day={}",
            request.timezone,
            request.start_date,
            request.window_start_time,
            request.window_end_time,
            request.bidders_per_day
        )),
    };

    let before_snapshot: String = if let Some((tz, sd, wst, wet, bpd)) = old_schedule {
        format!(
            r#"{{"timezone":{},"start_date":{},"window_start_time":{},"window_end_time":{},"bidders_per_day":{}}}"#,
            tz.as_ref()
                .map_or_else(|| "null".to_string(), |s| format!("\"{s}\"")),
            sd.as_ref()
                .map_or_else(|| "null".to_string(), |s| format!("\"{s}\"")),
            wst.as_ref()
                .map_or_else(|| "null".to_string(), |s| format!("\"{s}\"")),
            wet.as_ref()
                .map_or_else(|| "null".to_string(), |s| format!("\"{s}\"")),
            bpd.map_or_else(|| "null".to_string(), |v| v.to_string())
        )
    } else {
        String::from("null")
    };

    let after_snapshot: String = format!(
        r#"{{"timezone":"{}","start_date":"{}","window_start_time":"{}","window_end_time":"{}","bidders_per_day":{}}}"#,
        request.timezone,
        request.start_date,
        request.window_start_time,
        request.window_end_time,
        request.bidders_per_day
    );

    let before: StateSnapshot = StateSnapshot::new(before_snapshot);
    let after: StateSnapshot = StateSnapshot::new(after_snapshot);

    let audit_event: AuditEvent = AuditEvent::new_global(actor, cause, action, before, after);

    // Persist audit event
    persistence
        .persist_audit_event(&audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(SetBidScheduleResponse {
        bid_year_id: request.bid_year_id,
        year,
        bid_schedule: BidScheduleInfo {
            timezone: request.timezone.clone(),
            start_date: request.start_date.clone(),
            window_start_time: request.window_start_time.clone(),
            window_end_time: request.window_end_time.clone(),
            bidders_per_day: request.bidders_per_day,
        },
        message: format!("Bid schedule set for bid year {year}"),
    })
}

/// Gets the bid schedule for a bid year.
///
/// Phase 29C: Returns the configured bid schedule or None if not set.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// * `Ok(GetBidScheduleResponse)` containing the bid schedule (if configured)
/// * `Err(ApiError)` if the bid year doesn't exist
///
/// # Errors
///
/// Returns an error if the bid year is not found.
pub fn get_bid_schedule(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    bid_year_id: i64,
) -> Result<GetBidScheduleResponse, ApiError> {
    // Retrieve the bid year
    let bid_year: &zab_bid_domain::BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {bid_year_id} not found"),
        })?;

    let year: u16 = bid_year.year();

    // Fetch bid schedule from persistence
    let bid_schedule = persistence
        .get_bid_schedule(bid_year_id)
        .map_err(|e| match e {
            PersistenceError::NotFound(_) => ApiError::ResourceNotFound {
                resource_type: String::from("BidYear"),
                message: format!("Bid year with ID {bid_year_id} not found"),
            },
            _ => ApiError::Internal {
                message: format!("Failed to get bid schedule: {e}"),
            },
        })
        .ok()
        .and_then(|(tz, sd, wst, wet, bpd)| {
            // Only construct BidScheduleInfo if all fields are present
            if let (
                Some(timezone),
                Some(start_date),
                Some(window_start_time),
                Some(window_end_time),
                Some(bidders_per_day),
            ) = (tz, sd, wst, wet, bpd)
            {
                Some(BidScheduleInfo {
                    timezone,
                    start_date,
                    window_start_time,
                    window_end_time,
                    bidders_per_day: bidders_per_day.cast_unsigned(),
                })
            } else {
                None
            }
        });

    Ok(GetBidScheduleResponse {
        bid_year_id,
        year,
        bid_schedule,
    })
}

/// Gets the currently active bid year.
#[allow(dead_code)]
///
/// # Arguments
///
/// * `persistence` - The persistence layer
///
/// # Errors
///
/// Returns an error if database operations fail.
pub fn get_active_bid_year(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
) -> Result<GetActiveBidYearResponse, ApiError> {
    let year: u16 = persistence
        .get_active_bid_year()
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get active bid year: {e}"),
        })?;

    // Extract bid_year_id if there is an active year
    let bid_year_id: Option<i64> = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == year)
        .and_then(zab_bid_domain::BidYear::bid_year_id);

    Ok(GetActiveBidYearResponse {
        bid_year_id,
        year: Some(year),
    })
}

/// Sets the expected area count for a bid year.
#[allow(dead_code)]
///
/// Only admins can set expected area counts.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `request` - The set expected area count request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
/// * `cause` - The cause or reason for this action
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The bid year does not exist
/// - The expected count is zero
/// - Database operations fail
pub fn set_expected_area_count(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &SetExpectedAreaCountRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<SetExpectedAreaCountResponse, ApiError> {
    // Enforce authorization - only admins can set expected counts
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("set_expected_area_count"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve the active bid year from canonical state
    let active_bid_year: BidYear = resolve_active_bid_year(persistence)?;

    let command = Command::SetExpectedAreaCount {
        expected_count: request.expected_count,
    };

    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    let result: BootstrapResult =
        apply_bootstrap(metadata, &active_bid_year, command, actor, cause)
            .map_err(translate_core_error)?;

    // Persist the expected area count
    persistence
        .set_expected_area_count(&active_bid_year, request.expected_count as usize)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to set expected area count: {e}"),
        })?;

    // Persist audit event
    persistence
        .persist_audit_event(&result.audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    // Extract bid_year_id from metadata
    let bid_year_id: i64 = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == active_bid_year.year())
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| ApiError::Internal {
            message: format!(
                "Bid year {} exists but has no ID in metadata",
                active_bid_year.year()
            ),
        })?;

    Ok(SetExpectedAreaCountResponse {
        bid_year_id,
        bid_year: active_bid_year.year(),
        expected_count: request.expected_count,
        message: format!(
            "Expected area count set to {} for bid year {}",
            request.expected_count,
            active_bid_year.year()
        ),
    })
}

/// Sets the expected user count for an area.
#[allow(dead_code)]
///
/// Only admins can set expected user counts.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `request` - The set expected user count request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
/// * `cause` - The cause or reason for this action
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The bid year or area does not exist
/// - The expected count is zero
/// - Database operations fail
pub fn set_expected_user_count(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &SetExpectedUserCountRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<SetExpectedUserCountResponse, ApiError> {
    // Enforce authorization - only admins can set expected counts
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("set_expected_user_count"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve the active bid year from canonical state
    let active_bid_year: BidYear = resolve_active_bid_year(persistence)?;

    // Resolve area_id to Area from metadata
    let area: &Area = metadata
        .areas
        .iter()
        .filter(|(by, _)| by.year() == active_bid_year.year())
        .find(|(_, a)| a.area_id() == Some(request.area_id))
        .map(|(_, a)| a)
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("Area"),
            message: format!(
                "Area with ID {} not found in active bid year",
                request.area_id
            ),
        })?;

    // Check if this is a system area (No Bid should not have expected count)
    let is_system =
        persistence
            .is_system_area(request.area_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to check system area status: {e}"),
            })?;

    if is_system {
        return Err(ApiError::InvalidInput {
            field: String::from("area_id"),
            message: format!(
                "Cannot set expected user count for system area '{}'",
                area.area_code()
            ),
        });
    }

    // Get bid_year_id for lifecycle check
    let bid_year_id = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == active_bid_year.year())
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| ApiError::Internal {
            message: format!("Bid year {} has no ID", active_bid_year.year()),
        })?;

    // Check lifecycle state - reject if >= Canonicalized
    let lifecycle_state_str =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    let lifecycle_state = zab_bid_domain::BidYearLifecycle::from_str(&lifecycle_state_str)
        .map_err(|_| ApiError::Internal {
            message: format!("Invalid lifecycle state: {lifecycle_state_str}"),
        })?;

    // Expected user count can only be set before canonicalization
    if matches!(
        lifecycle_state,
        zab_bid_domain::BidYearLifecycle::Canonicalized
            | zab_bid_domain::BidYearLifecycle::BiddingActive
            | zab_bid_domain::BidYearLifecycle::BiddingClosed
    ) {
        return Err(translate_domain_error(
            DomainError::CannotEditAreaAfterCanonicalization {
                bid_year: active_bid_year.year(),
                lifecycle_state: lifecycle_state_str,
            },
        ));
    }

    let command = Command::SetExpectedUserCount {
        area: area.clone(),
        expected_count: request.expected_count,
    };

    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    let result: BootstrapResult =
        apply_bootstrap(metadata, &active_bid_year, command, actor, cause)
            .map_err(translate_core_error)?;

    // Persist the expected user count
    persistence
        .set_expected_user_count(&active_bid_year, area, request.expected_count as usize)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to set expected user count: {e}"),
        })?;

    // Persist audit event
    persistence
        .persist_audit_event(&result.audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(SetExpectedUserCountResponse {
        bid_year_id,
        bid_year: active_bid_year.year(),
        area_id: request.area_id,
        area_code: area.area_code().to_string(),
        expected_count: request.expected_count,
        message: format!(
            "Expected user count set to {} for area '{}' in bid year {}",
            request.expected_count,
            area.area_code(),
            active_bid_year.year()
        ),
    })
}

/// Updates an existing user's information.
#[allow(dead_code)]
///
/// Only admins can update users.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
/// * `state` - The current system state
/// * `request` - The update user request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
/// * `cause` - The cause or reason for this action
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The user does not exist
/// - Validation fails
/// - Database operations fail
pub fn update_user(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    state: &State,
    request: &UpdateUserRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<ApiResult<UpdateUserResponse>, ApiError> {
    // Enforce authorization - only admins can update users
    AuthorizationService::authorize_register_user(authenticated_actor)?;

    // Resolve the active bid year from canonical state
    let active_bid_year: BidYear = resolve_active_bid_year(persistence)?;

    // Resolve area_id to Area from metadata
    let area: &Area = metadata
        .areas
        .iter()
        .filter(|(by, _)| by.year() == active_bid_year.year())
        .find(|(_, a)| a.area_id() == Some(request.area_id))
        .map(|(_, a)| a)
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("Area"),
            message: format!(
                "Area with ID {} not found in active bid year",
                request.area_id
            ),
        })?;

    // Translate API request into domain types
    let initials: Initials = Initials::new(&request.initials);
    let user_type: UserType =
        UserType::parse(&request.user_type).map_err(translate_domain_error)?;
    let crew: Option<Crew> = match request.crew {
        Some(crew_num) => Some(Crew::new(crew_num).map_err(translate_domain_error)?),
        None => None,
    };
    let seniority_data: SeniorityData = SeniorityData::new(
        request.cumulative_natca_bu_date.clone(),
        request.natca_bu_date.clone(),
        request.eod_faa_date.clone(),
        request.service_computation_date.clone(),
        request.lottery_value,
    );

    // Create command
    let command = Command::UpdateUser {
        user_id: request.user_id,
        initials: initials.clone(),
        name: request.name.clone(),
        area: area.clone(),
        user_type,
        crew,
        seniority_data,
    };

    // Convert authenticated actor to audit actor
    let actor: Actor = authenticated_actor.to_audit_actor(operator);

    // Apply the command
    let result: TransitionResult = apply(metadata, state, &active_bid_year, command, actor, cause)
        .map_err(translate_core_error)?;

    // Persist the updated canonical user state using user_id from request
    persistence
        .update_user(
            request.user_id,
            &initials,
            &request.name,
            area,
            &request.user_type,
            request.crew,
            &request.cumulative_natca_bu_date,
            &request.natca_bu_date,
            &request.eod_faa_date,
            &request.service_computation_date,
            request.lottery_value,
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update user: {e}"),
        })?;

    // Persist audit event
    persistence
        .persist_audit_event(&result.audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    // Extract bid_year_id from metadata
    let bid_year_id: i64 = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == active_bid_year.year())
        .and_then(zab_bid_domain::BidYear::bid_year_id)
        .ok_or_else(|| ApiError::Internal {
            message: format!(
                "Bid year {} exists but has no ID in metadata",
                active_bid_year.year()
            ),
        })?;

    // Build response
    let response = UpdateUserResponse {
        bid_year_id,
        bid_year: active_bid_year.year(),
        user_id: request.user_id,
        initials: request.initials.clone(),
        name: request.name.clone(),
        message: String::from("User updated successfully"),
    };

    Ok(ApiResult {
        response,
        audit_event: result.audit_event,
        new_state: result.new_state,
    })
}

/// Gets the bootstrap completeness status for all bid years and areas.
#[allow(dead_code)]
///
/// This function computes whether each bid year and area meets its
/// expected counts and returns detailed blocking reasons.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The current bootstrap metadata
///
/// # Errors
///
/// Returns an error if database operations fail.
#[allow(clippy::too_many_lines)]
pub fn get_bootstrap_completeness(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
) -> Result<GetBootstrapCompletenessResponse, ApiError> {
    let active_bid_year: Option<u16> = persistence.get_active_bid_year().ok();

    // Extract active_bid_year_id if there is an active year
    let active_bid_year_id: Option<i64> = active_bid_year.and_then(|y| {
        metadata
            .bid_years
            .iter()
            .find(|by| by.year() == y)
            .and_then(zab_bid_domain::BidYear::bid_year_id)
    });

    let mut bid_years_info: Vec<BidYearCompletenessInfo> = Vec::new();
    let mut areas_info: Vec<AreaCompletenessInfo> = Vec::new();
    let mut top_level_blocking: Vec<BlockingReason> = Vec::new();

    // If no active bid year, that's a top-level blocker
    if active_bid_year.is_none() {
        top_level_blocking.push(BlockingReason::NoActiveBidYear);
    }

    // Phase 25E: Check for users in No Bid area across all bid years
    for bid_year in &metadata.bid_years {
        let year: u16 = bid_year.year();
        let bid_year_id: i64 = match bid_year.bid_year_id() {
            Some(id) => id,
            None => continue, // Skip bid years without IDs
        };

        let users_in_no_bid: usize = persistence
            .count_users_in_system_area(bid_year_id)
            .unwrap_or(0);

        if users_in_no_bid > 0 {
            let sample_initials: Vec<String> = persistence
                .list_users_in_system_area(bid_year_id, 5)
                .unwrap_or_default();

            top_level_blocking.push(BlockingReason::UsersInNoBidArea {
                bid_year_id,
                bid_year: year,
                user_count: users_in_no_bid,
                sample_initials,
            });
        }
    }

    // Check each bid year
    for bid_year in &metadata.bid_years {
        let year: u16 = bid_year.year();
        let bid_year_id: i64 = bid_year.bid_year_id().ok_or_else(|| ApiError::Internal {
            message: format!("Bid year {year} has no ID in metadata"),
        })?;
        let is_active: bool = active_bid_year == Some(year);

        let expected_area_count: Option<u32> = persistence
            .get_expected_area_count(&BidYear::new(year))
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get expected area count: {e}"),
            })?
            .map(|v| {
                u32::try_from(v).unwrap_or_else(|_| {
                    tracing::warn!("Expected area count out of range: {}", v);
                    u32::MAX
                })
            });

        let actual_area_count: usize = persistence
            .get_actual_area_count(&BidYear::new(year))
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get actual area count: {e}"),
            })?;

        let mut blocking_reasons: Vec<BlockingReason> = Vec::new();

        // Check if expected count is set
        let expected_count = expected_area_count.unwrap_or_else(|| {
            blocking_reasons.push(BlockingReason::ExpectedAreaCountNotSet {
                bid_year_id,
                bid_year: year,
            });
            0 // Placeholder
        });

        // Check if actual matches expected
        if expected_area_count.is_some() && actual_area_count != expected_count as usize {
            blocking_reasons.push(BlockingReason::AreaCountMismatch {
                bid_year_id,
                bid_year: year,
                expected: expected_count,
                actual: actual_area_count,
            });
        }

        let is_complete: bool = blocking_reasons.is_empty() && expected_area_count.is_some();

        // Fetch lifecycle state
        let lifecycle_state: String =
            persistence
                .get_lifecycle_state(bid_year_id)
                .map_err(|e| ApiError::Internal {
                    message: format!("Failed to get lifecycle state: {e}"),
                })?;

        bid_years_info.push(BidYearCompletenessInfo {
            bid_year_id,
            year,
            is_active,
            expected_area_count,
            actual_area_count,
            is_complete,
            blocking_reasons,
            lifecycle_state,
        });
    }

    // Check each area
    for (bid_year, area) in &metadata.areas {
        let year: u16 = bid_year.year();
        let bid_year_id: i64 = metadata
            .bid_years
            .iter()
            .find(|by| by.year() == year)
            .and_then(zab_bid_domain::BidYear::bid_year_id)
            .ok_or_else(|| ApiError::Internal {
                message: format!("Bid year {year} has no ID in metadata"),
            })?;
        let area_code: String = area.area_code().to_string();
        let area_id: i64 = area.area_id().ok_or_else(|| ApiError::Internal {
            message: format!("Area '{area_code}' in bid year {year} has no ID in metadata"),
        })?;

        let expected_user_count: Option<u32> = persistence
            .get_expected_user_count(bid_year, area)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get expected user count: {e}"),
            })?
            .map(|v| {
                u32::try_from(v).unwrap_or_else(|_| {
                    tracing::warn!("Expected user count out of range: {}", v);
                    u32::MAX
                })
            });

        let actual_user_count: usize =
            persistence
                .get_actual_user_count(bid_year, area)
                .map_err(|e| ApiError::Internal {
                    message: format!("Failed to get actual user count: {e}"),
                })?;

        let mut blocking_reasons: Vec<BlockingReason> = Vec::new();
        let is_system_area: bool = area.is_system_area();

        // System areas do not require expected user count validation or round group assignment
        if !is_system_area {
            // Check if area has a round group assigned
            let round_group_id =
                persistence
                    .get_area_round_group_id(area_id)
                    .map_err(|e| ApiError::Internal {
                        message: format!("Failed to get round group for area: {e}"),
                    })?;

            if round_group_id.is_none() {
                blocking_reasons.push(BlockingReason::AreaMissingRoundGroup {
                    bid_year_id,
                    bid_year: year,
                    area_id,
                    area_code: area_code.clone(),
                });
            }

            // Check if expected count is set
            let expected_count = expected_user_count.unwrap_or_else(|| {
                blocking_reasons.push(BlockingReason::ExpectedUserCountNotSet {
                    bid_year_id,
                    bid_year: year,
                    area_id,
                    area_code: area_code.clone(),
                });
                0 // Placeholder
            });

            // Check if actual matches expected
            if expected_user_count.is_some() && actual_user_count != expected_count as usize {
                blocking_reasons.push(BlockingReason::UserCountMismatch {
                    bid_year_id,
                    bid_year: year,
                    area_id,
                    area_code: area_code.clone(),
                    expected: expected_count,
                    actual: actual_user_count,
                });
            }
        }

        let is_complete: bool = if is_system_area {
            true // System areas are always complete
        } else {
            blocking_reasons.is_empty() && expected_user_count.is_some()
        };

        areas_info.push(AreaCompletenessInfo {
            bid_year_id,
            bid_year: year,
            area_id,
            area_code,
            is_system_area,
            expected_user_count,
            actual_user_count,
            is_complete,
            blocking_reasons,
        });
    }

    // Check round groups have at least one round
    if let Some(active_bid_year_id) = active_bid_year_id {
        let round_groups = persistence
            .list_round_groups(active_bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to list round groups: {e}"),
            })?;

        for rg in round_groups {
            let rg_id = rg.round_group_id().ok_or_else(|| ApiError::Internal {
                message: String::from("Round group has no ID"),
            })?;

            let rounds = persistence
                .list_rounds(rg_id)
                .map_err(|e| ApiError::Internal {
                    message: format!("Failed to list rounds for round group: {e}"),
                })?;

            if rounds.is_empty() {
                top_level_blocking.push(BlockingReason::RoundGroupHasNoRounds {
                    bid_year_id: active_bid_year_id,
                    bid_year: active_bid_year.unwrap_or(0),
                    round_group_id: rg_id,
                    round_group_name: rg.name().to_string(),
                });
            }
        }
    }

    // Determine if system is ready for bidding
    // System is ready only when there are NO blocking reasons at any level
    let is_ready_for_bidding: bool = top_level_blocking.is_empty()
        && bid_years_info.iter().all(|b| b.blocking_reasons.is_empty())
        && areas_info.iter().all(|a| a.blocking_reasons.is_empty());

    Ok(GetBootstrapCompletenessResponse {
        active_bid_year_id,
        active_bid_year,
        bid_years: bid_years_info,
        areas: areas_info,
        is_ready_for_bidding,
        blocking_reasons: top_level_blocking,
    })
}

/// Previews and validates CSV user data without persisting.
///
/// This handler:
/// - Accepts CSV content and a bid year
/// - Parses and validates each row
/// - Returns structured preview results
/// - Does NOT mutate state or emit audit events
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `persistence` - The persistence layer for querying existing users
/// * `request` - The preview request containing bid year and CSV content
/// * `authenticated_actor` - The authenticated actor making the request
///
/// # Returns
///
/// * `Ok(PreviewCsvUsersResponse)` with per-row validation results
/// * `Err(ApiError)` if unauthorized or CSV format is invalid
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an admin
/// - The bid year does not exist
/// - The CSV format is invalid
pub fn preview_csv_users(
    metadata: &BootstrapMetadata,
    persistence: &mut SqlitePersistence,
    request: &PreviewCsvUsersRequest,
    authenticated_actor: &AuthenticatedActor,
) -> Result<PreviewCsvUsersResponse, ApiError> {
    // Enforce authorization - only admins can preview CSV imports
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("preview_csv_users"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve the active bid year from canonical state
    let active_bid_year: BidYear = resolve_active_bid_year(persistence)?;

    // Validate bid year exists
    validate_bid_year_exists(metadata, &active_bid_year).map_err(translate_domain_error)?;

    // Perform CSV preview validation
    let preview_result = preview_csv_users_impl(
        &request.csv_content,
        &active_bid_year,
        metadata,
        persistence,
    )?;

    // Convert internal result to API response
    let rows: Vec<CsvRowPreview> = preview_result
        .rows
        .into_iter()
        .map(|r: CsvRowResult| CsvRowPreview {
            row_number: r.row_number,
            initials: r.initials,
            name: r.name,
            area_id: r.area_id,
            user_type: r.user_type,
            crew: r.crew,
            status: match r.status {
                crate::csv_preview::CsvRowStatus::Valid => CsvRowStatus::Valid,
                crate::csv_preview::CsvRowStatus::Invalid => CsvRowStatus::Invalid,
            },
            errors: r.errors,
        })
        .collect();

    Ok(PreviewCsvUsersResponse {
        bid_year: active_bid_year.year(),
        rows,
        total_rows: preview_result.total_rows,
        valid_count: preview_result.valid_count,
        invalid_count: preview_result.invalid_count,
    })
}

/// Imports selected CSV rows as users.
///
/// This function:
/// - Verifies the actor is authorized (Admin role required)
/// - Re-parses each selected CSV row
/// - Attempts to create each user individually
/// - Returns per-row success/failure results
/// - Does NOT roll back on failure
///
/// # Arguments
///
/// * `metadata` - The current bootstrap metadata
/// * `state` - The current system state
/// * `persistence` - The persistence layer
/// * `request` - The API request containing CSV content and selected row indices
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data for audit trail
/// * `cause` - The cause or reason for this action
///
/// # Returns
///
/// * `Ok((ImportCsvUsersResponse, Vec<AuditEvent>, State))` on completion
/// * `Err(ApiError)` if unauthorized or CSV parsing fails
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not authorized (not an Admin)
/// - The CSV cannot be parsed
/// - The bid year does not exist
///
/// Individual row failures are captured in the response, not as errors.
#[allow(clippy::too_many_lines)]
pub fn import_csv_users(
    metadata: &BootstrapMetadata,
    _state: &State,
    persistence: &mut SqlitePersistence,
    request: &ImportCsvUsersRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: &Cause,
) -> Result<ImportCsvUsersResponse, ApiError> {
    // Enforce authorization - only admins can import users
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("import_csv_users"),
            required_role: String::from("Admin"),
        });
    }

    // Resolve the active bid year from canonical state
    let active_bid_year: BidYear = resolve_active_bid_year(persistence)?;

    // Validate bid year exists
    validate_bid_year_exists(metadata, &active_bid_year).map_err(translate_domain_error)?;

    // Convert authenticated actor to audit actor
    let actor: Actor = authenticated_actor.to_audit_actor(operator);

    // Parse CSV and collect all rows first
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(request.csv_content.as_bytes());

    let headers = reader
        .headers()
        .map_err(|e| ApiError::InvalidCsvFormat {
            reason: format!("Failed to read CSV headers: {e}"),
        })?
        .clone();

    // Build header map for field extraction
    let mut header_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (idx, header) in headers.iter().enumerate() {
        let normalized = header.trim().to_lowercase().replace(' ', "_");
        header_map.insert(normalized, idx);
    }

    // Collect all records into a vec so we can index into them
    let all_records: Vec<csv::StringRecord> = reader
        .records()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ApiError::InvalidCsvFormat {
            reason: format!("Failed to read CSV records: {e}"),
        })?;

    let total_selected: usize = request.selected_row_indices.len();
    let mut successful_count: usize = 0;
    let mut failed_count: usize = 0;
    let mut results: Vec<CsvImportRowResult> = Vec::new();

    // Process each selected row
    for &row_index in &request.selected_row_indices {
        let row_number: usize = row_index + 1;

        // Check if row index is valid
        if row_index >= all_records.len() {
            results.push(CsvImportRowResult {
                row_index,
                row_number,
                initials: None,
                status: CsvImportRowStatus::Failed,
                error: Some(String::from("Row index out of bounds")),
            });
            failed_count += 1;
            continue;
        }

        let record = &all_records[row_index];

        // Extract fields using header map
        let get_field = |name: &str| -> Option<String> {
            header_map
                .get(name)
                .and_then(|&idx| record.get(idx))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        // Extract required fields
        let Some(initials_str) = get_field("initials") else {
            results.push(CsvImportRowResult {
                row_index,
                row_number,
                initials: None,
                status: CsvImportRowStatus::Failed,
                error: Some(String::from("Missing initials")),
            });
            failed_count += 1;
            continue;
        };

        let Some(name) = get_field("name") else {
            results.push(CsvImportRowResult {
                row_index,
                row_number,
                initials: Some(initials_str.clone()),
                status: CsvImportRowStatus::Failed,
                error: Some(String::from("Missing name")),
            });
            failed_count += 1;
            continue;
        };

        let Some(area_str) = get_field("area_id") else {
            results.push(CsvImportRowResult {
                row_index,
                row_number,
                initials: Some(initials_str.clone()),
                status: CsvImportRowStatus::Failed,
                error: Some(String::from("Missing area_id")),
            });
            failed_count += 1;
            continue;
        };

        let Some(user_type_str) = get_field("user_type") else {
            results.push(CsvImportRowResult {
                row_index,
                row_number,
                initials: Some(initials_str.clone()),
                status: CsvImportRowStatus::Failed,
                error: Some(String::from("Missing user_type")),
            });
            failed_count += 1;
            continue;
        };

        let Some(crew_str) = get_field("crew") else {
            results.push(CsvImportRowResult {
                row_index,
                row_number,
                initials: Some(initials_str.clone()),
                status: CsvImportRowStatus::Failed,
                error: Some(String::from("Missing crew")),
            });
            failed_count += 1;
            continue;
        };

        let Some(service_computation_date) = get_field("service_computation_date") else {
            results.push(CsvImportRowResult {
                row_index,
                row_number,
                initials: Some(initials_str.clone()),
                status: CsvImportRowStatus::Failed,
                error: Some(String::from("Missing service_computation_date")),
            });
            failed_count += 1;
            continue;
        };

        let Some(eod_faa_date) = get_field("eod_faa_date").or_else(|| get_field("eod_date")) else {
            results.push(CsvImportRowResult {
                row_index,
                row_number,
                initials: Some(initials_str.clone()),
                status: CsvImportRowStatus::Failed,
                error: Some(String::from("Missing eod_faa_date or eod_date")),
            });
            failed_count += 1;
            continue;
        };

        // Parse crew
        let Ok(crew_num) = crew_str.parse::<u8>() else {
            results.push(CsvImportRowResult {
                row_index,
                row_number,
                initials: Some(initials_str.clone()),
                status: CsvImportRowStatus::Failed,
                error: Some(format!("Invalid crew number: {crew_str}")),
            });
            failed_count += 1;
            continue;
        };

        // Optional fields
        let cumulative_natca_bu_date = get_field("cumulative_natca_bu_date").unwrap_or_default();
        let natca_bu_date = get_field("natca_bu_date").unwrap_or_default();
        let lottery_value = get_field("lottery_value").and_then(|v| v.parse().ok());

        // Parse domain types
        let initials = Initials::new(&initials_str);
        let area = Area::new(&area_str);

        let user_type = match UserType::parse(&user_type_str).map_err(translate_domain_error) {
            Ok(ut) => ut,
            Err(e) => {
                results.push(CsvImportRowResult {
                    row_index,
                    row_number,
                    initials: Some(initials_str.clone()),
                    status: CsvImportRowStatus::Failed,
                    error: Some(format!("Invalid user type: {e}")),
                });
                failed_count += 1;
                continue;
            }
        };

        let crew = match Crew::new(crew_num).map_err(translate_domain_error) {
            Ok(c) => Some(c),
            Err(e) => {
                results.push(CsvImportRowResult {
                    row_index,
                    row_number,
                    initials: Some(initials_str.clone()),
                    status: CsvImportRowStatus::Failed,
                    error: Some(format!("Invalid crew: {e}")),
                });
                failed_count += 1;
                continue;
            }
        };

        let seniority_data = SeniorityData::new(
            cumulative_natca_bu_date,
            natca_bu_date,
            eod_faa_date,
            service_computation_date,
            lottery_value,
        );

        // Load current state for this user's area from the database
        // This ensures duplicate detection works correctly across areas
        let area_state: State = persistence
            .get_current_state(&active_bid_year, &area)
            .unwrap_or_else(|_| State::new(active_bid_year.clone(), area.clone()));

        // Create the command
        let command = Command::RegisterUser {
            initials: initials.clone(),
            name: name.clone(),
            area: area.clone(),
            user_type,
            crew,
            seniority_data,
        };

        // Attempt to apply the command
        match apply(
            metadata,
            &area_state,
            &active_bid_year,
            command,
            actor.clone(),
            cause.clone(),
        )
        .map_err(translate_core_error)
        {
            Ok(transition_result) => {
                // Persist immediately to ensure subsequent rows see this user
                if let Err(persist_err) = persistence.persist_transition(&transition_result) {
                    results.push(CsvImportRowResult {
                        row_index,
                        row_number,
                        initials: Some(initials.value().to_string()),
                        status: CsvImportRowStatus::Failed,
                        error: Some(format!("Failed to persist: {persist_err}")),
                    });
                    failed_count += 1;
                    continue;
                }

                // Success
                results.push(CsvImportRowResult {
                    row_index,
                    row_number,
                    initials: Some(initials.value().to_string()),
                    status: CsvImportRowStatus::Success,
                    error: None,
                });
                successful_count += 1;
            }
            Err(e) => {
                // Failure
                results.push(CsvImportRowResult {
                    row_index,
                    row_number,
                    initials: Some(initials.value().to_string()),
                    status: CsvImportRowStatus::Failed,
                    error: Some(format!("{e}")),
                });
                failed_count += 1;
            }
        }
    }

    let response = ImportCsvUsersResponse {
        bid_year: active_bid_year.year(),
        total_selected,
        successful_count,
        failed_count,
        results,
    };

    Ok(response)
}

/// Override a user's area assignment after canonicalization.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The override request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
///
/// # Returns
///
/// Returns the audit event ID on success.
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an admin
/// - The lifecycle state is not >= Canonicalized
/// - The override reason is invalid
/// - The target area is a system area
/// - The canonical record does not exist
#[allow(clippy::too_many_lines)]
#[allow(dead_code)]
pub fn override_area_assignment(
    persistence: &mut SqlitePersistence,
    request: &OverrideAreaAssignmentRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<OverrideAreaAssignmentResponse, ApiError> {
    // Enforce authorization - only admins can perform overrides
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("override_area_assignment"),
            required_role: String::from("Admin"),
        });
    }

    // Validate override reason (min 10 chars)
    let reason = request.reason.trim();
    if reason.len() < 10 {
        return Err(translate_domain_error(DomainError::InvalidOverrideReason {
            reason: request.reason.clone(),
        }));
    }

    // Get user details
    let (bid_year_id, user_initials): (i64, String) = persistence
        .get_user_details(request.user_id)
        .map_err(|_| ApiError::ResourceNotFound {
            resource_type: String::from("User"),
            message: format!("User with ID {} not found", request.user_id),
        })?;

    // Check lifecycle state >= Canonicalized
    let lifecycle_state =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    if !matches!(
        lifecycle_state.as_str(),
        "Canonicalized" | "BiddingActive" | "BiddingClosed"
    ) {
        return Err(translate_domain_error(
            DomainError::CannotOverrideBeforeCanonicalization {
                current_state: lifecycle_state,
            },
        ));
    }

    // Verify target area exists and is not a system area
    let (area_code, area_name): (String, Option<String>) = persistence
        .get_area_details(request.new_area_id)
        .map_err(|_| ApiError::ResourceNotFound {
            resource_type: String::from("Area"),
            message: format!("Area with ID {} not found", request.new_area_id),
        })?;

    // Check if target area is a system area
    let is_system = persistence
        .is_system_area(request.new_area_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to check system area: {e}"),
        })?;

    if is_system {
        return Err(translate_domain_error(
            DomainError::CannotAssignToSystemArea { area_code },
        ));
    }

    // Get previous area info for audit event
    let previous_area_id: i64 = persistence
        .get_current_area_assignment(bid_year_id, request.user_id)
        .map_err(|_| {
            translate_domain_error(DomainError::CanonicalRecordNotFound {
                description: format!(
                    "Canonical area membership not found for user_id={}",
                    request.user_id
                ),
            })
        })?;

    let (prev_area_code, prev_area_name): (String, Option<String>) = persistence
        .get_area_details(previous_area_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to fetch previous area info: {e}"),
        })?;

    // Perform override
    let (_, was_already_overridden) = persistence
        .override_area_assignment(bid_year_id, request.user_id, request.new_area_id, reason)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to override area assignment: {e}"),
        })?;

    // Create and persist audit event
    let actor = authenticated_actor.to_audit_actor(operator);
    let cause = Cause::new(
        String::from("override_area_assignment"),
        format!("Override area assignment for user {user_initials}"),
    );

    let action = Action::new(
        String::from("UserAreaAssignmentOverridden"),
        Some(format!(
            "user_id={}, previous_area={}, new_area={}, reason={}, was_overridden={}",
            request.user_id,
            prev_area_name.unwrap_or(prev_area_code),
            area_name.unwrap_or(area_code),
            reason,
            was_already_overridden
        )),
    );

    let before = StateSnapshot::new(format!("area_id={previous_area_id}"));
    let after = StateSnapshot::new(format!("area_id={}", request.new_area_id));

    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get bid year: {e}"),
        })?;
    let bid_year = BidYear::new(year);
    let area = Area::new("_override");

    let audit_event = AuditEvent::new(actor, cause, action, before, after, bid_year, area);

    let event_id =
        persistence
            .persist_audit_event(&audit_event)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to persist audit event: {e}"),
            })?;

    Ok(OverrideAreaAssignmentResponse {
        audit_event_id: event_id,
        message: format!(
            "Area assignment overridden for user {user_initials} (audit event {event_id})"
        ),
    })
}

/// Override a user's eligibility after canonicalization.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The override request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
///
/// # Returns
///
/// Returns the audit event ID on success.
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an admin
/// - The lifecycle state is not >= Canonicalized
/// - The override reason is invalid
/// - The canonical record does not exist
#[allow(dead_code)]
pub fn override_eligibility(
    persistence: &mut SqlitePersistence,
    request: &OverrideEligibilityRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<OverrideEligibilityResponse, ApiError> {
    // Enforce authorization - only admins can perform overrides
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("override_eligibility"),
            required_role: String::from("Admin"),
        });
    }

    // Validate override reason (min 10 chars)
    let reason = request.reason.trim();
    if reason.len() < 10 {
        return Err(translate_domain_error(DomainError::InvalidOverrideReason {
            reason: request.reason.clone(),
        }));
    }

    // Get user details
    let (bid_year_id, user_initials): (i64, String) =
        persistence.get_user_details(request.user_id).map_err(|_| {
            let user_id = request.user_id;
            ApiError::ResourceNotFound {
                resource_type: String::from("User"),
                message: format!("User with ID {user_id} not found"),
            }
        })?;

    // Check lifecycle state >= Canonicalized
    let lifecycle_state =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    if !matches!(
        lifecycle_state.as_str(),
        "Canonicalized" | "BiddingActive" | "BiddingClosed"
    ) {
        return Err(translate_domain_error(
            DomainError::CannotOverrideBeforeCanonicalization {
                current_state: lifecycle_state,
            },
        ));
    }

    // Perform override
    let (previous_eligibility, was_already_overridden) = persistence
        .override_eligibility(bid_year_id, request.user_id, request.can_bid, reason)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to override eligibility: {e}"),
        })?;

    // Create and persist audit event
    let actor = authenticated_actor.to_audit_actor(operator);
    let cause = Cause::new(
        String::from("override_eligibility"),
        format!("Override eligibility for user {user_initials}"),
    );

    let action = Action::new(
        String::from("UserEligibilityOverridden"),
        Some(format!(
            "user_id={}, previous_eligibility={}, new_eligibility={}, reason={}, was_overridden={}",
            request.user_id, previous_eligibility, request.can_bid, reason, was_already_overridden
        )),
    );

    let before = StateSnapshot::new(format!("can_bid={previous_eligibility}"));
    let after = StateSnapshot::new(format!("can_bid={}", request.can_bid));

    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get bid year: {e}"),
        })?;
    let bid_year = BidYear::new(year);
    let area = Area::new("_override");

    let audit_event = AuditEvent::new(actor, cause, action, before, after, bid_year, area);

    let event_id =
        persistence
            .persist_audit_event(&audit_event)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to persist audit event: {e}"),
            })?;

    Ok(OverrideEligibilityResponse {
        audit_event_id: event_id,
        message: format!(
            "Eligibility overridden for user {user_initials} (audit event {event_id})"
        ),
    })
}

/// Override a user's bid order after canonicalization.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The override request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
///
/// # Returns
///
/// Returns the audit event ID on success.
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an admin
/// - The lifecycle state is not >= Canonicalized
/// - The override reason is invalid
/// - The bid order is invalid (must be positive if provided)
/// - The canonical record does not exist
#[allow(dead_code)]
pub fn override_bid_order(
    persistence: &mut SqlitePersistence,
    request: &OverrideBidOrderRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<OverrideBidOrderResponse, ApiError> {
    // Enforce authorization - only admins can perform overrides
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("override_bid_order"),
            required_role: String::from("Admin"),
        });
    }

    // Validate override reason (min 10 chars)
    let reason = request.reason.trim();
    if reason.len() < 10 {
        return Err(translate_domain_error(DomainError::InvalidOverrideReason {
            reason: request.reason.clone(),
        }));
    }

    // Validate bid order if provided
    if let Some(order) = request.bid_order
        && order <= 0
    {
        return Err(translate_domain_error(DomainError::InvalidBidOrder {
            reason: format!("Bid order must be positive (got: {order})"),
        }));
    }

    // Get user details
    let (bid_year_id, user_initials): (i64, String) =
        persistence.get_user_details(request.user_id).map_err(|_| {
            let user_id = request.user_id;
            ApiError::ResourceNotFound {
                resource_type: String::from("User"),
                message: format!("User with ID {user_id} not found"),
            }
        })?;

    // Check lifecycle state >= Canonicalized
    let lifecycle_state =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    if !matches!(
        lifecycle_state.as_str(),
        "Canonicalized" | "BiddingActive" | "BiddingClosed"
    ) {
        return Err(translate_domain_error(
            DomainError::CannotOverrideBeforeCanonicalization {
                current_state: lifecycle_state,
            },
        ));
    }

    // Perform override
    let (previous_bid_order, was_already_overridden) = persistence
        .override_bid_order(bid_year_id, request.user_id, request.bid_order, reason)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to override bid order: {e}"),
        })?;

    // Create and persist audit event
    let actor = authenticated_actor.to_audit_actor(operator);
    let cause = Cause::new(
        String::from("override_bid_order"),
        format!("Override bid order for user {user_initials}"),
    );

    let action = Action::new(
        String::from("UserBidOrderOverridden"),
        Some(format!(
            "user_id={}, previous_bid_order={:?}, new_bid_order={:?}, reason={}, was_overridden={}",
            request.user_id, previous_bid_order, request.bid_order, reason, was_already_overridden
        )),
    );

    let before = StateSnapshot::new(format!("bid_order={previous_bid_order:?}"));
    let after = StateSnapshot::new(format!("bid_order={:?}", request.bid_order));

    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get bid year: {e}"),
        })?;
    let bid_year = BidYear::new(year);
    let area = Area::new("_override");

    let audit_event = AuditEvent::new(actor, cause, action, before, after, bid_year, area);

    let event_id =
        persistence
            .persist_audit_event(&audit_event)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to persist audit event: {e}"),
            })?;

    Ok(OverrideBidOrderResponse {
        audit_event_id: event_id,
        message: format!("Bid order overridden for user {user_initials} (audit event {event_id})"),
    })
}

/// Override a user's bid window after canonicalization.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The override request
/// * `authenticated_actor` - The authenticated actor performing this action
/// * `operator` - The operator data
///
/// # Returns
///
/// Returns the audit event ID on success.
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an admin
/// - The lifecycle state is not >= Canonicalized
/// - The override reason is invalid
/// - The bid window dates are invalid (start > end, partial window)
/// - The canonical record does not exist
#[allow(clippy::too_many_lines)]
#[allow(dead_code)]
pub fn override_bid_window(
    persistence: &mut SqlitePersistence,
    request: &OverrideBidWindowRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<OverrideBidWindowResponse, ApiError> {
    // Enforce authorization - only admins can perform overrides
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("override_bid_window"),
            required_role: String::from("Admin"),
        });
    }

    // Validate override reason (min 10 chars)
    let reason = request.reason.trim();
    if reason.len() < 10 {
        return Err(translate_domain_error(DomainError::InvalidOverrideReason {
            reason: request.reason.clone(),
        }));
    }

    // Validate bid window - both must be present or both must be None
    match (&request.window_start, &request.window_end) {
        (Some(start), Some(end)) => {
            // Parse dates to validate format and ordering
            let start_date = time::Date::parse(
                start,
                time::macros::format_description!("[year]-[month]-[day]"),
            )
            .map_err(|e| {
                translate_domain_error(DomainError::DateParseError {
                    date_string: start.clone(),
                    error: e.to_string(),
                })
            })?;
            let end_date = time::Date::parse(
                end,
                time::macros::format_description!("[year]-[month]-[day]"),
            )
            .map_err(|e| {
                translate_domain_error(DomainError::DateParseError {
                    date_string: end.clone(),
                    error: e.to_string(),
                })
            })?;

            if start_date > end_date {
                return Err(translate_domain_error(DomainError::InvalidBidWindow {
                    reason: format!("Window start date ({start}) must be <= end date ({end})"),
                }));
            }
        }
        (None, None) => {
            // Both None is valid (clears the window)
        }
        _ => {
            return Err(translate_domain_error(DomainError::InvalidBidWindow {
                reason: String::from(
                    "Both window_start and window_end must be provided or both must be null",
                ),
            }));
        }
    }

    // Get user details
    let (bid_year_id, user_initials): (i64, String) =
        persistence.get_user_details(request.user_id).map_err(|_| {
            let user_id = request.user_id;
            ApiError::ResourceNotFound {
                resource_type: String::from("User"),
                message: format!("User with ID {user_id} not found"),
            }
        })?;

    // Check lifecycle state >= Canonicalized
    let lifecycle_state =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    if !matches!(
        lifecycle_state.as_str(),
        "Canonicalized" | "BiddingActive" | "BiddingClosed"
    ) {
        return Err(translate_domain_error(
            DomainError::CannotOverrideBeforeCanonicalization {
                current_state: lifecycle_state,
            },
        ));
    }

    // Perform override
    let (previous_start, previous_end, was_already_overridden) = persistence
        .override_bid_window(
            bid_year_id,
            request.user_id,
            request.window_start.as_ref(),
            request.window_end.as_ref(),
            reason,
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to override bid window: {e}"),
        })?;

    // Create and persist audit event
    let actor = authenticated_actor.to_audit_actor(operator);
    let cause = Cause::new(
        String::from("override_bid_window"),
        format!("Override bid window for user {user_initials}"),
    );

    let action = Action::new(
        String::from("UserBidWindowOverridden"),
        Some(format!(
            "user_id={}, previous_start={:?}, previous_end={:?}, new_start={:?}, new_end={:?}, reason={}, was_overridden={}",
            request.user_id,
            previous_start,
            previous_end,
            request.window_start,
            request.window_end,
            reason,
            was_already_overridden
        )),
    );

    let before = StateSnapshot::new(format!(
        "window_start={previous_start:?}, window_end={previous_end:?}"
    ));
    let after = StateSnapshot::new(format!(
        "window_start={:?}, window_end={:?}",
        request.window_start, request.window_end
    ));

    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get bid year: {e}"),
        })?;
    let bid_year = BidYear::new(year);
    let area = Area::new("_override");

    let audit_event = AuditEvent::new(actor, cause, action, before, after, bid_year, area);

    let event_id =
        persistence
            .persist_audit_event(&audit_event)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to persist audit event: {e}"),
            })?;

    Ok(OverrideBidWindowResponse {
        audit_event_id: event_id,
        message: format!("Bid window overridden for user {user_initials} (audit event {event_id})"),
    })
}

// ============================================================================
// Phase 29G: Post-Confirmation Bid Order Adjustments
// ============================================================================

/// Adjust bid order for multiple users in bulk.
///
/// # Arguments
///
/// * `persistence` - Persistence layer
/// * `bid_year_id` - The bid year ID
/// * `area_id` - The area ID
/// * `request` - The bulk adjustment request
/// * `authenticated_actor` - The authenticated actor performing the adjustment
/// * `operator` - The operator data
///
/// # Returns
///
/// Returns a success response with the audit event ID.
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an admin
/// - The reason is too short
/// - Any bid order value is invalid
/// - The lifecycle state is not Canonicalized or later
/// - The database operation fails
pub fn adjust_bid_order(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
    area_id: i64,
    request: &AdjustBidOrderRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<AdjustBidOrderResponse, ApiError> {
    // Enforce authorization - only admins can perform adjustments
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("adjust_bid_order"),
            required_role: String::from("Admin"),
        });
    }

    // Validate reason (min 10 chars)
    let reason = request.reason.trim();
    if reason.len() < 10 {
        return Err(translate_domain_error(DomainError::InvalidOverrideReason {
            reason: request.reason.clone(),
        }));
    }

    // Validate all bid orders are positive
    for adjustment in &request.adjustments {
        if adjustment.new_bid_order <= 0 {
            return Err(translate_domain_error(DomainError::InvalidBidOrder {
                reason: format!(
                    "Bid order must be positive (got: {})",
                    adjustment.new_bid_order
                ),
            }));
        }
    }

    // Check lifecycle state >= Canonicalized
    let lifecycle_state =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    if !matches!(
        lifecycle_state.as_str(),
        "Canonicalized" | "BiddingActive" | "BiddingClosed"
    ) {
        return Err(translate_domain_error(
            DomainError::CannotOverrideBeforeCanonicalization {
                current_state: lifecycle_state,
            },
        ));
    }

    // Apply adjustments
    let mut users_adjusted = 0;
    for adjustment in &request.adjustments {
        // Verify user exists and get details
        let (_user_bid_year_id, _user_initials) = persistence
            .get_user_details(adjustment.user_id)
            .map_err(|_| ApiError::ResourceNotFound {
                resource_type: String::from("User"),
                message: format!("User with ID {} not found", adjustment.user_id),
            })?;

        // Perform override using existing function
        persistence
            .override_bid_order(
                bid_year_id,
                adjustment.user_id,
                Some(adjustment.new_bid_order),
                reason,
            )
            .map_err(|e| ApiError::Internal {
                message: format!(
                    "Failed to adjust bid order for user {}: {e}",
                    adjustment.user_id
                ),
            })?;

        users_adjusted += 1;
    }

    // Create and persist audit event
    let actor = authenticated_actor.to_audit_actor(operator);
    let cause = Cause::new(
        String::from("adjust_bid_order"),
        format!("Bulk bid order adjustment for {users_adjusted} users"),
    );

    let action = Action::new(
        String::from("BulkBidOrderAdjustment"),
        Some(format!(
            "area_id={area_id}, users_adjusted={users_adjusted}, reason={reason}"
        )),
    );

    let before = StateSnapshot::new(String::from("bulk_adjustment_requested"));
    let after = StateSnapshot::new(format!("users_adjusted={users_adjusted}"));

    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get bid year: {e}"),
        })?;
    let bid_year = BidYear::new(year);
    let area = Area::new("_bulk_adjustment");

    let audit_event = AuditEvent::new(actor, cause, action, before, after, bid_year, area);

    let event_id =
        persistence
            .persist_audit_event(&audit_event)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to persist audit event: {e}"),
            })?;

    Ok(AdjustBidOrderResponse {
        audit_event_id: event_id,
        users_adjusted,
        message: format!("Adjusted bid order for {users_adjusted} users (audit event {event_id})"),
    })
}

/// Adjust a bid window for a specific user and round.
///
/// # Arguments
///
/// * `persistence` - Persistence layer
/// * `bid_year_id` - The bid year ID
/// * `area_id` - The area ID
/// * `request` - The adjustment request
/// * `authenticated_actor` - The authenticated actor performing the adjustment
/// * `operator` - The operator data
///
/// # Returns
///
/// Returns a success response with the audit event ID.
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an admin
/// - The reason is too short
/// - The window start/end datetimes are invalid
/// - The lifecycle state is not Canonicalized or later
/// - The database operation fails
pub fn adjust_bid_window(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
    area_id: i64,
    request: &AdjustBidWindowRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<AdjustBidWindowResponse, ApiError> {
    // Enforce authorization - only admins can perform adjustments
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("adjust_bid_window"),
            required_role: String::from("Admin"),
        });
    }

    // Validate reason (min 10 chars)
    let reason = request.reason.trim();
    if reason.len() < 10 {
        return Err(translate_domain_error(DomainError::InvalidOverrideReason {
            reason: request.reason.clone(),
        }));
    }

    // Validate window times (basic format check - detailed validation happens in persistence layer)
    let window_start = &request.new_window_start;
    let window_end = &request.new_window_end;
    if window_start >= window_end {
        return Err(translate_domain_error(DomainError::InvalidBidWindow {
            reason: format!(
                "Window start ({window_start}) must be before window end ({window_end})"
            ),
        }));
    }

    let (user_initials, previous_start, previous_end) = adjust_bid_window_impl(
        persistence,
        bid_year_id,
        area_id,
        request.user_id,
        request.round_id,
        &request.new_window_start,
        &request.new_window_end,
    )?;

    // Create and persist audit event
    let actor = authenticated_actor.to_audit_actor(operator);
    let cause = Cause::new(
        String::from("adjust_bid_window"),
        format!(
            "Adjust bid window for user {user_initials}, round {}",
            request.round_id
        ),
    );

    let user_id = request.user_id;
    let round_id = request.round_id;
    let new_start = &request.new_window_start;
    let new_end = &request.new_window_end;

    let action = Action::new(
        String::from("BidWindowAdjusted"),
        Some(format!(
            "user_id={user_id}, round_id={round_id}, previous_start={previous_start}, previous_end={previous_end}, new_start={new_start}, new_end={new_end}, reason={reason}"
        )),
    );

    let before = StateSnapshot::new(format!(
        "window_start={previous_start}, window_end={previous_end}"
    ));
    let after = StateSnapshot::new(format!("window_start={new_start}, window_end={new_end}"));

    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get bid year: {e}"),
        })?;
    let bid_year = BidYear::new(year);
    let area = Area::new("_window_adjustment");

    let audit_event = AuditEvent::new(actor, cause, action, before, after, bid_year, area);

    let event_id =
        persistence
            .persist_audit_event(&audit_event)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to persist audit event: {e}"),
            })?;

    let round_id = request.round_id;
    Ok(AdjustBidWindowResponse {
        audit_event_id: event_id,
        message: format!(
            "Adjusted bid window for user {user_initials}, round {round_id} (audit event {event_id})"
        ),
    })
}

/// Internal helper for bid window adjustment implementation.
fn adjust_bid_window_impl(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
    area_id: i64,
    user_id: i64,
    round_id: i64,
    new_window_start: &str,
    new_window_end: &str,
) -> Result<(String, String, String), ApiError> {
    // Get user details
    let (_user_bid_year_id, user_initials) =
        persistence
            .get_user_details(user_id)
            .map_err(|_| ApiError::ResourceNotFound {
                resource_type: String::from("User"),
                message: format!("User with ID {user_id} not found"),
            })?;

    // Check lifecycle state >= Canonicalized
    let lifecycle_state =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    if !matches!(
        lifecycle_state.as_str(),
        "Canonicalized" | "BiddingActive" | "BiddingClosed"
    ) {
        return Err(translate_domain_error(
            DomainError::CannotOverrideBeforeCanonicalization {
                current_state: lifecycle_state,
            },
        ));
    }

    // Perform adjustment
    let (previous_start, previous_end) = persistence
        .adjust_bid_window(
            bid_year_id,
            area_id,
            user_id,
            round_id,
            new_window_start,
            new_window_end,
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to adjust bid window: {e}"),
        })?;

    Ok((user_initials, previous_start, previous_end))
}

/// Recalculate bid windows for multiple users and rounds in bulk.
///
/// This endpoint deletes existing bid windows and allows them to be recalculated.
/// The actual recalculation logic is expected to be invoked separately.
///
/// # Arguments
///
/// * `persistence` - Persistence layer
/// * `bid_year_id` - The bid year ID
/// * `area_id` - The area ID
/// * `request` - The recalculation request
/// * `authenticated_actor` - The authenticated actor performing the recalculation
/// * `operator` - The operator data
///
/// # Returns
///
/// Returns a success response with the audit event ID.
///
/// # Errors
///
/// Returns an error if:
/// - The actor is not an admin
/// - The reason is too short
/// - The lifecycle state is not Canonicalized or later
/// - The database operation fails
pub fn recalculate_bid_windows(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
    area_id: i64,
    request: &RecalculateBidWindowsRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<RecalculateBidWindowsResponse, ApiError> {
    // Enforce authorization - only admins can perform recalculations
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("recalculate_bid_windows"),
            required_role: String::from("Admin"),
        });
    }

    // Validate reason (min 10 chars)
    let reason = request.reason.trim();
    if reason.len() < 10 {
        return Err(translate_domain_error(DomainError::InvalidOverrideReason {
            reason: request.reason.clone(),
        }));
    }

    // Check lifecycle state >= Canonicalized
    let lifecycle_state =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    if !matches!(
        lifecycle_state.as_str(),
        "Canonicalized" | "BiddingActive" | "BiddingClosed"
    ) {
        return Err(translate_domain_error(
            DomainError::CannotOverrideBeforeCanonicalization {
                current_state: lifecycle_state,
            },
        ));
    }

    // Delete existing bid windows for the specified users and rounds
    let windows_deleted = persistence
        .delete_bid_windows_for_users_and_rounds(
            bid_year_id,
            area_id,
            &request.user_ids,
            &request.rounds,
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to delete bid windows: {e}"),
        })?;

    // Create and persist audit event
    let actor = authenticated_actor.to_audit_actor(operator);
    let cause = Cause::new(
        String::from("recalculate_bid_windows"),
        format!(
            "Bulk bid window recalculation for {} users, {} rounds",
            request.user_ids.len(),
            request.rounds.len()
        ),
    );

    let action = Action::new(
        String::from("BulkBidWindowRecalculation"),
        Some(format!(
            "area_id={area_id}, user_count={}, round_count={}, windows_deleted={windows_deleted}, reason={reason}",
            request.user_ids.len(),
            request.rounds.len()
        )),
    );

    let before = StateSnapshot::new(format!("windows_existed={windows_deleted}"));
    let after = StateSnapshot::new(String::from("windows_deleted_for_recalculation"));

    let year = persistence
        .get_bid_year_from_id(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get bid year: {e}"),
        })?;
    let bid_year = BidYear::new(year);
    let area = Area::new("_window_recalculation");

    let audit_event = AuditEvent::new(actor, cause, action, before, after, bid_year, area);

    let event_id =
        persistence
            .persist_audit_event(&audit_event)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to persist audit event: {e}"),
            })?;

    Ok(RecalculateBidWindowsResponse {
        audit_event_id: event_id,
        windows_recalculated: windows_deleted,
        message: format!(
            "Deleted {windows_deleted} bid windows for recalculation (audit event {event_id})"
        ),
    })
}

/// Update a user's participation flags.
///
/// Phase 29A: Controls bid order derivation and leave calculation inclusion.
///
/// # Directional Invariant
///
/// `excluded_from_leave_calculation == true` ⇒ `excluded_from_bidding == true`
///
/// A user may never be included in bidding while excluded from leave calculation.
///
/// # Lifecycle Constraints
///
/// Flags are editable in `Draft` and `BootstrapComplete` states.
/// After canonicalization, flags become immutable (or require override).
///
/// # Arguments
///
/// * `metadata` - Bootstrap metadata
/// * `persistence` - Persistence layer
/// * `request` - The participation flag update request
/// * `authenticated_actor` - The authenticated actor performing the update
///
/// # Returns
///
/// * `Ok(UpdateUserParticipationResponse)` on success
/// * `Err(ApiError)` on validation failure or lifecycle constraint violation
///
/// # Errors
///
/// Returns an error if:
/// - User does not exist
/// - Directional invariant is violated
/// - Lifecycle state does not allow flag updates
#[allow(clippy::too_many_arguments)]
pub fn update_user_participation(
    metadata: &BootstrapMetadata,
    persistence: &mut SqlitePersistence,
    request: &crate::request_response::UpdateUserParticipationRequest,
    authenticated_actor: &Actor,
    lifecycle_state: zab_bid_domain::BidYearLifecycle,
) -> Result<crate::request_response::UpdateUserParticipationResponse, ApiError> {
    use zab_bid_domain::DomainError;

    // Enforce lifecycle constraints: participation flags locked after Canonicalized
    if lifecycle_state.is_locked() {
        return Err(ApiError::DomainRuleViolation {
            rule: String::from("participation_flags_lifecycle"),
            message: format!(
                "Cannot update participation flags in state '{lifecycle_state}': structural changes locked after confirmation"
            ),
        });
    }

    // Validate directional invariant before constructing command
    if request.excluded_from_leave_calculation && !request.excluded_from_bidding {
        return Err(translate_domain_error(
            DomainError::ParticipationFlagViolation {
                user_initials: format!("user_id={}", request.user_id),
                reason: String::from(
                    "User excluded from leave calculation must also be excluded from bidding",
                ),
            },
        ));
    }

    // Resolve the active bid year from canonical state
    let active_bid_year: BidYear = resolve_active_bid_year(persistence)?;

    // Find bid_year_id
    let bid_year_id: i64 = metadata
        .bid_years
        .iter()
        .find(|by| by.year() == active_bid_year.year())
        .and_then(BidYear::bid_year_id)
        .ok_or_else(|| ApiError::Internal {
            message: format!(
                "Active bid year {} has no ID in metadata",
                active_bid_year.year()
            ),
        })?;

    // We need to iterate through all areas to find the user
    // since we don't know which area the user is in
    let mut found_user: Option<(zab_bid_domain::User, Area, State)> = None;

    for (by, area_meta) in &metadata.areas {
        if by.year() != active_bid_year.year() {
            continue;
        }

        let area = Area::new(area_meta.area_code());

        // Try to load state for this area
        let Ok(state) = persistence.get_current_state(&active_bid_year, &area) else {
            continue; // Skip areas with no state
        };

        // Check if the user is in this area
        if let Some(user) = state
            .users
            .iter()
            .find(|u| u.user_id == Some(request.user_id))
        {
            found_user = Some((user.clone(), area, state));
            break;
        }
    }

    let (user, _area, state) = found_user.ok_or_else(|| ApiError::ResourceNotFound {
        resource_type: String::from("User"),
        message: format!(
            "User with user_id={} not found in active bid year",
            request.user_id
        ),
    })?;

    // Create the command
    let command: Command = Command::UpdateUserParticipation {
        user_id: request.user_id,
        initials: user.initials.clone(),
        excluded_from_bidding: request.excluded_from_bidding,
        excluded_from_leave_calculation: request.excluded_from_leave_calculation,
    };

    // Apply the command
    let cause = Cause::new(
        String::from("update_user_participation"),
        format!(
            "Update participation flags for user {}",
            user.initials.value()
        ),
    );
    let result: TransitionResult = apply(
        metadata,
        &state,
        &active_bid_year,
        command,
        authenticated_actor.clone(),
        cause,
    )
    .map_err(translate_core_error)?;

    // Persist the audit event and new state
    persistence
        .persist_audit_event(&result.audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    Ok(crate::request_response::UpdateUserParticipationResponse {
        bid_year_id,
        bid_year: active_bid_year.year(),
        user_id: request.user_id,
        initials: user.initials.value().to_string(),
        excluded_from_bidding: request.excluded_from_bidding,
        excluded_from_leave_calculation: request.excluded_from_leave_calculation,
        message: format!(
            "Updated participation flags for user '{}'",
            user.initials.value()
        ),
    })
}

// TODO Phase 26C: Add integration tests for update_area handler:
// - test_update_area_allowed_in_draft
// - test_update_area_denied_after_canonicalization
// - test_update_area_denied_for_system_area
// - test_update_area_requires_admin
// - test_update_area_creates_audit_event

// ============================================================================
// Phase 29B: Round Groups and Rounds
// ============================================================================

/// Creates a new round group for a bid year.
///
/// Round groups are editable in `Draft` and `BootstrapComplete` states.
/// After canonicalization, round configuration becomes immutable (or requires override).
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `bid_year_id` - The bid year ID this round group belongs to
/// * `request` - The round group creation request
/// * `authenticated_actor` - The authenticated actor performing the operation
///
/// # Returns
///
/// * `Ok(CreateRoundGroupResponse)` on success
/// * `Err(ApiError)` on validation failure or lifecycle constraint violation
///
/// # Errors
///
/// Returns an error if:
/// - Actor is not authorized (Admin role required)
/// - Lifecycle state does not allow round group creation
/// - Round group name already exists in bid year
/// - Validation fails
#[allow(dead_code)]
pub fn create_round_group(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
    request: &crate::request_response::CreateRoundGroupRequest,
    authenticated_actor: &AuthenticatedActor,
) -> Result<crate::request_response::CreateRoundGroupResponse, ApiError> {
    use zab_bid_domain::BidYearLifecycle;

    // Enforce authorization - only admins can manage round groups
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("create_round_group"),
            required_role: String::from("Admin"),
        });
    }

    // Enforce lifecycle constraints: round configuration locked after Canonicalized
    let lifecycle_state_str: String =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    let lifecycle_state: BidYearLifecycle = lifecycle_state_str
        .parse()
        .map_err(translate_domain_error)?;

    if lifecycle_state.is_locked() {
        return Err(ApiError::DomainRuleViolation {
            rule: String::from("round_group_lifecycle"),
            message: format!(
                "Cannot create round group in state '{lifecycle_state}': structural changes locked after confirmation"
            ),
        });
    }

    // Validate round group name is not empty
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidInput {
            field: String::from("name"),
            message: String::from("Round group name cannot be empty"),
        });
    }

    // Check for duplicate name
    let name_exists = persistence
        .round_group_name_exists(bid_year_id, &request.name, None)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to check round group name: {e}"),
        })?;

    if name_exists {
        return Err(translate_domain_error(
            DomainError::DuplicateRoundGroupName {
                bid_year: 0, // We don't have the year value here, but error translation handles it
                name: request.name.clone(),
            },
        ));
    }

    // Insert the round group
    let round_group_id = persistence
        .insert_round_group(bid_year_id, &request.name, request.editing_enabled)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to insert round group: {e}"),
        })?;

    Ok(crate::request_response::CreateRoundGroupResponse {
        round_group_id,
        bid_year_id,
        name: request.name.clone(),
        editing_enabled: request.editing_enabled,
        message: format!("Created round group '{}'", request.name),
    })
}

/// Lists all round groups for a bid year.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `bid_year_id` - The bid year ID
/// * `authenticated_actor` - The authenticated actor performing the operation
///
/// # Returns
///
/// * `Ok(ListRoundGroupsResponse)` on success
/// * `Err(ApiError)` on query failure
///
/// # Errors
///
/// Returns an error if:
/// - Actor is not authorized (Admin role required)
/// - Database query fails
///
/// # Panics
///
/// Panics if a persisted round group does not have an ID.
#[allow(dead_code)]
pub fn list_round_groups(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
    authenticated_actor: &AuthenticatedActor,
) -> Result<crate::request_response::ListRoundGroupsResponse, ApiError> {
    // Enforce authorization - only admins can view round groups
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("list_round_groups"),
            required_role: String::from("Admin"),
        });
    }

    let round_groups: Vec<RoundGroup> =
        persistence
            .list_round_groups(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to list round groups: {e}"),
            })?;

    let round_group_infos: Vec<crate::request_response::RoundGroupInfo> = round_groups
        .into_iter()
        .map(|rg| {
            let round_group_id = rg.round_group_id().ok_or_else(|| ApiError::Internal {
                message: String::from("persisted round group missing ID"),
            })?;
            Ok(crate::request_response::RoundGroupInfo {
                round_group_id,
                bid_year_id,
                name: rg.name().to_string(),
                editing_enabled: rg.editing_enabled(),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(crate::request_response::ListRoundGroupsResponse {
        bid_year_id,
        round_groups: round_group_infos,
    })
}

/// Updates an existing round group.
///
/// Round groups are editable in `Draft` and `BootstrapComplete` states.
/// After canonicalization, round configuration becomes immutable (or requires override).
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The round group update request
/// * `authenticated_actor` - The authenticated actor performing the operation
///
/// # Returns
///
/// * `Ok(UpdateRoundGroupResponse)` on success
/// * `Err(ApiError)` on validation failure or lifecycle constraint violation
///
/// # Errors
///
/// Returns an error if:
/// - Actor is not authorized (Admin role required)
/// - Round group does not exist
/// - Lifecycle state does not allow updates
/// - Round group name already exists (duplicate)
///
/// # Panics
///
/// Panics if the persisted round group's bid year does not have an ID.
#[allow(dead_code)]
pub fn update_round_group(
    persistence: &mut SqlitePersistence,
    request: &crate::request_response::UpdateRoundGroupRequest,
    authenticated_actor: &AuthenticatedActor,
) -> Result<crate::request_response::UpdateRoundGroupResponse, ApiError> {
    use zab_bid_domain::BidYearLifecycle;

    // Enforce authorization - only admins can manage round groups
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("update_round_group"),
            required_role: String::from("Admin"),
        });
    }

    // Get the existing round group to find its bid_year_id
    let existing_rg: RoundGroup = persistence
        .get_round_group(request.round_group_id)
        .map_err(|e| match e {
            PersistenceError::NotFound(_) => {
                translate_domain_error(DomainError::RoundGroupNotFound {
                    round_group_id: request.round_group_id,
                })
            }
            _ => ApiError::Internal {
                message: format!("Failed to get round group: {e}"),
            },
        })?;

    let bid_year_id = existing_rg
        .bid_year()
        .bid_year_id()
        .ok_or_else(|| ApiError::Internal {
            message: String::from("persisted bid year missing ID"),
        })?;

    // Enforce lifecycle constraints
    let lifecycle_state_str: String =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    let lifecycle_state: BidYearLifecycle = lifecycle_state_str
        .parse()
        .map_err(translate_domain_error)?;

    if lifecycle_state.is_locked() {
        return Err(ApiError::DomainRuleViolation {
            rule: String::from("round_group_lifecycle"),
            message: format!(
                "Cannot update round group in state '{lifecycle_state}': structural changes locked after confirmation"
            ),
        });
    }

    // Validate round group name is not empty
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidInput {
            field: String::from("name"),
            message: String::from("Round group name cannot be empty"),
        });
    }

    // Check for duplicate name (excluding this round group)
    let name_exists = persistence
        .round_group_name_exists(bid_year_id, &request.name, Some(request.round_group_id))
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to check round group name: {e}"),
        })?;

    if name_exists {
        return Err(translate_domain_error(
            DomainError::DuplicateRoundGroupName {
                bid_year: 0,
                name: request.name.clone(),
            },
        ));
    }

    // Update the round group
    persistence
        .update_round_group(
            request.round_group_id,
            &request.name,
            request.editing_enabled,
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update round group: {e}"),
        })?;

    Ok(crate::request_response::UpdateRoundGroupResponse {
        round_group_id: request.round_group_id,
        bid_year_id,
        name: request.name.clone(),
        editing_enabled: request.editing_enabled,
        message: format!("Updated round group '{}'", request.name),
    })
}

/// Deletes a round group.
///
/// Round groups can only be deleted if no rounds reference them.
/// Deletion is only allowed in `Draft` and `BootstrapComplete` states.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `round_group_id` - The round group ID to delete
/// * `authenticated_actor` - The authenticated actor performing the operation
///
/// # Returns
///
/// * `Ok(DeleteRoundGroupResponse)` on success
/// * `Err(ApiError)` on validation failure or lifecycle constraint violation
///
/// # Errors
///
/// Returns an error if:
/// - Actor is not authorized (Admin role required)
/// - Round group does not exist
/// - Lifecycle state does not allow deletion
/// - Round group is referenced by rounds
///
/// # Panics
///
/// Panics if the persisted round group's bid year does not have an ID.
#[allow(dead_code)]
pub fn delete_round_group(
    persistence: &mut SqlitePersistence,
    round_group_id: i64,
    authenticated_actor: &AuthenticatedActor,
) -> Result<crate::request_response::DeleteRoundGroupResponse, ApiError> {
    use zab_bid_domain::BidYearLifecycle;

    // Enforce authorization - only admins can manage round groups
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("delete_round_group"),
            required_role: String::from("Admin"),
        });
    }

    // Get the existing round group to find its bid_year_id
    let existing_rg: RoundGroup =
        persistence
            .get_round_group(round_group_id)
            .map_err(|e| match e {
                PersistenceError::NotFound(_) => {
                    translate_domain_error(DomainError::RoundGroupNotFound { round_group_id })
                }
                _ => ApiError::Internal {
                    message: format!("Failed to get round group: {e}"),
                },
            })?;

    let bid_year_id = existing_rg
        .bid_year()
        .bid_year_id()
        .ok_or_else(|| ApiError::Internal {
            message: String::from("persisted bid year missing ID"),
        })?;

    // Enforce lifecycle constraints
    let lifecycle_state_str: String =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    let lifecycle_state: BidYearLifecycle = lifecycle_state_str
        .parse()
        .map_err(translate_domain_error)?;

    if lifecycle_state.is_locked() {
        return Err(ApiError::DomainRuleViolation {
            rule: String::from("round_group_lifecycle"),
            message: format!(
                "Cannot delete round group in state '{lifecycle_state}': structural changes locked after confirmation"
            ),
        });
    }

    // Check if round group is in use
    let round_count = persistence
        .count_rounds_using_group(round_group_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to check round group usage: {e}"),
        })?;

    if round_count > 0 {
        return Err(translate_domain_error(DomainError::RoundGroupInUse {
            round_group_id,
            round_count,
        }));
    }

    // Delete the round group
    persistence
        .delete_round_group(round_group_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to delete round group: {e}"),
        })?;

    Ok(crate::request_response::DeleteRoundGroupResponse {
        message: format!("Deleted round group '{}'", existing_rg.name()),
    })
}

/// Creates a new round in a round group.
///
/// Rounds are editable in `Draft` and `BootstrapComplete` states.
/// After canonicalization, round configuration becomes immutable (or requires override).
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `round_group_id` - The round group ID this round belongs to
/// * `request` - The round creation request
/// * `authenticated_actor` - The authenticated actor performing the operation
///
/// # Returns
///
/// * `Ok(CreateRoundResponse)` on success
/// * `Err(ApiError)` on validation failure or lifecycle constraint violation
///
/// # Errors
///
/// Returns an error if:
/// - Actor is not authorized (Admin role required)
/// - Round group does not exist
/// - Lifecycle state does not allow round creation
/// - Round number already exists in round group
/// - Validation fails (`slots_per_day`, `max_groups`, `max_total_hours` must be > 0)
///
/// # Panics
///
/// Panics if the persisted round group does not have a `bid_year_id`.
#[allow(dead_code)]
#[allow(clippy::too_many_lines)]
pub fn create_round(
    persistence: &mut SqlitePersistence,
    round_group_id: i64,
    request: &crate::request_response::CreateRoundRequest,
    authenticated_actor: &AuthenticatedActor,
) -> Result<crate::request_response::CreateRoundResponse, ApiError> {
    use zab_bid_domain::BidYearLifecycle;

    // Enforce authorization - only admins can manage rounds
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("create_round"),
            required_role: String::from("Admin"),
        });
    }

    // Get area to validate it exists and get bid_year_id
    // Verify round group exists and get its bid year
    let round_group = persistence
        .get_round_group(round_group_id)
        .map_err(|e| match e {
            PersistenceError::NotFound(_) => {
                translate_domain_error(DomainError::RoundGroupNotFound { round_group_id })
            }
            _ => ApiError::Internal {
                message: format!("Failed to get round group: {e}"),
            },
        })?;

    let bid_year_id = round_group
        .bid_year()
        .bid_year_id()
        .ok_or_else(|| ApiError::Internal {
            message: String::from("persisted bid year missing ID"),
        })?;

    // Enforce lifecycle constraints
    let lifecycle_state_str: String =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    let lifecycle_state: BidYearLifecycle = lifecycle_state_str
        .parse()
        .map_err(translate_domain_error)?;

    if lifecycle_state.is_locked() {
        return Err(ApiError::DomainRuleViolation {
            rule: String::from("round_lifecycle"),
            message: format!(
                "Cannot create round in state '{lifecycle_state}': structural changes locked after confirmation"
            ),
        });
    }

    // Validate round configuration
    if request.slots_per_day == 0 {
        return Err(ApiError::InvalidInput {
            field: String::from("slots_per_day"),
            message: String::from("slots_per_day must be greater than 0"),
        });
    }
    if request.max_groups == 0 {
        return Err(ApiError::InvalidInput {
            field: String::from("max_groups"),
            message: String::from("max_groups must be greater than 0"),
        });
    }
    if request.max_total_hours == 0 {
        return Err(ApiError::InvalidInput {
            field: String::from("max_total_hours"),
            message: String::from("max_total_hours must be greater than 0"),
        });
    }
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidInput {
            field: String::from("name"),
            message: String::from("Round name cannot be empty"),
        });
    }

    // Check for duplicate round number within the round group
    let round_number_exists = persistence
        .round_number_exists(round_group_id, request.round_number, None)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to check round number: {e}"),
        })?;

    if round_number_exists {
        return Err(translate_domain_error(DomainError::DuplicateRoundNumber {
            area_code: round_group.name().to_string(),
            round_number: request.round_number,
        }));
    }

    // Insert the round
    let round_id = persistence
        .insert_round(
            round_group_id,
            request.round_number,
            &request.name,
            request.slots_per_day,
            request.max_groups,
            request.max_total_hours,
            request.include_holidays,
            request.allow_overbid,
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to insert round: {e}"),
        })?;

    Ok(crate::request_response::CreateRoundResponse {
        round_id,
        round_group_id,
        round_number: request.round_number,
        name: request.name.clone(),
        message: format!("Created round {} '{}'", request.round_number, request.name),
    })
}

/// Lists all rounds in a round group.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `round_group_id` - The round group ID
/// * `authenticated_actor` - The authenticated actor performing the operation
///
/// # Returns
///
/// * `Ok(ListRoundsResponse)` on success
/// * `Err(ApiError)` on query failure
///
/// # Errors
///
/// Returns an error if:
/// - Actor is not authorized (Admin role required)
/// - Database query fails
///
/// # Panics
///
/// Panics if a persisted round or its round group does not have an ID.
#[allow(dead_code)]
pub fn list_rounds(
    persistence: &mut SqlitePersistence,
    round_group_id: i64,
    authenticated_actor: &AuthenticatedActor,
) -> Result<crate::request_response::ListRoundsResponse, ApiError> {
    // Enforce authorization - only admins can view rounds
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("list_rounds"),
            required_role: String::from("Admin"),
        });
    }

    let rounds = persistence
        .list_rounds(round_group_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to list rounds: {e}"),
        })?;

    let round_infos: Vec<crate::request_response::RoundInfo> = rounds
        .into_iter()
        .map(|r| {
            let round_id = r.round_id().ok_or_else(|| ApiError::Internal {
                message: String::from("persisted round missing ID"),
            })?;
            let round_group_id =
                r.round_group()
                    .round_group_id()
                    .ok_or_else(|| ApiError::Internal {
                        message: String::from("persisted round group missing ID"),
                    })?;
            Ok(crate::request_response::RoundInfo {
                round_id,
                round_group_id,
                name: r.name().to_string(),
                round_number: r.round_number(),
                slots_per_day: r.slots_per_day(),
                max_groups: r.max_groups(),
                max_total_hours: r.max_total_hours(),
                include_holidays: r.include_holidays(),
                allow_overbid: r.allow_overbid(),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    Ok(crate::request_response::ListRoundsResponse {
        round_group_id,
        rounds: round_infos,
    })
}

/// Updates an existing round.
///
/// Rounds are editable in `Draft` and `BootstrapComplete` states.
/// After canonicalization, round configuration becomes immutable (or requires override).
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The round update request
/// * `authenticated_actor` - The authenticated actor performing the operation
///
/// # Returns
///
/// * `Ok(UpdateRoundResponse)` on success
/// * `Err(ApiError)` on validation failure or lifecycle constraint violation
///
/// # Errors
///
/// Returns an error if:
/// - Actor is not authorized (Admin role required)
/// - Round does not exist
/// - Lifecycle state does not allow updates
/// - Round number already exists (duplicate)
/// - Validation fails
///
/// # Panics
///
/// Panics if the persisted round's round group does not have an ID or `bid_year_id`.
#[allow(dead_code)]
#[allow(clippy::too_many_lines)]
pub fn update_round(
    persistence: &mut SqlitePersistence,
    request: &crate::request_response::UpdateRoundRequest,
    authenticated_actor: &AuthenticatedActor,
) -> Result<crate::request_response::UpdateRoundResponse, ApiError> {
    use zab_bid_domain::BidYearLifecycle;

    // Enforce authorization - only admins can manage rounds
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("update_round"),
            required_role: String::from("Admin"),
        });
    }

    // Get the existing round to find its round_group_id and bid_year_id
    let existing_round = persistence
        .get_round(request.round_id)
        .map_err(|e| match e {
            PersistenceError::NotFound(_) => translate_domain_error(DomainError::RoundNotFound {
                round_id: request.round_id,
            }),
            _ => ApiError::Internal {
                message: format!("Failed to get round: {e}"),
            },
        })?;

    let round_group_id = existing_round
        .round_group()
        .round_group_id()
        .ok_or_else(|| ApiError::Internal {
            message: String::from("persisted round group missing ID"),
        })?;

    // Get bid_year_id from the round group
    let round_group =
        persistence
            .get_round_group(round_group_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get round group: {e}"),
            })?;

    let bid_year_id = round_group
        .bid_year()
        .bid_year_id()
        .ok_or_else(|| ApiError::Internal {
            message: String::from("persisted bid year missing ID"),
        })?;

    // Enforce lifecycle constraints
    let lifecycle_state_str: String =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    let lifecycle_state: BidYearLifecycle = lifecycle_state_str
        .parse()
        .map_err(translate_domain_error)?;

    if lifecycle_state.is_locked() {
        return Err(ApiError::DomainRuleViolation {
            rule: String::from("round_lifecycle"),
            message: format!(
                "Cannot update round in state '{lifecycle_state}': structural changes locked after confirmation"
            ),
        });
    }

    // Validate round configuration
    if request.slots_per_day == 0 {
        return Err(ApiError::InvalidInput {
            field: String::from("slots_per_day"),
            message: String::from("slots_per_day must be greater than 0"),
        });
    }
    if request.max_groups == 0 {
        return Err(ApiError::InvalidInput {
            field: String::from("max_groups"),
            message: String::from("max_groups must be greater than 0"),
        });
    }
    if request.max_total_hours == 0 {
        return Err(ApiError::InvalidInput {
            field: String::from("max_total_hours"),
            message: String::from("max_total_hours must be greater than 0"),
        });
    }
    if request.name.trim().is_empty() {
        return Err(ApiError::InvalidInput {
            field: String::from("name"),
            message: String::from("Round name cannot be empty"),
        });
    }

    // Check for duplicate round number within the round group (excluding this round)
    let round_number_exists = persistence
        .round_number_exists(round_group_id, request.round_number, Some(request.round_id))
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to check round number: {e}"),
        })?;

    if round_number_exists {
        return Err(translate_domain_error(DomainError::DuplicateRoundNumber {
            area_code: round_group.name().to_string(),
            round_number: request.round_number,
        }));
    }

    // Update the round
    persistence
        .update_round(
            request.round_id,
            &request.name,
            request.slots_per_day,
            request.max_groups,
            request.max_total_hours,
            request.include_holidays,
            request.allow_overbid,
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update round: {e}"),
        })?;

    Ok(crate::request_response::UpdateRoundResponse {
        round_id: request.round_id,
        round_group_id,
        round_number: request.round_number,
        name: request.name.clone(),
        message: format!("Updated round {} '{}'", request.round_number, request.name),
    })
}

/// Deletes a round.
///
/// Rounds can be deleted only in `Draft` and `BootstrapComplete` states.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `round_id` - The round ID to delete
/// * `authenticated_actor` - The authenticated actor performing the operation
///
/// # Returns
///
/// * `Ok(DeleteRoundResponse)` on success
/// * `Err(ApiError)` on validation failure or lifecycle constraint violation
///
/// # Errors
///
/// Returns an error if:
/// - Actor is not authorized (Admin role required)
/// - Round does not exist
/// - Lifecycle state does not allow deletion
///
/// # Panics
///
/// Panics if the persisted round's round group does not have an ID or `bid_year_id`.
#[allow(dead_code)]
pub fn delete_round(
    persistence: &mut SqlitePersistence,
    round_id: i64,
    authenticated_actor: &AuthenticatedActor,
) -> Result<crate::request_response::DeleteRoundResponse, ApiError> {
    use zab_bid_domain::BidYearLifecycle;

    // Enforce authorization - only admins can manage rounds
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("delete_round"),
            required_role: String::from("Admin"),
        });
    }

    // Get the existing round to find its bid_year_id
    let existing_round = persistence.get_round(round_id).map_err(|e| match e {
        PersistenceError::NotFound(_) => {
            translate_domain_error(DomainError::RoundNotFound { round_id })
        }
        _ => ApiError::Internal {
            message: format!("Failed to get round: {e}"),
        },
    })?;

    // Get bid_year_id from the round group
    let round_group_id = existing_round
        .round_group()
        .round_group_id()
        .ok_or_else(|| ApiError::Internal {
            message: String::from("persisted round group missing ID"),
        })?;
    let round_group =
        persistence
            .get_round_group(round_group_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get round group: {e}"),
            })?;

    let bid_year_id = round_group
        .bid_year()
        .bid_year_id()
        .ok_or_else(|| ApiError::Internal {
            message: String::from("persisted bid year missing ID"),
        })?;

    // Enforce lifecycle constraints
    let lifecycle_state_str: String =
        persistence
            .get_lifecycle_state(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to get lifecycle state: {e}"),
            })?;

    let lifecycle_state: BidYearLifecycle = lifecycle_state_str
        .parse()
        .map_err(translate_domain_error)?;

    if lifecycle_state.is_locked() {
        return Err(ApiError::DomainRuleViolation {
            rule: String::from("round_lifecycle"),
            message: format!(
                "Cannot delete round in state '{lifecycle_state}': structural changes locked after confirmation"
            ),
        });
    }

    // Delete the round
    persistence
        .delete_round(round_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to delete round: {e}"),
        })?;

    Ok(crate::request_response::DeleteRoundResponse {
        message: format!(
            "Deleted round {} '{}'",
            existing_round.round_number(),
            existing_round.name()
        ),
    })
}

/// Detects seniority conflicts by computing bid order for all non-system areas.
///
/// # Returns
///
/// A tuple of (`conflict_count`, `detailed_conflict_messages`).
fn detect_seniority_conflicts(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
) -> Result<(usize, Vec<String>), ApiError> {
    let users_by_area = persistence
        .get_users_by_area_for_conflict_detection(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get users for conflict detection: {e}"),
        })?;

    let mut seniority_conflicts: usize = 0;
    let mut conflict_areas: Vec<String> = Vec::new();

    for (area_id, area_code, users) in users_by_area {
        // Attempt to compute bid order for this area
        match zab_bid_domain::compute_bid_order(&users) {
            Ok(_) => {
                // No conflict in this area
            }
            Err(zab_bid_domain::DomainError::SeniorityConflict {
                user1_initials,
                user2_initials,
                reason,
            }) => {
                seniority_conflicts += 1;
                conflict_areas.push(format!(
                    "Area '{area_code}': seniority conflict between '{user1_initials}' and '{user2_initials}' ({reason})"
                ));
            }
            Err(e) => {
                return Err(ApiError::Internal {
                    message: format!(
                        "Unexpected error computing bid order for area '{area_code}' (ID {area_id}): {e}"
                    ),
                });
            }
        }
    }

    Ok((seniority_conflicts, conflict_areas))
}

/// Builds the list of blocking reasons for bid year readiness.
fn build_blocking_reasons(
    areas_missing_rounds: &[String],
    no_bid_users_pending_review: usize,
    participation_flag_violations: usize,
    seniority_conflicts: usize,
    conflict_details: &[String],
    bid_schedule_set: bool,
) -> Vec<String> {
    let mut blocking_reasons: Vec<String> = Vec::new();

    for area_code in areas_missing_rounds {
        blocking_reasons.push(format!("Area '{area_code}' has no rounds configured"));
    }

    if no_bid_users_pending_review > 0 {
        blocking_reasons.push(format!(
            "{no_bid_users_pending_review} users in No Bid area have not been reviewed"
        ));
    }

    if participation_flag_violations > 0 {
        blocking_reasons.push(format!(
            "{participation_flag_violations} users violate participation flag invariant"
        ));
    }

    if seniority_conflicts > 0 {
        blocking_reasons.push(format!(
            "{seniority_conflicts} seniority conflict(s) detected"
        ));
        // Add detailed conflict information
        for conflict_detail in conflict_details {
            blocking_reasons.push(conflict_detail.clone());
        }
    }

    if !bid_schedule_set {
        blocking_reasons.push(String::from("Bid schedule is not set"));
    }

    blocking_reasons
}

/// Gets the readiness status for a bid year.
///
/// Evaluates all readiness criteria and returns a structured response
/// indicating whether the bid year is ready for confirmation.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - Bootstrap metadata
/// * `bid_year_id` - The canonical bid year ID
///
/// # Returns
///
/// A structured readiness evaluation response.
///
/// # Errors
///
/// Returns an error if:
/// - The bid year does not exist
/// - Database queries fail
/// - Seniority conflict detection fails
#[allow(dead_code)] // Phase 29D: Will be used when wired up in server
pub fn get_bid_year_readiness(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    bid_year_id: i64,
) -> Result<GetBidYearReadinessResponse, ApiError> {
    // Get the bid year to validate it exists and get the year value
    let bid_year_value: u16 = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {bid_year_id} not found"),
        })?
        .year();

    // Query readiness criteria from persistence
    let areas_missing_rounds: Vec<String> = persistence
        .get_areas_missing_rounds(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get areas missing rounds: {e}"),
        })?;

    let no_bid_users_pending_review: i64 = persistence
        .count_unreviewed_no_bid_users(bid_year_id)
        .map_err(|e| ApiError::Internal {
        message: format!("Failed to count unreviewed No Bid users: {e}"),
    })?;

    let participation_flag_violations: i64 = persistence
        .count_participation_flag_violations(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to count participation flag violations: {e}"),
        })?;

    // Convert database counts to usize for response
    // Database counts are always non-negative, so we can safely cast
    let no_bid_users_pending_review_usize: usize =
        no_bid_users_pending_review.to_usize().ok_or_else(|| {
            ApiError::Internal {
                message: format!(
                    "Failed to convert no_bid_users_pending_review count {no_bid_users_pending_review} to usize"
                ),
            }
        })?;

    let participation_flag_violations_usize: usize =
        participation_flag_violations.to_usize().ok_or_else(|| {
            ApiError::Internal {
                message: format!(
                    "Failed to convert participation_flag_violations count {participation_flag_violations} to usize"
                ),
            }
        })?;

    let bid_schedule_set: bool =
        persistence
            .is_bid_schedule_set(bid_year_id)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to check bid schedule status: {e}"),
            })?;

    // Detect seniority conflicts
    let (seniority_conflicts, conflict_areas) =
        detect_seniority_conflicts(persistence, bid_year_id)?;

    // Build blocking reasons
    let blocking_reasons = build_blocking_reasons(
        &areas_missing_rounds,
        no_bid_users_pending_review_usize,
        participation_flag_violations_usize,
        seniority_conflicts,
        &conflict_areas,
        bid_schedule_set,
    );

    let is_ready: bool = blocking_reasons.is_empty();

    Ok(GetBidYearReadinessResponse {
        bid_year_id,
        year: bid_year_value,
        is_ready,
        blocking_reasons,
        details: ReadinessDetailsInfo {
            areas_missing_rounds,
            no_bid_users_pending_review: no_bid_users_pending_review_usize,
            participation_flag_violations: participation_flag_violations_usize,
            seniority_conflicts,
            bid_schedule_set,
        },
    })
}

/// Confirms a bid year is ready to bid, materializing bid order and calculating bid windows.
///
/// This is the irreversible confirmation action that:
/// - Validates readiness preconditions
/// - Computes and stores bid order for all eligible users
/// - Calculates and stores bid windows based on bid schedule
/// - Transitions lifecycle state from `BootstrapComplete` to `Canonicalized`
/// - Engages editing locks
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - The bootstrap metadata
/// * `request` - The confirmation request
/// * `authenticated_actor` - The authenticated actor performing the action
/// * `operator` - The operator data
/// * `cause` - The audit cause
///
/// # Returns
///
/// A response containing confirmation details and statistics.
///
/// # Errors
///
/// Returns an error if:
/// - Authorization fails (only admins can confirm)
/// - Confirmation text doesn't match
/// - Bid year doesn't exist
/// - Current state is not `BootstrapComplete`
/// - Readiness evaluation fails
/// - Bid schedule is missing or invalid
/// - Bid order computation fails
/// - Bid window calculation fails
/// - Database operations fail
#[allow(clippy::too_many_lines)]
#[allow(dead_code)] // Phase 29E: Will be wired up in server layer
pub fn confirm_ready_to_bid(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    request: &ConfirmReadyToBidRequest,
    authenticated_actor: &AuthenticatedActor,
    operator: &OperatorData,
    cause: Cause,
) -> Result<ConfirmReadyToBidResponse, ApiError> {
    const REQUIRED_CONFIRMATION: &str = "I understand this action is irreversible";

    // Enforce authorization - only admins can confirm
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("confirm_ready_to_bid"),
            required_role: String::from("Admin"),
        });
    }

    // Validate confirmation text
    if request.confirmation != REQUIRED_CONFIRMATION {
        return Err(ApiError::InvalidInput {
            field: String::from("confirmation"),
            message: format!("Confirmation text must be exactly: '{REQUIRED_CONFIRMATION}'"),
        });
    }

    // Resolve bid_year_id to BidYear from metadata
    let bid_year: &BidYear = metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(request.bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {} not found", request.bid_year_id),
        })?;

    let year: u16 = bid_year.year();

    // Check readiness first
    let readiness: GetBidYearReadinessResponse =
        get_bid_year_readiness(persistence, metadata, request.bid_year_id)?;

    if !readiness.is_ready {
        return Err(ApiError::DomainRuleViolation {
            rule: String::from("Readiness criteria must be satisfied"),
            message: format!(
                "Bid year {} is not ready for confirmation. Blocking reasons: {}",
                year,
                readiness.blocking_reasons.join(", ")
            ),
        });
    }

    // Load current lifecycle state
    let current_state_str: String = persistence
        .get_lifecycle_state(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get lifecycle state: {e}"),
        })?;

    let current_state: zab_bid_domain::BidYearLifecycle =
        current_state_str.parse().map_err(translate_domain_error)?;

    // If in Draft state and ready, auto-transition to BootstrapComplete first
    if current_state == zab_bid_domain::BidYearLifecycle::Draft {
        let bootstrap_req = TransitionToBootstrapCompleteRequest {
            bid_year_id: request.bid_year_id,
        };

        transition_to_bootstrap_complete(
            persistence,
            metadata,
            &bootstrap_req,
            authenticated_actor,
            operator,
            cause.clone(),
        )?;
    }

    // Reload lifecycle state after potential auto-transition
    let current_state_str: String = persistence
        .get_lifecycle_state(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get lifecycle state: {e}"),
        })?;

    let current_state: zab_bid_domain::BidYearLifecycle =
        current_state_str.parse().map_err(translate_domain_error)?;

    // Validate we're now in BootstrapComplete
    if current_state != zab_bid_domain::BidYearLifecycle::BootstrapComplete {
        return Err(ApiError::DomainRuleViolation {
            rule: String::from("ConfirmReadyToBid requires BootstrapComplete state"),
            message: format!(
                "Cannot confirm bid year in state '{}' (must be 'BootstrapComplete')",
                current_state.as_str()
            ),
        });
    }

    // Get bid schedule
    let bid_schedule_result = persistence
        .get_bid_schedule(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get bid schedule: {e}"),
        })?;

    let bid_schedule: zab_bid_domain::BidSchedule = match bid_schedule_result {
        (
            Some(timezone),
            Some(start_date_str),
            Some(window_start_time_str),
            Some(window_end_time_str),
            Some(bidders_per_day),
        ) => {
            // Parse date and times from strings
            let start_date = time::Date::parse(
                &start_date_str,
                &time::format_description::well_known::Iso8601::DEFAULT,
            )
            .map_err(|_| ApiError::Internal {
                message: format!("Failed to parse bid start date: {start_date_str}"),
            })?;

            let window_start_time = time::Time::parse(
                &window_start_time_str,
                &time::format_description::well_known::Iso8601::DEFAULT,
            )
            .map_err(|_| ApiError::Internal {
                message: format!("Failed to parse window start time: {window_start_time_str}"),
            })?;

            let window_end_time = time::Time::parse(
                &window_end_time_str,
                &time::format_description::well_known::Iso8601::DEFAULT,
            )
            .map_err(|_| ApiError::Internal {
                message: format!("Failed to parse window end time: {window_end_time_str}"),
            })?;

            let bidders_per_day_u32 =
                bidders_per_day.to_u32().ok_or_else(|| ApiError::Internal {
                    message: format!("Invalid bidders_per_day value: {bidders_per_day}"),
                })?;

            zab_bid_domain::BidSchedule::new(
                timezone,
                start_date,
                window_start_time,
                window_end_time,
                bidders_per_day_u32,
            )
            .map_err(translate_domain_error)?
        }
        _ => {
            return Err(ApiError::DomainRuleViolation {
                rule: String::from("Bid schedule must be set before confirmation"),
                message: format!("No bid schedule configured for bid year {year}"),
            });
        }
    };

    // Get all users grouped by area for this bid year
    let users_by_area = persistence
        .get_users_by_area_for_conflict_detection(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get users for bid year {year}: {e}"),
        })?;

    // Get all rounds for this bid year to calculate windows per-round
    let all_rounds = persistence
        .list_all_rounds_for_bid_year(request.bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get rounds for bid year {year}: {e}"),
        })?;

    let round_ids: Vec<i64> = all_rounds.iter().map(|(id, _name)| *id).collect();

    // Materialize bid order and calculate windows for each area
    let mut total_bid_order_count: usize = 0;
    let mut total_bid_windows_count: usize = 0;

    for (_area_id, _area_code, users_in_area) in &users_by_area {
        if users_in_area.is_empty() {
            continue;
        }

        // Compute bid order
        let bid_order_positions: Vec<zab_bid_domain::BidOrderPosition> =
            zab_bid_domain::compute_bid_order(users_in_area).map_err(translate_domain_error)?;

        if bid_order_positions.is_empty() {
            continue;
        }

        // Calculate bid windows (per-round)
        let user_positions: Vec<(i64, usize)> = bid_order_positions
            .iter()
            .map(|pos| (pos.user_id, pos.position))
            .collect();

        let bid_windows: Vec<zab_bid_domain::BidWindow> =
            zab_bid_domain::calculate_bid_windows(&user_positions, &round_ids, &bid_schedule)
                .map_err(translate_domain_error)?;

        total_bid_order_count += bid_order_positions.len();
        total_bid_windows_count += bid_windows.len();
    }

    // Apply the core command
    let command = Command::ConfirmReadyToBid { year };
    let actor: Actor = authenticated_actor.to_audit_actor(operator);
    let result: BootstrapResult =
        apply_bootstrap(metadata, bid_year, command, actor, cause).map_err(translate_core_error)?;

    // Persist audit event first to get the audit_event_id
    let audit_event_id: i64 = persistence
        .persist_audit_event(&result.audit_event)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to persist audit event: {e}"),
        })?;

    // Now materialize bid order and windows with the audit_event_id
    for (area_id, _area_code, users_in_area) in &users_by_area {
        if users_in_area.is_empty() {
            continue;
        }

        // Compute bid order again (deterministic, so same result)
        let bid_order_positions: Vec<zab_bid_domain::BidOrderPosition> =
            zab_bid_domain::compute_bid_order(users_in_area).map_err(translate_domain_error)?;

        if bid_order_positions.is_empty() {
            continue;
        }

        // Convert to persistence records
        let bid_order_records: Vec<zab_bid_persistence::data_models::NewCanonicalBidOrder> =
            bid_order_positions
                .iter()
                .map(
                    |pos| zab_bid_persistence::data_models::NewCanonicalBidOrder {
                        bid_year_id: request.bid_year_id,
                        audit_event_id,
                        user_id: pos.user_id,
                        bid_order: pos.position.to_i32(),
                        is_overridden: 0,
                        override_reason: None,
                    },
                )
                .collect();

        // Persist bid order
        persistence
            .bulk_insert_canonical_bid_order(&bid_order_records)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to persist bid order: {e}"),
            })?;

        // Calculate bid windows (per-round)
        let user_positions: Vec<(i64, usize)> = bid_order_positions
            .iter()
            .map(|pos| (pos.user_id, pos.position))
            .collect();

        let bid_windows: Vec<zab_bid_domain::BidWindow> =
            zab_bid_domain::calculate_bid_windows(&user_positions, &round_ids, &bid_schedule)
                .map_err(translate_domain_error)?;

        // Convert to persistence records
        let bid_window_records: Vec<zab_bid_persistence::data_models::NewBidWindow> = bid_windows
            .iter()
            .map(|window| zab_bid_persistence::data_models::NewBidWindow {
                bid_year_id: request.bid_year_id,
                area_id: *area_id,
                user_id: window.user_id,
                round_id: window.round_id,
                window_start_datetime: window.window_start_datetime.clone(),
                window_end_datetime: window.window_end_datetime.clone(),
            })
            .collect();

        // Persist bid windows
        persistence
            .bulk_insert_bid_windows(&bid_window_records)
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to persist bid windows: {e}"),
            })?;
    }

    // Update lifecycle state to Canonicalized
    let target_state = zab_bid_domain::BidYearLifecycle::Canonicalized;
    persistence
        .update_lifecycle_state(request.bid_year_id, target_state.as_str())
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update lifecycle state: {e}"),
        })?;

    // Initialize bid status tracking for all users in all rounds
    if !all_rounds.is_empty() {
        // Create initial bid status records for all user/round combinations
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ApiError::Internal {
                message: format!("System time error: {e}"),
            })?
            .as_secs();
        let current_timestamp =
            time::OffsetDateTime::from_unix_timestamp(now.to_i64().ok_or_else(|| {
                ApiError::Internal {
                    message: String::from("Timestamp conversion failed"),
                }
            })?)
            .map_err(|e| ApiError::Internal {
                message: format!("Invalid timestamp: {e}"),
            })?
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| ApiError::Internal {
                message: format!("Timestamp formatting failed: {e}"),
            })?;
        let mut bid_status_records = Vec::new();

        for (area_id, _area_code, users_in_area) in &users_by_area {
            for user in users_in_area {
                // All users should have IDs at confirmation time
                let user_id = user.user_id.ok_or_else(|| ApiError::Internal {
                    message: format!("User with initials {:?} has no user_id", user.initials),
                })?;

                for (round_id, _round_name) in &all_rounds {
                    bid_status_records.push(zab_bid_persistence::data_models::NewBidStatus {
                        bid_year_id: request.bid_year_id,
                        area_id: *area_id,
                        user_id,
                        round_id: *round_id,
                        status: String::from("NotStartedPreWindow"),
                        updated_at: current_timestamp.clone(),
                        updated_by: operator.operator_id,
                        notes: Some(String::from("Initial status at confirmation")),
                    });
                }
            }
        }

        // Bulk insert all bid status records
        if !bid_status_records.is_empty() {
            persistence
                .bulk_insert_bid_status(&bid_status_records)
                .map_err(|e| ApiError::Internal {
                    message: format!("Failed to initialize bid status tracking: {e}"),
                })?;
        }
    }

    Ok(ConfirmReadyToBidResponse {
        bid_year_id: request.bid_year_id,
        year,
        lifecycle_state: target_state.as_str().to_string(),
        audit_event_id,
        message: format!("Bid year {year} confirmed ready to bid"),
        bid_order_count: total_bid_order_count,
        bid_windows_calculated: total_bid_windows_count,
    })
}

/// Marks a user in a system area as reviewed.
///
/// This endpoint is used to confirm that a user assigned to a system area
/// (e.g., "No Bid") has been reviewed and their assignment is correct.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `user_id` - The user's canonical ID
/// * `authenticated_actor` - The authenticated actor performing the action
///
/// # Returns
///
/// A success response confirming the review.
///
/// # Errors
///
/// Returns an error if:
/// - The user does not exist
/// - The user is not in a system area
/// - Authorization fails
/// - Database update fails
#[allow(dead_code)] // Phase 29D: Will be used when wired up in server
pub fn review_no_bid_user(
    persistence: &mut SqlitePersistence,
    user_id: i64,
    authenticated_actor: &AuthenticatedActor,
) -> Result<ReviewNoBidUserResponse, ApiError> {
    // Enforce authorization - only admins can review No Bid users
    if authenticated_actor.role != Role::Admin {
        return Err(ApiError::Unauthorized {
            action: String::from("review_no_bid_user"),
            required_role: String::from("Admin"),
        });
    }

    // Mark the user as reviewed
    persistence
        .mark_user_no_bid_reviewed(user_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to mark user as reviewed: {e}"),
        })?;

    Ok(ReviewNoBidUserResponse {
        user_id,
        message: format!("User {user_id} marked as reviewed"),
    })
}

/// Gets a preview of the derived bid order for an area.
///
/// This is a read-only preview endpoint that shows what the bid order will be
/// when frozen at confirmation. No persistence or audit events are generated.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `metadata` - Bootstrap metadata
/// * `bid_year_id` - The canonical bid year ID
/// * `area_id` - The canonical area ID
///
/// # Returns
///
/// An ordered list of users with their positions and seniority inputs.
///
/// # Errors
///
/// Returns an error if:
/// - The bid year does not exist
/// - The area does not exist
/// - Database queries fail
/// - Seniority conflicts are detected (unresolved ties)
/// - Area is a system area (no bid order applies)
#[allow(dead_code)] // Phase 29D: Will be used when wired up in server
pub fn get_bid_order_preview(
    persistence: &mut SqlitePersistence,
    metadata: &BootstrapMetadata,
    bid_year_id: i64,
    area_id: i64,
) -> Result<GetBidOrderPreviewResponse, ApiError> {
    // Validate bid year exists
    metadata
        .bid_years
        .iter()
        .find(|by| by.bid_year_id() == Some(bid_year_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("BidYear"),
            message: format!("Bid year with ID {bid_year_id} not found"),
        })?;

    // Validate area exists and get area code
    let (_, area) = metadata
        .areas
        .iter()
        .find(|(_by, a)| a.area_id() == Some(area_id))
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("Area"),
            message: format!("Area with ID {area_id} not found in bid year {bid_year_id}"),
        })?;

    let area_code = area.area_code().to_string();

    // Reject system areas
    if area.is_system_area() {
        return Err(ApiError::InvalidInput {
            field: String::from("area_id"),
            message: format!("Cannot compute bid order for system area '{area_code}'"),
        });
    }

    // Get all users grouped by area for this bid year
    let users_by_area = persistence
        .get_users_by_area_for_conflict_detection(bid_year_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get users for bid year {bid_year_id}: {e}"),
        })?;

    // Find the specific area we're interested in
    let users = users_by_area
        .into_iter()
        .find(|(aid, _, _)| *aid == area_id)
        .map(|(_, _, users)| users)
        .ok_or_else(|| ApiError::ResourceNotFound {
            resource_type: String::from("Area"),
            message: format!("No users found for area {area_id}"),
        })?;

    // Compute bid order
    let bid_order = zab_bid_domain::compute_bid_order(&users).map_err(|e| match e {
        zab_bid_domain::DomainError::SeniorityConflict {
            user1_initials,
            user2_initials,
            reason,
        } => ApiError::DomainRuleViolation {
            rule: String::from("seniority_total_ordering"),
            message: format!(
                "Seniority conflict in area '{area_code}': '{user1_initials}' and '{user2_initials}' ({reason})"
            ),
        },
        _ => ApiError::Internal {
            message: format!("Failed to compute bid order for area '{area_code}': {e}"),
        },
    })?;

    // Convert to API response type
    let positions: Vec<BidOrderPositionInfo> = bid_order
        .into_iter()
        .map(|pos| BidOrderPositionInfo {
            position: pos.position,
            user_id: pos.user_id,
            initials: pos.initials,
            seniority_inputs: SeniorityInputsInfo {
                cumulative_natca_bu_date: pos.seniority_inputs.cumulative_natca_bu_date,
                natca_bu_date: pos.seniority_inputs.natca_bu_date,
                eod_faa_date: pos.seniority_inputs.eod_faa_date,
                service_computation_date: pos.seniority_inputs.service_computation_date,
                lottery_value: pos.seniority_inputs.lottery_value,
            },
        })
        .collect();

    Ok(GetBidOrderPreviewResponse {
        bid_year_id,
        area_id,
        area_code,
        positions,
    })
}

// ========================================================================
// Phase 29F: Bid Status Tracking Handlers
// ========================================================================

/// Get bid status for all users in an area across all rounds.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `actor` - The authenticated actor
/// * `bid_year_id` - The canonical bid year identifier
/// * `area_id` - The canonical area identifier
///
/// # Returns
///
/// * `Ok(GetBidStatusForAreaResponse)` - The bid status records
/// * `Err(ApiError)` - If the query fails
///
/// # Errors
///
/// Returns an error if:
/// - The bid year does not exist
/// - The area does not exist
/// - The database query fails
pub fn get_bid_status_for_area(
    persistence: &mut SqlitePersistence,
    request: &GetBidStatusForAreaRequest,
    _actor: &AuthenticatedActor,
) -> Result<GetBidStatusForAreaResponse, ApiError> {
    get_bid_status_for_area_impl(persistence, request.bid_year_id, request.area_id)
}

#[allow(dead_code)]
fn get_bid_status_for_area_impl(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
    area_id: i64,
) -> Result<GetBidStatusForAreaResponse, ApiError> {
    // Get area code for display (validates area exists)
    let area = persistence
        .get_area_by_id(area_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get area: {e}"),
        })?;

    // Query bid status records
    let status_rows = persistence
        .get_bid_status_for_area(bid_year_id, area_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get bid status for area: {e}"),
        })?;

    // Convert to API response type
    let statuses: Vec<BidStatusInfo> = status_rows
        .into_iter()
        .map(|row| {
            // Get user initials
            let user = persistence
                .get_user_by_id(row.user_id)
                .ok()
                .map_or_else(|| String::from("Unknown"), |u| u.initials);

            // Get round name
            let round = persistence
                .get_round_by_id(row.round_id)
                .ok()
                .map_or_else(|| String::from("Unknown"), |r| r.round_name);

            // Get operator display name
            let operator = persistence
                .get_operator_by_id(row.updated_by)
                .ok()
                .flatten()
                .map_or_else(|| String::from("Unknown"), |op| op.display_name);

            BidStatusInfo {
                bid_status_id: row.bid_status_id,
                user_id: row.user_id,
                initials: user,
                round_id: row.round_id,
                round_name: round,
                status: row.status,
                updated_at: row.updated_at,
                updated_by: operator,
                notes: row.notes,
            }
        })
        .collect();

    Ok(GetBidStatusForAreaResponse {
        bid_year_id,
        area_id,
        area_code: area.0.area_code().to_string(),
        statuses,
    })
}

/// Get bid status for a specific user and round.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `actor` - The authenticated actor
/// * `request` - The request containing `user_id` and `round_id`
///
/// # Returns
///
/// * `Ok(GetBidStatusResponse)` - The bid status and history
/// * `Err(ApiError)` - If the query fails
///
/// # Errors
///
/// Returns an error if:
/// - The bid status record does not exist
/// - The database query fails
pub fn get_bid_status(
    persistence: &mut SqlitePersistence,
    request: &GetBidStatusRequest,
    _actor: &AuthenticatedActor,
) -> Result<GetBidStatusResponse, ApiError> {
    get_bid_status_impl(
        persistence,
        request.bid_year_id,
        request.area_id,
        request.user_id,
        request.round_id,
    )
}

#[allow(dead_code)]
fn get_bid_status_impl(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
    area_id: i64,
    user_id: i64,
    round_id: i64,
) -> Result<GetBidStatusResponse, ApiError> {
    // Query bid status record (validates bid year, area exist)
    let status_row = persistence
        .get_bid_status_for_user_and_round(bid_year_id, area_id, user_id, round_id)
        .map_err(|e| match e {
            PersistenceError::NotFound(_) => ApiError::ResourceNotFound {
                resource_type: String::from("bid_status"),
                message: format!("Bid status not found for user_id={user_id}, round_id={round_id}"),
            },
            _ => ApiError::Internal {
                message: format!("Failed to get bid status: {e}"),
            },
        })?;

    // Get user initials
    let user = persistence
        .get_user_by_id(user_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get user: {e}"),
        })?;

    // Get round name
    let round = persistence
        .get_round_by_id(round_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get round: {e}"),
        })?;

    // Get operator display name
    let operator = persistence
        .get_operator_by_id(status_row.updated_by)
        .ok()
        .flatten()
        .map_or_else(|| String::from("Unknown"), |op| op.display_name);

    let status = BidStatusInfo {
        bid_status_id: status_row.bid_status_id,
        user_id: status_row.user_id,
        initials: user.initials,
        round_id: status_row.round_id,
        round_name: round.round_name,
        status: status_row.status,
        updated_at: status_row.updated_at,
        updated_by: operator,
        notes: status_row.notes,
    };

    // Query status history
    let history_rows = persistence
        .get_bid_status_history(status_row.bid_status_id)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to get bid status history: {e}"),
        })?;

    let history: Vec<BidStatusHistoryInfo> = history_rows
        .into_iter()
        .map(|row| {
            let operator = persistence
                .get_operator_by_id(row.transitioned_by)
                .ok()
                .flatten()
                .map_or_else(|| String::from("Unknown"), |op| op.display_name);

            BidStatusHistoryInfo {
                history_id: row.history_id,
                previous_status: row.previous_status,
                new_status: row.new_status,
                transitioned_at: row.transitioned_at,
                transitioned_by: operator,
                notes: row.notes,
            }
        })
        .collect();

    Ok(GetBidStatusResponse { status, history })
}

/// Transition a bid status to a new state.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The request containing `bid_status_id`, `new_status`, and notes
/// * `actor` - The authenticated actor (must be Admin or Bidder)
/// * `operator` - The operator performing the action
///
/// # Returns
///
/// * `Ok(TransitionBidStatusResponse)` - Details of the transition
/// * `Err(ApiError)` - If the transition fails
///
/// # Errors
///
/// Returns an error if:
/// - Authorization fails (must be Admin or Bidder)
/// - Notes are too short (< 10 characters)
/// - The bid status record does not exist
/// - The status transition is invalid
/// - The database operation fails
pub fn transition_bid_status(
    persistence: &mut SqlitePersistence,
    request: &TransitionBidStatusRequest,
    actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<TransitionBidStatusResponse, ApiError> {
    transition_bid_status_impl(
        persistence,
        actor,
        operator,
        request.bid_status_id,
        &request.new_status,
        &request.notes,
    )
}

#[allow(dead_code)]
fn transition_bid_status_impl(
    persistence: &mut SqlitePersistence,
    actor: &AuthenticatedActor,
    _operator: &OperatorData,
    bid_status_id: i64,
    new_status_str: &str,
    notes: &str,
) -> Result<TransitionBidStatusResponse, ApiError> {
    // Authorization: Admin or Bidder required
    if !matches!(actor.role, Role::Admin | Role::Bidder) {
        return Err(ApiError::Unauthorized {
            action: String::from("transition_bid_status"),
            required_role: String::from("Admin or Bidder"),
        });
    }

    // Validate notes length
    if notes.len() < 10 {
        return Err(ApiError::InvalidInput {
            field: String::from("notes"),
            message: String::from("Notes must be at least 10 characters"),
        });
    }

    // Get current bid status record
    let current_row = persistence
        .get_bid_status_by_id(bid_status_id)
        .map_err(|e| match e {
            PersistenceError::NotFound(_) => ApiError::ResourceNotFound {
                resource_type: String::from("bid_status"),
                message: format!("Bid status {bid_status_id} not found"),
            },
            _ => ApiError::Internal {
                message: format!("Failed to get bid status: {e}"),
            },
        })?;

    // Parse current and new status
    let current_status =
        zab_bid_domain::BidStatus::from_str(&current_row.status).map_err(translate_domain_error)?;
    let new_status =
        zab_bid_domain::BidStatus::from_str(new_status_str).map_err(translate_domain_error)?;

    // Validate transition
    current_status
        .validate_transition(new_status)
        .map_err(translate_domain_error)?;

    // Get current timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| ApiError::Internal {
            message: format!("System time error: {e}"),
        })?
        .as_secs();
    let transitioned_at =
        time::OffsetDateTime::from_unix_timestamp(now.to_i64().ok_or_else(|| {
            ApiError::Internal {
                message: String::from("Timestamp conversion failed"),
            }
        })?)
        .map_err(|e| ApiError::Internal {
            message: format!("Invalid timestamp: {e}"),
        })?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to format timestamp: {e}"),
        })?;

    // Update bid status
    // Parse operator_id from actor.id (string) to i64
    let operator_id = actor.id.parse::<i64>().map_err(|_| ApiError::Internal {
        message: String::from("Invalid operator ID format"),
    })?;

    persistence
        .update_bid_status(
            bid_status_id,
            new_status_str,
            &transitioned_at,
            operator_id,
            Some(notes),
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to update bid status: {e}"),
        })?;

    // Record transition in history
    // Get the next audit event ID (this is a simplification - in a real implementation
    // we would create an actual audit event)
    let audit_event_id = persistence.get_next_audit_event_id().unwrap_or(1);

    persistence
        .insert_bid_status_history(
            bid_status_id,
            audit_event_id,
            Some(&current_row.status),
            new_status_str,
            &transitioned_at,
            operator_id,
            Some(notes),
        )
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to insert bid status history: {e}"),
        })?;

    Ok(TransitionBidStatusResponse {
        bid_status_id,
        user_id: current_row.user_id,
        round_id: current_row.round_id,
        previous_status: current_row.status.clone(),
        new_status: new_status_str.to_string(),
        transitioned_at,
        message: format!(
            "Bid status transitioned from '{}' to '{new_status_str}'",
            current_row.status
        ),
    })
}

/// Bulk update bid status for multiple users in a round.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `request` - The request containing `bid_year_id`, `area_id`, `user_ids`, `round_id`, `new_status`, and notes
/// * `actor` - The authenticated actor (must be Admin or Bidder)
/// * `operator` - The operator performing the action
///
/// # Returns
///
/// * `Ok(BulkUpdateBidStatusResponse)` - Details of the bulk update
/// * `Err(ApiError)` - If the update fails
///
/// # Errors
///
/// Returns an error if:
/// - Authorization fails (must be Admin or Bidder)
/// - Notes are too short (< 10 characters)
/// - Any bid status record does not exist
/// - Any status transition is invalid
/// - The database operation fails
pub fn bulk_update_bid_status(
    persistence: &mut SqlitePersistence,
    request: &BulkUpdateBidStatusRequest,
    actor: &AuthenticatedActor,
    operator: &OperatorData,
) -> Result<BulkUpdateBidStatusResponse, ApiError> {
    bulk_update_bid_status_impl(
        persistence,
        actor,
        operator,
        request.bid_year_id,
        request.area_id,
        &request.user_ids,
        request.round_id,
        &request.new_status,
        &request.notes,
    )
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
fn bulk_update_bid_status_impl(
    persistence: &mut SqlitePersistence,
    actor: &AuthenticatedActor,
    _operator: &OperatorData,
    bid_year_id: i64,
    area_id: i64,
    user_ids: &[i64],
    round_id: i64,
    new_status_str: &str,
    notes: &str,
) -> Result<BulkUpdateBidStatusResponse, ApiError> {
    // Authorization: Admin or Bidder required
    if !matches!(actor.role, Role::Admin | Role::Bidder) {
        return Err(ApiError::Unauthorized {
            action: String::from("bulk_update_bid_status"),
            required_role: String::from("Admin or Bidder"),
        });
    }

    // Validate notes length
    if notes.len() < 10 {
        return Err(ApiError::InvalidInput {
            field: String::from("notes"),
            message: String::from("Notes must be at least 10 characters"),
        });
    }

    // Parse new status
    let new_status =
        zab_bid_domain::BidStatus::from_str(new_status_str).map_err(translate_domain_error)?;

    // Get current timestamp
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| ApiError::Internal {
            message: format!("System time error: {e}"),
        })?
        .as_secs();
    let transitioned_at =
        time::OffsetDateTime::from_unix_timestamp(now.to_i64().ok_or_else(|| {
            ApiError::Internal {
                message: String::from("Timestamp conversion failed"),
            }
        })?)
        .map_err(|e| ApiError::Internal {
            message: format!("Invalid timestamp: {e}"),
        })?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| ApiError::Internal {
            message: format!("Failed to format timestamp: {e}"),
        })?;

    // Validate all transitions before updating
    let mut status_records = Vec::new();
    for &user_id in user_ids {
        let status_row = persistence
            .get_bid_status_for_user_and_round(bid_year_id, area_id, user_id, round_id)
            .map_err(|e| match e {
                PersistenceError::NotFound(_) => ApiError::ResourceNotFound {
                    resource_type: String::from("bid_status"),
                    message: format!(
                        "Bid status not found for user_id={user_id}, round_id={round_id}"
                    ),
                },
                _ => ApiError::Internal {
                    message: format!("Failed to get bid status: {e}"),
                },
            })?;

        let current_status = zab_bid_domain::BidStatus::from_str(&status_row.status)
            .map_err(translate_domain_error)?;

        // Validate transition
        current_status
            .validate_transition(new_status)
            .map_err(translate_domain_error)?;

        status_records.push(status_row);
    }

    // Parse operator_id from actor.id (string) to i64
    let operator_id = actor.id.parse::<i64>().map_err(|_| ApiError::Internal {
        message: String::from("Invalid operator ID format"),
    })?;

    // All validations passed - perform updates
    let mut updated_count = 0;
    for status_row in status_records {
        // Update bid status
        persistence
            .update_bid_status(
                status_row.bid_status_id,
                new_status_str,
                &transitioned_at,
                operator_id,
                Some(notes),
            )
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to update bid status: {e}"),
            })?;

        // Record transition in history
        let audit_event_id = persistence.get_next_audit_event_id().unwrap_or(1);

        persistence
            .insert_bid_status_history(
                status_row.bid_status_id,
                audit_event_id,
                Some(&status_row.status),
                new_status_str,
                &transitioned_at,
                operator_id,
                Some(notes),
            )
            .map_err(|e| ApiError::Internal {
                message: format!("Failed to insert bid status history: {e}"),
            })?;

        updated_count += 1;
    }

    Ok(BulkUpdateBidStatusResponse {
        updated_count,
        new_status: new_status_str.to_string(),
        message: format!(
            "Successfully updated {updated_count} bid status records to '{new_status_str}'"
        ),
    })
}
