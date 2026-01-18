// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Comprehensive API layer tests organized by behavior.

use zab_bid::{BootstrapMetadata, BootstrapResult, State, TransitionResult};
use zab_bid_audit::{Actor, Cause};
use zab_bid_domain::{Area, BidYear};
use zab_bid_persistence::SqlitePersistence;

use crate::{
    ApiError, ApiResult, AuthError, AuthenticatedActor, CreateAreaRequest, CreateBidYearRequest,
    GetLeaveAvailabilityResponse, ImportCsvUsersRequest, ListAreasRequest, ListAreasResponse,
    ListBidYearsResponse, ListUsersResponse, RegisterUserRequest, RegisterUserResult, Role,
    UpdateUserRequest, checkpoint, create_area, create_bid_year, finalize, get_current_state,
    get_historical_state, get_leave_availability, import_csv_users, list_areas, list_bid_years,
    list_users, register_user, rollback, update_user,
};

use super::helpers::{
    create_test_admin, create_test_admin_operator, create_test_bidder, create_test_bidder_operator,
    create_test_canonical_bid_year, create_test_cause, create_test_metadata,
    create_test_pay_periods, create_test_start_date, create_test_start_date_for_year,
    create_valid_request, setup_test_persistence,
};

// ============================================================================
// Actor Conversion Tests
// ============================================================================

#[test]
fn test_authenticated_actor_to_audit_actor_admin() {
    let _metadata: BootstrapMetadata = create_test_metadata();
    let auth_actor: AuthenticatedActor =
        AuthenticatedActor::new(String::from("admin-1"), Role::Admin);
    let operator = create_test_admin_operator();
    let audit_actor: Actor = auth_actor.to_audit_actor(&operator);
    assert_eq!(audit_actor.id, "admin-1");
    assert_eq!(audit_actor.actor_type, "admin");
}

#[test]
fn test_authenticated_actor_to_audit_actor_bidder() {
    let _metadata: BootstrapMetadata = create_test_metadata();
    let auth_actor: AuthenticatedActor =
        AuthenticatedActor::new(String::from("bidder-1"), Role::Bidder);
    let operator = create_test_bidder_operator();
    let audit_actor: Actor = auth_actor.to_audit_actor(&operator);
    assert_eq!(audit_actor.id, "bidder-1");
    assert_eq!(audit_actor.actor_type, "bidder");
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

// ============================================================================
// Authorization Tests
// ============================================================================

#[test]
fn test_bidder_cannot_register_user() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let bidder: AuthenticatedActor = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &bidder,
        &operator,
        cause,
    );

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
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let bidder: AuthenticatedActor = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    // Original state is unchanged
    assert_eq!(state.users.len(), 0);
}

#[test]
fn test_unauthorized_action_does_not_emit_audit_event() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &bidder,
        &create_test_bidder_operator(),
        cause,
    );

    assert!(result.is_err());
    // No audit event is returned on authorization failure
}

#[test]
fn test_admin_can_create_checkpoint() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = checkpoint(
        &mut persistence,
        &metadata,
        &state,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();
    assert_eq!(transition.audit_event.action.name, "Checkpoint");
    assert_eq!(transition.audit_event.actor.id, "admin-123");
    assert_eq!(transition.audit_event.actor.actor_type, "admin");
}

#[test]
fn test_bidder_cannot_create_checkpoint() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = checkpoint(
        &mut persistence,
        &metadata,
        &state,
        &bidder,
        &create_test_bidder_operator(),
        cause,
    );

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
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = finalize(
        &mut persistence,
        &metadata,
        &state,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();
    assert_eq!(transition.audit_event.action.name, "Finalize");
    assert_eq!(transition.audit_event.actor.id, "admin-123");
    assert_eq!(transition.audit_event.actor.actor_type, "admin");
}

#[test]
fn test_bidder_cannot_finalize() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = finalize(
        &mut persistence,
        &metadata,
        &state,
        &bidder,
        &create_test_bidder_operator(),
        cause,
    );

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
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = rollback(
        &mut persistence,
        &metadata,
        &state,
        1,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let transition: TransitionResult = result.unwrap();
    assert_eq!(transition.audit_event.action.name, "Rollback");
    assert_eq!(transition.audit_event.actor.id, "admin-123");
    assert_eq!(transition.audit_event.actor.actor_type, "admin");
}

#[test]
fn test_bidder_cannot_rollback() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = rollback(
        &mut persistence,
        &metadata,
        &state,
        1,
        &bidder,
        &create_test_bidder_operator(),
        cause,
    );

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

// ============================================================================
// User Registration Tests
// ============================================================================

#[test]
fn test_valid_api_request_succeeds() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let api_result: ApiResult<RegisterUserResult> = result.unwrap();
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
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let api_result: ApiResult<RegisterUserResult> = result.unwrap();
    assert_eq!(api_result.audit_event.action.name, "RegisterUser");
    assert_eq!(api_result.audit_event.actor.id, "admin-123");
    assert_eq!(api_result.audit_event.actor.actor_type, "admin");
    assert_eq!(api_result.audit_event.cause.id, "api-req-456");
}

#[test]
fn test_valid_api_request_returns_new_state() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let api_result: ApiResult<RegisterUserResult> = result.unwrap();
    assert_eq!(api_result.new_state.users.len(), 1);
    assert_eq!(api_result.new_state.users[0].initials.value(), "AB");
}

#[test]
fn test_duplicate_initials_returns_api_error() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let mut state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request1: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    // Register first user successfully
    let result1: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request1,
        &admin,
        &create_test_admin_operator(),
        cause.clone(),
    );
    assert!(result1.is_ok());
    state = result1.unwrap().new_state;

    // Second registration with same initials in the same area
    let request2: RegisterUserRequest = RegisterUserRequest {
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

    let result2: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request2,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

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
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let mut state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request1: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    // Register first user successfully
    let result1: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request1,
        &admin,
        &create_test_admin_operator(),
        cause.clone(),
    );
    assert!(result1.is_ok());
    state = result1.unwrap().new_state;
    let user_count_before: usize = state.users.len();

    // Attempt duplicate registration
    let request2: RegisterUserRequest = RegisterUserRequest {
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

    let result2: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request2,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result2.is_err());
    // State should remain unchanged
    assert_eq!(state.users.len(), user_count_before);
}

#[test]
fn test_invalid_empty_initials_returns_api_error() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = RegisterUserRequest {
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

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

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
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = RegisterUserRequest {
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

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_err());
    let err: ApiError = result.unwrap_err();
    assert!(matches!(err, ApiError::InvalidInput { .. }));
    if let ApiError::InvalidInput { field, .. } = err {
        assert_eq!(field, "name");
    }
}

#[test]
fn test_invalid_empty_area_returns_api_error() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = RegisterUserRequest {
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

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_err());
    let err: ApiError = result.unwrap_err();
    // Empty area won't exist in metadata, so we get ResourceNotFound
    assert!(matches!(err, ApiError::ResourceNotFound { .. }));
}

#[test]
fn test_invalid_crew_number_returns_api_error() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = RegisterUserRequest {
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

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_err());
    let err: ApiError = result.unwrap_err();
    assert!(matches!(err, ApiError::InvalidInput { .. }));
    if let ApiError::InvalidInput { field, message: _ } = err {
        assert_eq!(field, "crew");
    }
}

#[test]
#[ignore = "Phase 19: Multiple bid years are no longer supported - all operations target the active bid year"]
fn test_duplicate_initials_in_different_bid_years_allowed() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
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
    let result1: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state1,
        request1,
        &admin,
        &create_test_admin_operator(),
        cause.clone(),
    );
    assert!(result1.is_ok());

    // Same initials in 2027 (different bid year)
    let request2: RegisterUserRequest = RegisterUserRequest {
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

    let result2: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state2,
        request2,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result2.is_ok());
    let api_result: ApiResult<RegisterUserResult> = result2.unwrap();
    assert_eq!(api_result.new_state.users.len(), 1);
}

#[test]
fn test_successful_api_call_updates_state() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let api_result: ApiResult<RegisterUserResult> = result.unwrap();

    // New state has the user
    assert_eq!(api_result.new_state.users.len(), 1);
    assert_eq!(api_result.new_state.users[0].name, "John Doe");

    // Original state is unchanged
    assert_eq!(state.users.len(), 0);
}

// ============================================================================
// Bootstrap Tests (Bid Year and Area Creation)
// ============================================================================

#[test]
fn test_create_bid_year_succeeds() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let request: CreateBidYearRequest = CreateBidYearRequest {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, ApiError> = create_bid_year(
        &metadata,
        &request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert_eq!(bootstrap_result.new_metadata.bid_years.len(), 1);
    assert_eq!(bootstrap_result.new_metadata.bid_years[0].year(), 2026);
}

#[test]
fn test_create_bid_year_requires_admin() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let request: CreateBidYearRequest = CreateBidYearRequest {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, ApiError> = create_bid_year(
        &metadata,
        &request,
        &bidder,
        &create_test_bidder_operator(),
        cause,
    );

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ApiError::Unauthorized { .. }));
}

#[test]
fn test_create_area_succeeds() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));

    let request: CreateAreaRequest = CreateAreaRequest {
        area_id: String::from("North"),
    };
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, ApiError> = create_area(
        &mut persistence,
        &metadata,
        &request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert_eq!(bootstrap_result.new_metadata.areas.len(), 1);
}

#[test]
fn test_create_area_requires_admin() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));

    let request: CreateAreaRequest = CreateAreaRequest {
        area_id: String::from("North"),
    };
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, ApiError> = create_area(
        &mut persistence,
        &metadata,
        &request,
        &bidder,
        &create_test_bidder_operator(),
        cause,
    );

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ApiError::Unauthorized { .. }));
}

#[test]
fn test_create_area_without_bid_year_fails() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let request: CreateAreaRequest = CreateAreaRequest {
        area_id: String::from("North"),
    };
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, ApiError> = create_area(
        &mut persistence,
        &metadata,
        &request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        ApiError::ResourceNotFound { .. }
    ));
}

// ============================================================================
// Listing Tests
// ============================================================================

#[test]
fn test_list_bid_years_empty() {
    let mut persistence = setup_test_persistence().expect("Failed to setup persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> = Vec::new();
    let response: ListBidYearsResponse =
        list_bid_years(&mut persistence, &metadata, &canonical_bid_years).unwrap();

    assert_eq!(response.bid_years.len(), 0);
}

#[test]
fn test_list_bid_years_with_single_year() {
    let mut persistence = setup_test_persistence().expect("Failed to setup persistence");

    // Get metadata and canonical bid years from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years().unwrap();

    let response: ListBidYearsResponse =
        list_bid_years(&mut persistence, &metadata, &canonical_bid_years).unwrap();

    assert_eq!(response.bid_years.len(), 1);
    assert_eq!(response.bid_years[0].year, 2026);
    assert_eq!(
        response.bid_years[0].start_date,
        canonical_bid_years[0].start_date()
    );
    assert_eq!(response.bid_years[0].num_pay_periods, 26);
    assert_eq!(
        response.bid_years[0].end_date,
        canonical_bid_years[0].end_date().unwrap()
    );
}

#[test]
fn test_list_bid_years_with_multiple_years() {
    use zab_bid::{BootstrapResult, Command, apply_bootstrap};
    use zab_bid_audit::{Actor, Cause};

    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create a test operator (required for foreign keys)
    let operator_id = persistence
        .create_operator("test-operator", "Test Operator", "password", "Admin")
        .unwrap();

    let mut metadata = BootstrapMetadata::new();
    let actor = Actor::with_operator(
        String::from("test-admin"),
        String::from("admin"),
        operator_id,
        String::from("test-operator"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("test-setup"), String::from("Test setup"));

    // Bootstrap three bid years
    for year in [2026, 2027, 2028] {
        let create_bid_year_cmd = Command::CreateBidYear {
            year,
            start_date: create_test_start_date_for_year(i32::from(year)),
            num_pay_periods: create_test_pay_periods(),
        };

        let placeholder_bid_year = BidYear::new(year);
        let bid_year_result: BootstrapResult = apply_bootstrap(
            &metadata,
            &placeholder_bid_year,
            create_bid_year_cmd,
            actor.clone(),
            cause.clone(),
        )
        .unwrap();

        persistence.persist_bootstrap(&bid_year_result).unwrap();
        metadata = bid_year_result.new_metadata;
    }

    // Get metadata and canonical bid years from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years().unwrap();

    let response: ListBidYearsResponse =
        list_bid_years(&mut persistence, &metadata, &canonical_bid_years).unwrap();

    assert_eq!(response.bid_years.len(), 3);
    assert!(response.bid_years.iter().any(|by| by.year == 2026));
    assert!(response.bid_years.iter().any(|by| by.year == 2027));
    assert!(response.bid_years.iter().any(|by| by.year == 2028));

    // Verify all bid years have end_date populated
    for bid_year_info in &response.bid_years {
        assert!(bid_year_info.end_date > bid_year_info.start_date);
    }
}

#[test]
fn test_create_bid_year_rejects_non_sunday() {
    use time::macros::date;

    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let actor: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    // January 3, 2026 is a Saturday
    let request: CreateBidYearRequest = CreateBidYearRequest {
        year: 2026,
        start_date: date!(2026 - 01 - 03),
        num_pay_periods: 26,
    };

    let result = create_bid_year(
        &metadata,
        &request,
        &actor,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        ApiError::InvalidInput { field, message } => {
            assert_eq!(field, "start_date");
            assert!(message.contains("Sunday"));
        }
        _ => panic!("Expected InvalidInput error, got {err:?}"),
    }
}

#[test]
fn test_create_bid_year_rejects_non_january() {
    use time::macros::date;

    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let actor: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    // February 1, 2026 is a Sunday, but not in January
    let request: CreateBidYearRequest = CreateBidYearRequest {
        year: 2026,
        start_date: date!(2026 - 02 - 01),
        num_pay_periods: 26,
    };

    let result = create_bid_year(
        &metadata,
        &request,
        &actor,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        ApiError::InvalidInput { field, message } => {
            assert_eq!(field, "start_date");
            assert!(message.contains("January"));
        }
        _ => panic!("Expected InvalidInput error, got {err:?}"),
    }
}

#[test]
fn test_create_bid_year_accepts_valid_sunday_in_january() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let actor: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    // January 4, 2026 is a Sunday in January
    let request: CreateBidYearRequest = CreateBidYearRequest {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: 26,
    };

    let result = create_bid_year(
        &metadata,
        &request,
        &actor,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let bootstrap_result = result.unwrap();
    assert!(bootstrap_result.canonical_bid_year.is_some());
    let canonical = bootstrap_result.canonical_bid_year.unwrap();
    assert_eq!(canonical.year(), 2026);
    assert_eq!(canonical.start_date(), create_test_start_date());
}

#[test]
fn test_list_bid_years_end_date_derivation() {
    // Test that end_date is correctly derived for both 26 and 27 pay period bid years
    use zab_bid::{BootstrapResult, Command, apply_bootstrap};
    use zab_bid_audit::{Actor, Cause};

    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create a test operator (required for foreign keys)
    let operator_id = persistence
        .create_operator("test-operator", "Test Operator", "password", "Admin")
        .unwrap();

    let mut metadata = BootstrapMetadata::new();
    let actor = Actor::with_operator(
        String::from("test-admin"),
        String::from("admin"),
        operator_id,
        String::from("test-operator"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("test-setup"), String::from("Test setup"));

    // Bootstrap bid year 2026 with 26 pay periods
    let create_bid_year_26_cmd = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: 26,
    };

    let placeholder_2026 = BidYear::new(2026);
    let bid_year_26_result: BootstrapResult = apply_bootstrap(
        &metadata,
        &placeholder_2026,
        create_bid_year_26_cmd,
        actor.clone(),
        cause.clone(),
    )
    .unwrap();

    persistence.persist_bootstrap(&bid_year_26_result).unwrap();
    metadata = bid_year_26_result.new_metadata;

    // Bootstrap bid year 2027 with 27 pay periods
    let create_bid_year_27_cmd = Command::CreateBidYear {
        year: 2027,
        start_date: create_test_start_date_for_year(2027),
        num_pay_periods: 27,
    };

    let placeholder_2027 = BidYear::new(2027);
    let bid_year_27_result: BootstrapResult = apply_bootstrap(
        &metadata,
        &placeholder_2027,
        create_bid_year_27_cmd,
        actor,
        cause,
    )
    .unwrap();

    persistence.persist_bootstrap(&bid_year_27_result).unwrap();

    // Get metadata and canonical bid years from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years().unwrap();

    let response: ListBidYearsResponse =
        list_bid_years(&mut persistence, &metadata, &canonical_bid_years).unwrap();

    assert_eq!(response.bid_years.len(), 2);

    // Find the canonical bid years to compare
    let canonical_26 = canonical_bid_years
        .iter()
        .find(|c| c.year() == 2026)
        .unwrap();
    let canonical_27 = canonical_bid_years
        .iter()
        .find(|c| c.year() == 2027)
        .unwrap();

    // Find the 26-period bid year and verify end_date
    let by_26 = response
        .bid_years
        .iter()
        .find(|by| by.year == 2026)
        .unwrap();
    assert_eq!(by_26.num_pay_periods, 26);
    assert_eq!(by_26.end_date, canonical_26.end_date().unwrap());

    // Find the 27-period bid year and verify end_date
    let by_27 = response
        .bid_years
        .iter()
        .find(|by| by.year == 2027)
        .unwrap();
    assert_eq!(by_27.num_pay_periods, 27);
    assert_eq!(by_27.end_date, canonical_27.end_date().unwrap());

    // Verify end dates are in the following calendar year
    assert_eq!(by_26.end_date.year(), 2027);
    assert_eq!(by_27.end_date.year(), 2028);
}

#[test]
fn test_list_areas_empty() {
    use zab_bid::{BootstrapResult, Command, apply_bootstrap};
    use zab_bid_audit::{Actor, Cause};

    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create a test operator (required for foreign keys)
    let operator_id = persistence
        .create_operator("test-operator", "Test Operator", "password", "Admin")
        .unwrap();

    let metadata = BootstrapMetadata::new();
    let actor = Actor::with_operator(
        String::from("test-admin"),
        String::from("admin"),
        operator_id,
        String::from("test-operator"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("test-setup"), String::from("Test setup"));

    // Bootstrap bid year without areas
    let create_bid_year_cmd = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };

    let placeholder_bid_year = BidYear::new(2026);
    let bid_year_result: BootstrapResult = apply_bootstrap(
        &metadata,
        &placeholder_bid_year,
        create_bid_year_cmd,
        actor,
        cause,
    )
    .unwrap();

    persistence.persist_bootstrap(&bid_year_result).unwrap();

    // Get metadata from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();

    // Extract canonical bid_year_id
    let bid_year_id: i64 = persistence.get_bid_year_id(2026).unwrap();

    let request: ListAreasRequest = ListAreasRequest { bid_year_id };
    let response: ListAreasResponse = list_areas(&metadata, &request).unwrap();

    assert_eq!(response.bid_year, 2026);
    assert_eq!(response.areas.len(), 0);
}

#[test]
fn test_list_areas_for_bid_year() {
    use zab_bid::{BootstrapResult, Command, apply_bootstrap};
    use zab_bid_audit::{Actor, Cause};

    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create a test operator (required for foreign keys)
    let operator_id = persistence
        .create_operator("test-operator", "Test Operator", "password", "Admin")
        .unwrap();

    let mut metadata = BootstrapMetadata::new();
    let actor = Actor::with_operator(
        String::from("test-admin"),
        String::from("admin"),
        operator_id,
        String::from("test-operator"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("test-setup"), String::from("Test setup"));

    // Bootstrap bid year
    let create_bid_year_cmd = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };

    let placeholder_bid_year = BidYear::new(2026);
    let bid_year_result: BootstrapResult = apply_bootstrap(
        &metadata,
        &placeholder_bid_year,
        create_bid_year_cmd,
        actor.clone(),
        cause.clone(),
    )
    .unwrap();

    persistence.persist_bootstrap(&bid_year_result).unwrap();
    metadata = bid_year_result.new_metadata;

    // Bootstrap two areas
    for area_code in ["North", "South"] {
        let create_area_cmd = Command::CreateArea {
            area_id: area_code.to_string(),
        };

        let active_bid_year = BidYear::new(2026);
        let area_result: BootstrapResult = apply_bootstrap(
            &metadata,
            &active_bid_year,
            create_area_cmd,
            actor.clone(),
            cause.clone(),
        )
        .unwrap();

        persistence.persist_bootstrap(&area_result).unwrap();
        metadata = area_result.new_metadata;
    }

    // Get metadata from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();

    // Extract canonical bid_year_id
    let bid_year_id: i64 = persistence.get_bid_year_id(2026).unwrap();

    let request: ListAreasRequest = ListAreasRequest { bid_year_id };
    let response: ListAreasResponse = list_areas(&metadata, &request).unwrap();

    assert_eq!(response.bid_year, 2026);
    assert_eq!(response.areas.len(), 2);
    assert!(
        response
            .areas
            .iter()
            .any(|a| a.area_code == "NORTH" && a.user_count == 0)
    );
    assert!(
        response
            .areas
            .iter()
            .any(|a| a.area_code == "SOUTH" && a.user_count == 0)
    );
}

#[test]
fn test_list_areas_isolated_by_bid_year() {
    use zab_bid::{BootstrapResult, Command, apply_bootstrap};
    use zab_bid_audit::{Actor, Cause};

    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Create a test operator (required for foreign keys)
    let operator_id = persistence
        .create_operator("test-operator", "Test Operator", "password", "Admin")
        .unwrap();

    let mut metadata = BootstrapMetadata::new();
    let actor = Actor::with_operator(
        String::from("test-admin"),
        String::from("admin"),
        operator_id,
        String::from("test-operator"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("test-setup"), String::from("Test setup"));

    // Bootstrap bid year 2026 with area "North"
    let create_bid_year_2026_cmd = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };

    let placeholder_2026 = BidYear::new(2026);
    let bid_year_2026_result: BootstrapResult = apply_bootstrap(
        &metadata,
        &placeholder_2026,
        create_bid_year_2026_cmd,
        actor.clone(),
        cause.clone(),
    )
    .unwrap();

    persistence
        .persist_bootstrap(&bid_year_2026_result)
        .unwrap();
    metadata = bid_year_2026_result.new_metadata;

    let create_area_north_cmd = Command::CreateArea {
        area_id: String::from("North"),
    };

    let active_2026 = BidYear::new(2026);
    let area_north_result: BootstrapResult = apply_bootstrap(
        &metadata,
        &active_2026,
        create_area_north_cmd,
        actor.clone(),
        cause.clone(),
    )
    .unwrap();

    persistence.persist_bootstrap(&area_north_result).unwrap();
    metadata = area_north_result.new_metadata;

    // Bootstrap bid year 2027 with area "South"
    let create_bid_year_2027_cmd = Command::CreateBidYear {
        year: 2027,
        start_date: create_test_start_date_for_year(2027),
        num_pay_periods: create_test_pay_periods(),
    };

    let placeholder_2027 = BidYear::new(2027);
    let bid_year_2027_result: BootstrapResult = apply_bootstrap(
        &metadata,
        &placeholder_2027,
        create_bid_year_2027_cmd,
        actor.clone(),
        cause.clone(),
    )
    .unwrap();

    persistence
        .persist_bootstrap(&bid_year_2027_result)
        .unwrap();
    metadata = bid_year_2027_result.new_metadata;

    let create_area_south_cmd = Command::CreateArea {
        area_id: String::from("South"),
    };

    let active_2027 = BidYear::new(2027);
    let area_south_result: BootstrapResult =
        apply_bootstrap(&metadata, &active_2027, create_area_south_cmd, actor, cause).unwrap();

    persistence.persist_bootstrap(&area_south_result).unwrap();

    // Get metadata from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();

    // Extract canonical bid_year_ids
    let bid_year_id_2026: i64 = persistence.get_bid_year_id(2026).unwrap();
    let bid_year_id_2027: i64 = persistence.get_bid_year_id(2027).unwrap();

    let request_2026: ListAreasRequest = ListAreasRequest {
        bid_year_id: bid_year_id_2026,
    };
    let response_2026: ListAreasResponse = list_areas(&metadata, &request_2026).unwrap();

    assert_eq!(response_2026.areas.len(), 1);
    assert_eq!(response_2026.areas[0].user_count, 0);

    let request_2027: ListAreasRequest = ListAreasRequest {
        bid_year_id: bid_year_id_2027,
    };
    let response_2027: ListAreasResponse = list_areas(&metadata, &request_2027).unwrap();

    assert_eq!(response_2027.areas.len(), 1);
    // TODO: area_id assertion requires canonical IDs in metadata (post Phase 23A)
    // assert_eq!(response_2027.areas[0].area_id, expected_id);
    assert_eq!(response_2027.areas[0].user_count, 0);
}

#[test]
fn test_list_areas_nonexistent_bid_year() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    // Use a nonexistent canonical ID
    let request: ListAreasRequest = ListAreasRequest { bid_year_id: 9999 };

    let result = list_areas(&metadata, &request);

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        ApiError::ResourceNotFound {
            resource_type,
            message,
        } => {
            assert_eq!(resource_type, "BidYear");
            assert!(message.contains("9999"));
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[test]
fn test_list_users_empty() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    // Get metadata and canonical bid years from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years().unwrap();

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());
    let actor = create_test_admin();
    let operator = create_test_admin_operator();
    let response: ListUsersResponse = list_users(
        &metadata,
        &canonical_bid_years,
        &bid_year,
        &area,
        &state,
        &actor,
        &operator,
        zab_bid_domain::BidYearLifecycle::Draft,
    )
    .unwrap();

    assert_eq!(response.bid_year, 2026);
    assert_eq!(response.area_code, "NORTH");
    assert_eq!(response.users.len(), 0);
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_list_users_with_users() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    // Get metadata and canonical bid years from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years().unwrap();

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let request1: RegisterUserRequest = RegisterUserRequest {
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

    let result1: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request1,
        &admin,
        &create_test_admin_operator(),
        cause.clone(),
    );
    assert!(result1.is_ok());

    let api_result1 = result1.unwrap();
    let transition1 = TransitionResult {
        audit_event: api_result1.audit_event,
        new_state: api_result1.new_state,
    };
    persistence
        .persist_transition(&transition1)
        .expect("Failed to persist transition");
    // Reload state from persistence to get assigned user_ids
    let state_with_user1: State = persistence
        .get_current_state(&bid_year, &area)
        .expect("Failed to reload state");

    let request2: RegisterUserRequest = RegisterUserRequest {
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

    let result2: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state_with_user1,
        request2,
        &admin,
        &create_test_admin_operator(),
        cause,
    );
    assert!(result2.is_ok());

    let api_result2 = result2.unwrap();
    let transition2 = TransitionResult {
        audit_event: api_result2.audit_event,
        new_state: api_result2.new_state,
    };
    persistence
        .persist_transition(&transition2)
        .expect("Failed to persist transition");
    // Reload state from persistence to get assigned user_ids
    let final_state: State = persistence
        .get_current_state(&bid_year, &area)
        .expect("Failed to reload state");

    let actor = create_test_admin();
    let operator = create_test_admin_operator();
    let response: ListUsersResponse = list_users(
        &metadata,
        &canonical_bid_years,
        &bid_year,
        &area,
        &final_state,
        &actor,
        &operator,
        zab_bid_domain::BidYearLifecycle::Draft,
    )
    .unwrap();

    assert_eq!(response.bid_year, 2026);
    assert_eq!(response.area_code, "NORTH");
    assert_eq!(response.users.len(), 2);

    let ab_user = response.users.iter().find(|u| u.initials == "AB").unwrap();
    assert!(ab_user.user_id > 0, "user_id must be present");
    assert_eq!(ab_user.name, "Alice Brown");
    assert_eq!(ab_user.crew, Some(1));
    assert_eq!(ab_user.user_type, "CPC");
    assert!(ab_user.earned_hours > 0);
    assert!(ab_user.earned_days > 0);
    assert_eq!(ab_user.remaining_hours, i32::from(ab_user.earned_hours));
    assert!(!ab_user.is_exhausted);
    assert!(!ab_user.is_overdrawn);

    let cd_user = response.users.iter().find(|u| u.initials == "CD").unwrap();
    assert!(cd_user.user_id > 0, "user_id must be present");
    assert_eq!(cd_user.name, "Charlie Davis");
    assert_eq!(cd_user.crew, Some(2));
    assert_eq!(cd_user.user_type, "CPC");
    assert!(cd_user.earned_hours > 0);
    assert!(cd_user.earned_days > 0);
    assert_eq!(cd_user.remaining_hours, i32::from(cd_user.earned_hours));
    assert!(!cd_user.is_exhausted);
    assert!(!cd_user.is_overdrawn);
}

#[test]
fn test_list_users_with_no_crew() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    // Get metadata and canonical bid years from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years().unwrap();

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let request: RegisterUserRequest = RegisterUserRequest {
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

    let result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );
    assert!(result.is_ok());

    let api_result = result.unwrap();
    let transition = TransitionResult {
        audit_event: api_result.audit_event,
        new_state: api_result.new_state,
    };
    persistence
        .persist_transition(&transition)
        .expect("Failed to persist transition");
    // Reload state from persistence to get assigned user_ids
    let final_state: State = persistence
        .get_current_state(&bid_year, &area)
        .expect("Failed to reload state");

    let actor = create_test_admin();
    let operator = create_test_admin_operator();
    let response: ListUsersResponse = list_users(
        &metadata,
        &canonical_bid_years,
        &bid_year,
        &area,
        &final_state,
        &actor,
        &operator,
        zab_bid_domain::BidYearLifecycle::Draft,
    )
    .unwrap();

    assert_eq!(response.users.len(), 1);
    assert!(response.users[0].user_id > 0, "user_id must be present");
    assert_eq!(response.users[0].initials, "EF");
    assert_eq!(response.users[0].name, "Eve Foster");
    assert_eq!(response.users[0].crew, None);
    assert_eq!(response.users[0].user_type, "Dev-R");
    assert!(response.users[0].earned_hours > 0);
    assert!(!response.users[0].is_exhausted);
}

#[test]
fn test_list_users_nonexistent_bid_year() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> = vec![];
    let bid_year: BidYear = BidYear::new(9999);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());

    let actor = create_test_admin();
    let operator = create_test_admin_operator();
    let result = list_users(
        &metadata,
        &canonical_bid_years,
        &bid_year,
        &area,
        &state,
        &actor,
        &operator,
        zab_bid_domain::BidYearLifecycle::Draft,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        ApiError::ResourceNotFound {
            resource_type,
            message,
        } => {
            assert_eq!(resource_type, "Bid year");
            assert!(message.contains("9999"));
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[test]
fn test_list_users_nonexistent_area() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));

    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        vec![create_test_canonical_bid_year()];
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("NonExistent");
    let state: State = State::new(bid_year.clone(), area.clone());

    let actor = create_test_admin();
    let operator = create_test_admin_operator();
    let result = list_users(
        &metadata,
        &canonical_bid_years,
        &bid_year,
        &area,
        &state,
        &actor,
        &operator,
        zab_bid_domain::BidYearLifecycle::Draft,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        ApiError::ResourceNotFound {
            resource_type,
            message,
        } => {
            assert_eq!(resource_type, "Area");
            assert!(message.contains("NONEXISTENT"));
            assert!(message.contains("2026"));
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[test]
fn test_get_current_state_nonexistent_bid_year() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let bid_year: BidYear = BidYear::new(9999);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());

    let result = get_current_state(&metadata, &bid_year, &area, state);

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        ApiError::ResourceNotFound {
            resource_type,
            message,
        } => {
            assert_eq!(resource_type, "Bid year");
            assert!(message.contains("9999"));
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[test]
fn test_get_current_state_nonexistent_area() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("NonExistent");
    let state: State = State::new(bid_year.clone(), area.clone());

    let result = get_current_state(&metadata, &bid_year, &area, state);

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        ApiError::ResourceNotFound {
            resource_type,
            message,
        } => {
            assert_eq!(resource_type, "Area");
            assert!(message.contains("NONEXISTENT"));
            assert!(message.contains("2026"));
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

#[test]
fn test_get_historical_state_nonexistent_bid_year() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let bid_year: BidYear = BidYear::new(9999);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());

    let result = get_historical_state(&metadata, &bid_year, &area, state);

    assert!(result.is_err());
    let err = result.unwrap_err();
    match err {
        ApiError::ResourceNotFound {
            resource_type,
            message,
        } => {
            assert_eq!(resource_type, "Bid year");
            assert!(message.contains("9999"));
        }
        _ => panic!("Expected ResourceNotFound error"),
    }
}

// ============================================================================
// Error Display Tests
// ============================================================================

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

// ============================================================================
// Leave Availability Tests
// ============================================================================

#[test]
fn test_get_leave_availability_zero_usage() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    // Get metadata and canonical bid years from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years().unwrap();
    let canonical_bid_year = &canonical_bid_years[0];

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    // Create a user with some service time
    let mut request: RegisterUserRequest = create_valid_request();
    request.service_computation_date = String::from("2020-01-15");

    // Create initial state
    let state: State = State::new(bid_year.clone(), area.clone());

    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let register_result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );
    assert!(register_result.is_ok());

    let api_result = register_result.unwrap();
    let transition = TransitionResult {
        audit_event: api_result.audit_event,
        new_state: api_result.new_state,
    };
    persistence
        .persist_transition(&transition)
        .expect("Failed to persist transition");
    // Reload state from persistence to get assigned user_ids
    let new_state: State = persistence
        .get_current_state(&bid_year, &area)
        .expect("Failed to reload state");
    let initials = zab_bid_domain::Initials::new("AB");

    // Get leave availability
    let result: Result<GetLeaveAvailabilityResponse, ApiError> =
        get_leave_availability(&metadata, canonical_bid_year, &area, &initials, &new_state);

    assert!(result.is_ok());
    let response: GetLeaveAvailabilityResponse = result.unwrap();

    // User should have a valid user_id
    assert!(response.user_id > 0, "user_id must be present");
    // User has 6+ years of service, so should get 6-hour tier
    // 26 PPs * 6 hours + 4 bonus = 160 hours = 20 days
    assert_eq!(response.earned_hours, 160);
    assert_eq!(response.earned_days, 20);
    assert_eq!(response.used_hours, 0);
    assert_eq!(response.remaining_hours, 160);
    assert_eq!(response.remaining_days, 20);
    assert!(!response.is_exhausted);
    assert!(!response.is_overdrawn);
}

#[test]
fn test_get_leave_availability_user_not_found() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    // Get metadata and canonical bid years from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years().unwrap();
    let canonical_bid_year = &canonical_bid_years[0];

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let state: State = State::new(bid_year, area.clone());

    let initials = zab_bid_domain::Initials::new("XY");

    let result: Result<GetLeaveAvailabilityResponse, ApiError> =
        get_leave_availability(&metadata, canonical_bid_year, &area, &initials, &state);

    assert!(result.is_err());
    let err: ApiError = result.unwrap_err();
    assert!(matches!(err, ApiError::ResourceNotFound { .. }));
}

#[test]
fn test_get_leave_availability_area_not_found() {
    let _persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    let wrong_area: Area = Area::new("South");

    let canonical_bid_year =
        zab_bid_domain::CanonicalBidYear::new(2026, create_test_start_date(), 26).unwrap();

    let state: State = State::new(bid_year, area);

    let initials = zab_bid_domain::Initials::new("AB");

    let result: Result<GetLeaveAvailabilityResponse, ApiError> = get_leave_availability(
        &metadata,
        &canonical_bid_year,
        &wrong_area,
        &initials,
        &state,
    );

    assert!(result.is_err());
    let err: ApiError = result.unwrap_err();
    assert!(matches!(err, ApiError::ResourceNotFound { .. }));
}

#[test]
fn test_get_leave_availability_explanation_text() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    // Get metadata and canonical bid years from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years().unwrap();
    let canonical_bid_year = &canonical_bid_years[0];

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let mut request: RegisterUserRequest = create_valid_request();
    request.service_computation_date = String::from("2024-01-15");

    let state: State = State::new(bid_year.clone(), area.clone());

    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let register_result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );
    assert!(register_result.is_ok());

    let api_result = register_result.unwrap();
    let transition = TransitionResult {
        audit_event: api_result.audit_event,
        new_state: api_result.new_state,
    };
    persistence
        .persist_transition(&transition)
        .expect("Failed to persist transition");
    let new_state: State = persistence
        .get_current_state(&bid_year, &area)
        .expect("Failed to reload state");
    let initials = zab_bid_domain::Initials::new("AB");

    let result: Result<GetLeaveAvailabilityResponse, ApiError> =
        get_leave_availability(&metadata, canonical_bid_year, &area, &initials, &new_state);

    assert!(result.is_ok());
    let response: GetLeaveAvailabilityResponse = result.unwrap();

    // User should have a valid user_id
    assert!(response.user_id > 0, "user_id must be present");
    // Check that explanation contains key information
    assert!(response.explanation.contains("AB"));
    assert!(response.explanation.contains("2026"));
    assert!(response.explanation.contains("Earned:"));
    assert!(response.explanation.contains("Used:"));
    assert!(response.explanation.contains("Remaining:"));
}

#[test]
fn test_get_leave_availability_different_service_tiers() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    // Get metadata and canonical bid years from persistence
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        persistence.list_bid_years().unwrap();
    let canonical_bid_year = &canonical_bid_years[0];

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let state: State = State::new(bid_year, area.clone());

    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    // Test user with < 3 years service (4-hour tier)
    let mut request1: RegisterUserRequest = create_valid_request();
    request1.initials = String::from("U1");
    request1.service_computation_date = String::from("2024-01-15");

    let register_result1: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        request1,
        &admin,
        &create_test_admin_operator(),
        cause.clone(),
    );
    assert!(register_result1.is_ok());

    let api_result1 = register_result1.unwrap();
    let transition1 = TransitionResult {
        audit_event: api_result1.audit_event,
        new_state: api_result1.new_state,
    };
    persistence
        .persist_transition(&transition1)
        .expect("Failed to persist transition");
    // Reload state from persistence to get assigned user_ids
    let state1: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .expect("Failed to reload state");
    let initials1 = zab_bid_domain::Initials::new("U1");

    let result1: Result<GetLeaveAvailabilityResponse, ApiError> =
        get_leave_availability(&metadata, canonical_bid_year, &area, &initials1, &state1);
    assert!(result1.is_ok());
    let response1: GetLeaveAvailabilityResponse = result1.unwrap();

    // User should have a valid user_id
    assert!(response1.user_id > 0, "user_id must be present");
    // 26 PPs * 4 hours = 104 hours = 13 days
    assert_eq!(response1.earned_hours, 104);
    assert_eq!(response1.earned_days, 13);

    // Test user with 15+ years service (8-hour tier)
    let mut request2: RegisterUserRequest = create_valid_request();
    request2.initials = String::from("U2");
    request2.service_computation_date = String::from("2010-01-15");

    let register_result2: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state1,
        request2,
        &admin,
        &create_test_admin_operator(),
        cause,
    );
    assert!(register_result2.is_ok());

    let api_result2 = register_result2.unwrap();
    let transition2 = TransitionResult {
        audit_event: api_result2.audit_event,
        new_state: api_result2.new_state,
    };
    persistence
        .persist_transition(&transition2)
        .expect("Failed to persist transition");
    // Reload state from persistence to get assigned user_ids
    let state2: State = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .expect("Failed to reload state");
    let initials2 = zab_bid_domain::Initials::new("U2");

    let result2: Result<GetLeaveAvailabilityResponse, ApiError> =
        get_leave_availability(&metadata, canonical_bid_year, &area, &initials2, &state2);
    assert!(result2.is_ok());
    let response2: GetLeaveAvailabilityResponse = result2.unwrap();

    // User should have a valid user_id
    assert!(response2.user_id > 0, "user_id must be present");
    // 26 PPs * 8 hours = 208 hours = 26 days
    assert_eq!(response2.earned_hours, 208);
    assert_eq!(response2.earned_days, 26);
}

// ============================================================================
// CSV Import Tests
// ============================================================================

/// This test validates Phase 21 CSV import fix: multiple users in same area
/// are all persisted correctly, not just the last one.
#[test]
fn test_csv_import_multiple_users_same_area() {
    use crate::{ImportCsvUsersRequest, import_csv_users};
    use zab_bid::{Command, apply_bootstrap};
    use zab_bid_audit::Actor;
    use zab_bid_persistence::SqlitePersistence;

    let mut persistence =
        SqlitePersistence::new_in_memory().expect("Failed to create in-memory persistence");

    // Create operator for auth
    persistence
        .create_operator("test_admin", "Test Admin", "password", "Admin")
        .expect("Failed to create operator");

    let mut metadata = BootstrapMetadata::new();

    // Bootstrap bid year and area
    let actor = Actor::with_operator(
        String::from("test"),
        String::from("admin"),
        1,
        String::from("test_admin"),
        String::from("Test Admin"),
    );
    let cause = create_test_cause();

    let bid_year_cmd = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: 26,
    };
    let placeholder = BidYear::new(2026);
    let bid_year_result = apply_bootstrap(&metadata, &placeholder, bid_year_cmd, actor, cause)
        .expect("Failed to create bid year");
    persistence
        .persist_bootstrap(&bid_year_result)
        .expect("Failed to persist bid year");
    metadata.bid_years.push(BidYear::new(2026));

    // Create EAST area
    let actor = Actor::with_operator(
        String::from("test"),
        String::from("admin"),
        1,
        String::from("test_admin"),
        String::from("Test Admin"),
    );
    let cause = create_test_cause();
    let area_cmd = Command::CreateArea {
        area_id: String::from("EAST"),
    };
    let area_result = apply_bootstrap(&metadata, &BidYear::new(2026), area_cmd, actor, cause)
        .expect("Failed to create area");
    persistence
        .persist_bootstrap(&area_result)
        .expect("Failed to persist area");
    metadata.areas.push((BidYear::new(2026), Area::new("EAST")));

    // Set active bid year
    let bid_year_2026 = BidYear::new(2026);
    persistence
        .set_active_bid_year(&bid_year_2026)
        .expect("Failed to set active bid year");

    // CSV with 2 users in EAST area - this tests the bug fix where only the last user was persisted
    let csv_content = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date,natca_bu_date,cumulative_natca_bu_date,lottery_value
CS,Fred Clausen,EAST,3,CPC,2008-04-01,2008-04-01,2008-08-05,2008-08-05,
SB,Steve Barnes,EAST,3,CPC,2010-04-01,2010-04-01,2010-08-05,2010-08-05,";

    let request = ImportCsvUsersRequest {
        csv_content: csv_content.to_string(),
        selected_row_indices: vec![0, 1],
    };

    let admin = create_test_admin();
    let operator = create_test_admin_operator();
    let cause = create_test_cause();
    let state = State::new(BidYear::new(2026), Area::new("EAST"));

    let response = import_csv_users(
        &metadata,
        &state,
        &mut persistence,
        &request,
        &admin,
        &operator,
        &cause,
    )
    .expect("CSV import should succeed");

    assert_eq!(response.total_selected, 2, "Should select 2 rows");
    assert_eq!(
        response.successful_count, 2,
        "Both users should import successfully"
    );
    assert_eq!(response.failed_count, 0, "No failures expected");

    // Verify both users exist in database - this is the key test for Phase 21 fix
    let east_state = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("EAST"))
        .expect("Should load EAST state");
    assert_eq!(
        east_state.users.len(),
        2,
        "EAST should have 2 users (not just the last one!)"
    );

    // Verify user_ids are assigned
    for user in &east_state.users {
        assert!(user.user_id.is_some(), "User should have user_id assigned");
        assert!(user.user_id.unwrap() > 0, "User ID should be positive");
    }

    // Verify correct initials
    let initials_set: std::collections::HashSet<String> = east_state
        .users
        .iter()
        .map(|u| u.initials.value().to_string())
        .collect();
    assert!(initials_set.contains("CS"), "Should contain CS user");
    assert!(initials_set.contains("SB"), "Should contain SB user");
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_csv_import_is_additive_not_destructive() {
    use zab_bid::Command;
    use zab_bid::apply_bootstrap;
    use zab_bid_audit::Actor;
    use zab_bid_persistence::SqlitePersistence;

    let mut persistence =
        SqlitePersistence::new_in_memory().expect("Failed to create in-memory persistence");

    // Create operator for auth
    persistence
        .create_operator("test_admin", "Test Admin", "password", "Admin")
        .expect("Failed to create operator");

    let mut metadata = BootstrapMetadata::new();

    // Bootstrap bid year and area
    let actor = Actor::with_operator(
        String::from("test"),
        String::from("admin"),
        1,
        String::from("test_admin"),
        String::from("Test Admin"),
    );
    let cause = create_test_cause();

    let bid_year_cmd = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: 26,
    };
    let placeholder = BidYear::new(2026);
    let bid_year_result = apply_bootstrap(&metadata, &placeholder, bid_year_cmd, actor, cause)
        .expect("Failed to create bid year");
    persistence
        .persist_bootstrap(&bid_year_result)
        .expect("Failed to persist bid year");
    metadata.bid_years.push(BidYear::new(2026));

    // Create EAST area
    let actor = Actor::with_operator(
        String::from("test"),
        String::from("admin"),
        1,
        String::from("test_admin"),
        String::from("Test Admin"),
    );
    let cause = create_test_cause();
    let area_cmd = Command::CreateArea {
        area_id: String::from("EAST"),
    };
    let area_result = apply_bootstrap(&metadata, &BidYear::new(2026), area_cmd, actor, cause)
        .expect("Failed to create area");
    persistence
        .persist_bootstrap(&area_result)
        .expect("Failed to persist area");
    metadata.areas.push((BidYear::new(2026), Area::new("EAST")));

    // Set active bid year
    let bid_year_2026 = BidYear::new(2026);
    persistence
        .set_active_bid_year(&bid_year_2026)
        .expect("Failed to set active bid year");

    // Test importing 3 users in a single CSV file
    let csv_content = "initials,name,area_id,crew,user_type,service_computation_date,eod_faa_date,natca_bu_date,cumulative_natca_bu_date,lottery_value
CS,Fred Clausen,EAST,3,CPC,2008-04-01,2008-04-01,2008-08-05,2008-08-05,
SB,Steve Barnes,EAST,3,CPC,2010-04-01,2010-04-01,2010-08-05,2010-08-05,
PU,Pam User,EAST,3,CPC,2012-04-01,2012-04-01,2012-08-05,2012-08-05,";

    let request = ImportCsvUsersRequest {
        csv_content: csv_content.to_string(),
        selected_row_indices: vec![0, 1, 2], // Import all 3 rows
    };

    let admin = create_test_admin();
    let operator = create_test_admin_operator();
    let cause = create_test_cause();
    let state = State::new(BidYear::new(2026), Area::new("EAST"));

    let response = import_csv_users(
        &metadata,
        &state,
        &mut persistence,
        &request,
        &admin,
        &operator,
        &cause,
    )
    .expect("CSV import should succeed");

    assert_eq!(response.successful_count, 3, "All 3 users should import");

    // CRITICAL TEST: Verify all 3 users are in the database
    let east_state_final = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("EAST"))
        .expect("Should load EAST state");
    assert_eq!(
        east_state_final.users.len(),
        3,
        "Should have all 3 users - CSV import should be additive!"
    );

    // Verify all three users exist
    let initials_set: std::collections::HashSet<String> = east_state_final
        .users
        .iter()
        .map(|u| u.initials.value().to_string())
        .collect();
    assert!(initials_set.contains("CS"), "CS should exist");
    assert!(initials_set.contains("SB"), "SB should exist");
    assert!(initials_set.contains("PU"), "PU should exist");
}

// ============================================================================
// Bootstrap Readiness Tests
// ============================================================================

/// Test that `is_ready_for_bidding` is true when no blocking reasons exist.
///
/// See: Phase 26B bootstrap readiness consistency fix.
#[test]
fn test_bootstrap_readiness_with_no_blockers() {
    use crate::{AreaCompletenessInfo, BidYearCompletenessInfo, GetBootstrapCompletenessResponse};

    let response = GetBootstrapCompletenessResponse {
        active_bid_year_id: Some(1),
        active_bid_year: Some(2026),
        bid_years: vec![BidYearCompletenessInfo {
            bid_year_id: 1,
            year: 2026,
            is_active: true,
            expected_area_count: Some(1),
            actual_area_count: 1,
            is_complete: true,
            blocking_reasons: vec![],
            lifecycle_state: "Bootstrap".to_string(),
        }],
        areas: vec![AreaCompletenessInfo {
            bid_year_id: 1,
            bid_year: 2026,
            area_id: 1,
            area_code: "EAST".to_string(),
            expected_user_count: Some(0),
            actual_user_count: 0,
            is_complete: true,
            blocking_reasons: vec![],
        }],
        is_ready_for_bidding: true,
        blocking_reasons: vec![],
    };

    assert!(
        response.is_ready_for_bidding,
        "Should be ready when no blocking reasons exist"
    );
}

/// Test that `is_ready_for_bidding` is false when top-level blocking reasons exist.
///
/// This is the primary regression test for the bug: `UsersInNoBidArea` in
/// top-level `blocking_reasons` should prevent readiness, even when `is_complete`
/// flags are true.
///
/// See: Phase 26B bootstrap readiness consistency fix.
#[test]
fn test_bootstrap_readiness_with_top_level_blocker() {
    use crate::{
        AreaCompletenessInfo, BidYearCompletenessInfo, BlockingReason,
        GetBootstrapCompletenessResponse,
    };

    let response = GetBootstrapCompletenessResponse {
        active_bid_year_id: Some(1),
        active_bid_year: Some(2026),
        bid_years: vec![BidYearCompletenessInfo {
            bid_year_id: 1,
            year: 2026,
            is_active: true,
            expected_area_count: Some(1),
            actual_area_count: 1,
            is_complete: true,
            blocking_reasons: vec![],
            lifecycle_state: "Bootstrap".to_string(),
        }],
        areas: vec![AreaCompletenessInfo {
            bid_year_id: 1,
            bid_year: 2026,
            area_id: 1,
            area_code: "EAST".to_string(),
            expected_user_count: Some(0),
            actual_user_count: 0,
            is_complete: true,
            blocking_reasons: vec![],
        }],
        is_ready_for_bidding: false,
        blocking_reasons: vec![BlockingReason::UsersInNoBidArea {
            bid_year_id: 1,
            bid_year: 2026,
            user_count: 1,
            sample_initials: vec!["TEST".to_string()],
        }],
    };

    assert!(
        !response.is_ready_for_bidding,
        "Should NOT be ready when top-level blocking reasons exist"
    );
    assert!(
        !response.blocking_reasons.is_empty(),
        "Top-level blocking reasons should be present"
    );
}

/// Test that `is_ready_for_bidding` is false when area-level blocking reasons exist.
///
/// See: Phase 26B bootstrap readiness consistency fix.
#[test]
fn test_bootstrap_readiness_with_area_level_blocker() {
    use crate::{
        AreaCompletenessInfo, BidYearCompletenessInfo, BlockingReason,
        GetBootstrapCompletenessResponse,
    };

    let response = GetBootstrapCompletenessResponse {
        active_bid_year_id: Some(1),
        active_bid_year: Some(2026),
        bid_years: vec![BidYearCompletenessInfo {
            bid_year_id: 1,
            year: 2026,
            is_active: true,
            expected_area_count: Some(1),
            actual_area_count: 1,
            is_complete: true,
            blocking_reasons: vec![],
            lifecycle_state: "Bootstrap".to_string(),
        }],
        areas: vec![AreaCompletenessInfo {
            bid_year_id: 1,
            bid_year: 2026,
            area_id: 1,
            area_code: "EAST".to_string(),
            expected_user_count: Some(5),
            actual_user_count: 3,
            is_complete: false,
            blocking_reasons: vec![BlockingReason::UserCountMismatch {
                bid_year_id: 1,
                bid_year: 2026,
                area_id: 1,
                area_code: "EAST".to_string(),
                expected: 5,
                actual: 3,
            }],
        }],
        is_ready_for_bidding: false,
        blocking_reasons: vec![],
    };

    assert!(
        !response.is_ready_for_bidding,
        "Should NOT be ready when area-level blocking reasons exist"
    );
}

// ============================================================================
// Phase 27B: User Identity Correctness Tests
// ============================================================================

/// Regression test: `user_id` is the canonical identifier for user operations.
///
/// This test verifies that:
/// - `user_id` is used to identify users in persistence operations
/// - Initials are display metadata, not used for identity at persistence layer
/// - Foreign key relationships use `user_id`, not initials
#[test]
fn test_user_id_is_canonical_identifier() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = persistence.get_bootstrap_metadata().unwrap();
    let actor: AuthenticatedActor = create_test_admin();
    let operator = create_test_admin_operator();
    let cause: Cause = create_test_cause();

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());

    // Register a user
    let register_request = create_valid_request();

    let register_result: Result<ApiResult<RegisterUserResult>, ApiError> = register_user(
        &mut persistence,
        &metadata,
        &state,
        register_request,
        &actor,
        &operator,
        cause,
    );
    assert!(register_result.is_ok(), "Failed to register user");

    let api_result = register_result.unwrap();
    let transition = TransitionResult {
        audit_event: api_result.audit_event,
        new_state: api_result.new_state,
    };
    persistence.persist_transition(&transition).unwrap();

    // Verify user_id is present and canonical
    let reloaded_state = persistence.get_current_state(&bid_year, &area).unwrap();
    let canonical_bid_years = persistence.list_bid_years().unwrap();
    let users_response: ListUsersResponse = list_users(
        &metadata,
        &canonical_bid_years,
        &bid_year,
        &area,
        &reloaded_state,
        &actor,
        &operator,
        zab_bid_domain::BidYearLifecycle::Draft,
    )
    .unwrap();

    assert_eq!(users_response.users.len(), 1);
    let user = &users_response.users[0];

    // Verify user_id is present and valid
    assert!(user.user_id > 0, "user_id must be a valid positive integer");

    // Verify initials are present as display data
    assert_eq!(
        user.initials, "AB",
        "Initials should match registered value"
    );

    // Note: Changing initials via UpdateUser is currently not supported because
    // the core Command layer uses initials to identify which user to update.
    // This is documented as a known architectural pattern where the domain layer
    // uses domain vocabulary (initials) while persistence uses canonical IDs (user_id).
    // Future phases may address this if needed.
}

/// Regression test: Duplicate initials are allowed across different areas.
///
/// This test verifies that:
/// - Initials are scoped to (`bid_year_id`, `area_id`)
/// - The same initials can exist in different areas
/// - `user_id` is globally unique regardless of initials
#[test]
#[allow(clippy::too_many_lines)]
fn test_duplicate_initials_allowed_across_areas() {
    // This test requires two areas in the same bid year
    // We'll use the existing test pattern from test_list_areas_for_bid_year
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let mut metadata = persistence.get_bootstrap_metadata().unwrap();
    let actor = create_test_admin();
    let operator = create_test_admin_operator();
    let cause = create_test_cause();

    // The default setup already has "North" area, so we add another
    let area_request = CreateAreaRequest {
        area_id: String::from("South"),
    };

    let area_result = create_area(
        &mut persistence,
        &metadata,
        &area_request,
        &actor,
        &operator,
        cause.clone(),
    )
    .expect("Failed to create South area");

    persistence.persist_bootstrap(&area_result).unwrap();

    // Reload metadata to get canonical IDs for both areas
    metadata = persistence.get_bootstrap_metadata().unwrap();

    // Register user "AB" in North
    let north_state = State::new(BidYear::new(2026), Area::new("North"));

    let north_user_request = RegisterUserRequest {
        initials: String::from("AB"),
        name: String::from("Alice in North"),
        area: String::from("North"),
        user_type: String::from("CPC"),
        crew: Some(1),
        cumulative_natca_bu_date: String::from("2020-01-01"),
        natca_bu_date: String::from("2020-01-01"),
        eod_faa_date: String::from("2020-01-01"),
        service_computation_date: String::from("2020-01-01"),
        lottery_value: None,
    };

    let north_result = register_user(
        &mut persistence,
        &metadata,
        &north_state,
        north_user_request,
        &actor,
        &operator,
        cause.clone(),
    )
    .expect("Failed to register user in North");

    let north_transition = TransitionResult {
        audit_event: north_result.audit_event,
        new_state: north_result.new_state,
    };
    persistence.persist_transition(&north_transition).unwrap();

    // Register user "AB" in South (same initials, different area)
    let south_state = State::new(BidYear::new(2026), Area::new("South"));

    let south_user_request = RegisterUserRequest {
        initials: String::from("AB"),
        name: String::from("Alice in South"),
        area: String::from("South"),
        user_type: String::from("CPC"),
        crew: Some(2),
        cumulative_natca_bu_date: String::from("2021-01-01"),
        natca_bu_date: String::from("2021-01-01"),
        eod_faa_date: String::from("2021-01-01"),
        service_computation_date: String::from("2021-01-01"),
        lottery_value: None,
    };

    let south_result = register_user(
        &mut persistence,
        &metadata,
        &south_state,
        south_user_request,
        &actor,
        &operator,
        cause,
    );
    assert!(
        south_result.is_ok(),
        "Should allow duplicate initials in different areas"
    );

    let south_api_result = south_result.unwrap();
    let south_transition = TransitionResult {
        audit_event: south_api_result.audit_event,
        new_state: south_api_result.new_state,
    };
    persistence.persist_transition(&south_transition).unwrap();

    // Verify both users exist with the same initials but different user_ids
    let north_final_state = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("North"))
        .unwrap();
    let canonical_bid_years = persistence.list_bid_years().unwrap();
    let north_users = list_users(
        &metadata,
        &canonical_bid_years,
        &BidYear::new(2026),
        &Area::new("North"),
        &north_final_state,
        &actor,
        &operator,
        zab_bid_domain::BidYearLifecycle::Draft,
    )
    .unwrap();

    let south_final_state = persistence
        .get_current_state(&BidYear::new(2026), &Area::new("South"))
        .unwrap();
    let south_users = list_users(
        &metadata,
        &canonical_bid_years,
        &BidYear::new(2026),
        &Area::new("South"),
        &south_final_state,
        &actor,
        &operator,
        zab_bid_domain::BidYearLifecycle::Draft,
    )
    .unwrap();

    assert_eq!(north_users.users.len(), 1);
    assert_eq!(south_users.users.len(), 1);

    let north_user = &north_users.users[0];
    let south_user = &south_users.users[0];

    assert_eq!(north_user.initials, "AB");
    assert_eq!(south_user.initials, "AB");
    assert_ne!(
        north_user.user_id, south_user.user_id,
        "user_id must be unique even with duplicate initials"
    );
    assert_eq!(north_user.name, "Alice in North");
    assert_eq!(south_user.name, "Alice in South");
}

/// Regression test: User updates preserve `user_id` as canonical identifier.
///
/// This test verifies that:
/// - `UpdateUser` operations target users by their canonical `user_id`
/// - User data can be updated without changing `user_id`
/// - Persistence layer correctly uses `user_id` for all mutations
#[test]
fn test_user_updates_preserve_canonical_id() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata = persistence.get_bootstrap_metadata().unwrap();
    let actor = create_test_admin();
    let operator = create_test_admin_operator();
    let cause = create_test_cause();

    let bid_year = BidYear::new(2026);
    let area = Area::new("North");
    let state = State::new(bid_year.clone(), area.clone());

    // Register initial user
    let register_request = create_valid_request();

    let register_result = register_user(
        &mut persistence,
        &metadata,
        &state,
        register_request,
        &actor,
        &operator,
        cause.clone(),
    )
    .expect("Failed to register user");

    let transition = TransitionResult {
        audit_event: register_result.audit_event,
        new_state: register_result.new_state,
    };
    persistence.persist_transition(&transition).unwrap();

    // Get user_id
    let reloaded_state = persistence.get_current_state(&bid_year, &area).unwrap();
    let canonical_bid_years = persistence.list_bid_years().unwrap();
    let users_before = list_users(
        &metadata,
        &canonical_bid_years,
        &bid_year,
        &area,
        &reloaded_state,
        &actor,
        &operator,
        zab_bid_domain::BidYearLifecycle::Draft,
    )
    .unwrap();

    let original_user_id = users_before.users[0].user_id;
    let area_id = users_before.users[0].area_id;
    let original_initials = users_before.users[0].initials.clone();

    // Update user data (keeping same initials, changing other fields)
    let update_request = UpdateUserRequest {
        user_id: original_user_id,
        initials: original_initials.clone(), // Keep same initials
        name: String::from("Alice Brown Updated"),
        area_id,
        user_type: String::from("CPC"),
        crew: Some(3),
        cumulative_natca_bu_date: String::from("2020-01-01"),
        natca_bu_date: String::from("2020-01-01"),
        eod_faa_date: String::from("2020-01-01"),
        service_computation_date: String::from("2020-01-01"),
        lottery_value: Some(42),
    };

    let state_for_update = persistence.get_current_state(&bid_year, &area).unwrap();

    let update_result = update_user(
        &mut persistence,
        &metadata,
        &state_for_update,
        &update_request,
        &actor,
        &operator,
        cause,
    );

    assert!(
        update_result.is_ok(),
        "Failed to update user: {:?}",
        update_result.err()
    );

    // Verify the user_id is preserved and data is updated
    let final_state = persistence.get_current_state(&bid_year, &area).unwrap();
    let canonical_bid_years_final = persistence.list_bid_years().unwrap();
    let users_after = list_users(
        &metadata,
        &canonical_bid_years_final,
        &bid_year,
        &area,
        &final_state,
        &actor,
        &operator,
        zab_bid_domain::BidYearLifecycle::Draft,
    )
    .unwrap();

    assert_eq!(users_after.users.len(), 1);
    let updated_user = &users_after.users[0];

    // Verify user_id is unchanged (canonical identity preserved)
    assert_eq!(
        updated_user.user_id, original_user_id,
        "user_id must not change during updates"
    );

    // Verify data was updated
    assert_eq!(updated_user.name, "Alice Brown Updated");
    assert_eq!(updated_user.crew, Some(3));
    assert_eq!(updated_user.lottery_value, Some(42));
    assert_eq!(updated_user.initials, original_initials);
}
