// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Comprehensive API layer tests organized by behavior.

use zab_bid::{BootstrapMetadata, BootstrapResult, State, TransitionResult};
use zab_bid_audit::{Actor, Cause};
use zab_bid_domain::{Area, BidYear};

use crate::{
    ApiError, ApiResult, AuthError, AuthenticatedActor, CreateAreaRequest, CreateBidYearRequest,
    GetLeaveAvailabilityResponse, ListAreasRequest, ListAreasResponse, ListBidYearsResponse,
    ListUsersResponse, RegisterUserRequest, RegisterUserResponse, Role, checkpoint, create_area,
    create_bid_year, finalize, get_current_state, get_historical_state, get_leave_availability,
    list_areas, list_bid_years, list_users, register_user, rollback,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let bidder: AuthenticatedActor = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let bidder: AuthenticatedActor = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = checkpoint(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = checkpoint(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = finalize(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = finalize(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = rollback(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<TransitionResult, ApiError> = rollback(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let api_result: ApiResult<RegisterUserResponse> = result.unwrap();
    assert_eq!(api_result.audit_event.action.name, "RegisterUser");
    assert_eq!(api_result.audit_event.actor.id, "admin-123");
    assert_eq!(api_result.audit_event.actor.actor_type, "admin");
    assert_eq!(api_result.audit_event.cause.id, "api-req-456");
}

#[test]
fn test_valid_api_request_returns_new_state() {
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let api_result: ApiResult<RegisterUserResponse> = result.unwrap();
    assert_eq!(api_result.new_state.users.len(), 1);
    assert_eq!(api_result.new_state.users[0].initials.value(), "AB");
}

#[test]
fn test_duplicate_initials_returns_api_error() {
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let mut state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request1: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    // Register first user successfully
    let result1: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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

    let result2: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let mut state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request1: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    // Register first user successfully
    let result1: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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

    let result2: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
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

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
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

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
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

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
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

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
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
    let result1: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
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

    let result2: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state2,
        request2,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result2.is_ok());
    let api_result: ApiResult<RegisterUserResponse> = result2.unwrap();
    assert_eq!(api_result.new_state.users.len(), 1);
}

#[test]
fn test_successful_api_call_updates_state() {
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let state: State = State::new(BidYear::new(2026), Area::new("North"));
    let request: RegisterUserRequest = create_valid_request();
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    assert!(result.is_ok());
    let api_result: ApiResult<RegisterUserResponse> = result.unwrap();

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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));

    let request: CreateAreaRequest = CreateAreaRequest {
        area_id: String::from("North"),
    };
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, ApiError> = create_area(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));

    let request: CreateAreaRequest = CreateAreaRequest {
        area_id: String::from("North"),
    };
    let bidder: AuthenticatedActor = create_test_bidder();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, ApiError> = create_area(
        &persistence,
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let request: CreateAreaRequest = CreateAreaRequest {
        area_id: String::from("North"),
    };
    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, ApiError> = create_area(
        &persistence,
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
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> = Vec::new();
    let response: ListBidYearsResponse = list_bid_years(&canonical_bid_years).unwrap();

    assert_eq!(response.bid_years.len(), 0);
}

#[test]
fn test_list_bid_years_with_single_year() {
    let canonical = zab_bid_domain::CanonicalBidYear::new(
        2026,
        create_test_start_date(),
        create_test_pay_periods(),
    )
    .unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> = vec![canonical.clone()];

    let response: ListBidYearsResponse = list_bid_years(&canonical_bid_years).unwrap();

    assert_eq!(response.bid_years.len(), 1);
    assert_eq!(response.bid_years[0].year, 2026);
    assert_eq!(response.bid_years[0].start_date, canonical.start_date());
    assert_eq!(response.bid_years[0].num_pay_periods, 26);
    assert_eq!(
        response.bid_years[0].end_date,
        canonical.end_date().unwrap()
    );
}

#[test]
fn test_list_bid_years_with_multiple_years() {
    let canonical1 = zab_bid_domain::CanonicalBidYear::new(
        2026,
        create_test_start_date(),
        create_test_pay_periods(),
    )
    .unwrap();
    let canonical2 = zab_bid_domain::CanonicalBidYear::new(
        2027,
        create_test_start_date_for_year(2027),
        create_test_pay_periods(),
    )
    .unwrap();
    let canonical3 = zab_bid_domain::CanonicalBidYear::new(
        2028,
        create_test_start_date_for_year(2028),
        create_test_pay_periods(),
    )
    .unwrap();
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        vec![canonical1, canonical2, canonical3];

    let response: ListBidYearsResponse = list_bid_years(&canonical_bid_years).unwrap();

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
    let canonical_26 =
        zab_bid_domain::CanonicalBidYear::new(2026, create_test_start_date(), 26).unwrap();
    let canonical_27 =
        zab_bid_domain::CanonicalBidYear::new(2027, create_test_start_date_for_year(2027), 27)
            .unwrap();

    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        vec![canonical_26.clone(), canonical_27.clone()];

    let response: ListBidYearsResponse = list_bid_years(&canonical_bid_years).unwrap();

    assert_eq!(response.bid_years.len(), 2);

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
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));
    let request: ListAreasRequest = ListAreasRequest { bid_year: 2026 };

    let response: ListAreasResponse = list_areas(&metadata, &request).unwrap();

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
    let response: ListAreasResponse = list_areas(&metadata, &request).unwrap();

    assert_eq!(response.bid_year, 2026);
    assert_eq!(response.areas.len(), 2);
    assert!(
        response
            .areas
            .iter()
            .any(|a| a.area_id == "NORTH" && a.user_count == 0)
    );
    assert!(
        response
            .areas
            .iter()
            .any(|a| a.area_id == "SOUTH" && a.user_count == 0)
    );
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
    let response_2026: ListAreasResponse = list_areas(&metadata, &request_2026).unwrap();

    assert_eq!(response_2026.areas.len(), 1);
    assert_eq!(response_2026.areas[0].area_id, "NORTH");
    assert_eq!(response_2026.areas[0].user_count, 0);

    let request_2027: ListAreasRequest = ListAreasRequest { bid_year: 2027 };
    let response_2027: ListAreasResponse = list_areas(&metadata, &request_2027).unwrap();

    assert_eq!(response_2027.areas.len(), 1);
    assert_eq!(response_2027.areas[0].area_id, "SOUTH");
    assert_eq!(response_2027.areas[0].user_count, 0);
}

#[test]
fn test_list_areas_nonexistent_bid_year() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let request: ListAreasRequest = ListAreasRequest { bid_year: 9999 };

    let result = list_areas(&metadata, &request);

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
fn test_list_users_empty() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("North")));

    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        vec![create_test_canonical_bid_year()];
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());
    let response: ListUsersResponse =
        list_users(&metadata, &canonical_bid_years, &bid_year, &area, &state).unwrap();

    assert_eq!(response.bid_year, 2026);
    assert_eq!(response.area, "NORTH");
    assert_eq!(response.users.len(), 0);
}

#[test]
fn test_list_users_with_users() {
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
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

    let result1: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state,
        request1,
        &admin,
        &create_test_admin_operator(),
        cause.clone(),
    );
    assert!(result1.is_ok());

    let state_with_user1: State = result1.unwrap().new_state;

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

    let result2: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state_with_user1,
        request2,
        &admin,
        &create_test_admin_operator(),
        cause,
    );
    assert!(result2.is_ok());

    let final_state: State = result2.unwrap().new_state;
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        vec![create_test_canonical_bid_year()];
    let response: ListUsersResponse = list_users(
        &metadata,
        &canonical_bid_years,
        &bid_year,
        &area,
        &final_state,
    )
    .unwrap();

    assert_eq!(response.bid_year, 2026);
    assert_eq!(response.area, "NORTH");
    assert_eq!(response.users.len(), 2);

    let ab_user = response.users.iter().find(|u| u.initials == "AB").unwrap();
    assert_eq!(ab_user.name, "Alice Brown");
    assert_eq!(ab_user.crew, Some(1));
    assert_eq!(ab_user.user_type, "CPC");
    assert!(ab_user.earned_hours > 0);
    assert!(ab_user.earned_days > 0);
    assert_eq!(ab_user.remaining_hours, i32::from(ab_user.earned_hours));
    assert!(!ab_user.is_exhausted);
    assert!(!ab_user.is_overdrawn);

    let cd_user = response.users.iter().find(|u| u.initials == "CD").unwrap();
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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
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

    let result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );
    assert!(result.is_ok());

    let final_state: State = result.unwrap().new_state;
    let canonical_bid_years: Vec<zab_bid_domain::CanonicalBidYear> =
        vec![create_test_canonical_bid_year()];
    let response: ListUsersResponse = list_users(
        &metadata,
        &canonical_bid_years,
        &bid_year,
        &area,
        &final_state,
    )
    .unwrap();

    assert_eq!(response.users.len(), 1);
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

    let result = list_users(&metadata, &canonical_bid_years, &bid_year, &area, &state);

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

    let result = list_users(&metadata, &canonical_bid_years, &bid_year, &area, &state);

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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    // Create a canonical bid year
    let canonical_bid_year =
        zab_bid_domain::CanonicalBidYear::new(2026, create_test_start_date(), 26).unwrap();

    // Create a user with some service time
    let mut request: RegisterUserRequest = create_valid_request();
    request.service_computation_date = String::from("2020-01-15");

    // Create initial state
    let state: State = State::new(bid_year, area.clone());

    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let register_result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );
    assert!(register_result.is_ok());

    let new_state: State = register_result.unwrap().new_state;
    let initials = zab_bid_domain::Initials::new("AB");

    // Get leave availability
    let result: Result<GetLeaveAvailabilityResponse, ApiError> =
        get_leave_availability(&metadata, &canonical_bid_year, &area, &initials, &new_state);

    assert!(result.is_ok());
    let response: GetLeaveAvailabilityResponse = result.unwrap();

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
    let _persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let canonical_bid_year =
        zab_bid_domain::CanonicalBidYear::new(2026, create_test_start_date(), 26).unwrap();

    let state: State = State::new(bid_year, area.clone());

    let initials = zab_bid_domain::Initials::new("XY");

    let result: Result<GetLeaveAvailabilityResponse, ApiError> =
        get_leave_availability(&metadata, &canonical_bid_year, &area, &initials, &state);

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
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let canonical_bid_year =
        zab_bid_domain::CanonicalBidYear::new(2026, create_test_start_date(), 26).unwrap();

    let mut request: RegisterUserRequest = create_valid_request();
    request.service_computation_date = String::from("2024-01-15");

    let state: State = State::new(bid_year, area.clone());

    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    let register_result: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );
    assert!(register_result.is_ok());

    let new_state: State = register_result.unwrap().new_state;
    let initials = zab_bid_domain::Initials::new("AB");

    let result: Result<GetLeaveAvailabilityResponse, ApiError> =
        get_leave_availability(&metadata, &canonical_bid_year, &area, &initials, &new_state);

    assert!(result.is_ok());
    let response: GetLeaveAvailabilityResponse = result.unwrap();

    // Check that explanation contains key information
    assert!(response.explanation.contains("AB"));
    assert!(response.explanation.contains("2026"));
    assert!(response.explanation.contains("Earned:"));
    assert!(response.explanation.contains("Used:"));
    assert!(response.explanation.contains("Remaining:"));
}

#[test]
fn test_get_leave_availability_different_service_tiers() {
    let persistence = setup_test_persistence().expect("Failed to setup test persistence");
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let canonical_bid_year =
        zab_bid_domain::CanonicalBidYear::new(2026, create_test_start_date(), 26).unwrap();

    let state: State = State::new(bid_year, area.clone());

    let admin: AuthenticatedActor = create_test_admin();
    let cause: Cause = create_test_cause();

    // Test user with < 3 years service (4-hour tier)
    let mut request1: RegisterUserRequest = create_valid_request();
    request1.initials = String::from("U1");
    request1.service_computation_date = String::from("2024-01-15");

    let register_result1: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state,
        request1,
        &admin,
        &create_test_admin_operator(),
        cause.clone(),
    );
    assert!(register_result1.is_ok());

    let state1: State = register_result1.unwrap().new_state;
    let initials1 = zab_bid_domain::Initials::new("U1");

    let result1: Result<GetLeaveAvailabilityResponse, ApiError> =
        get_leave_availability(&metadata, &canonical_bid_year, &area, &initials1, &state1);
    assert!(result1.is_ok());
    let response1: GetLeaveAvailabilityResponse = result1.unwrap();

    // 26 PPs * 4 hours = 104 hours = 13 days
    assert_eq!(response1.earned_hours, 104);
    assert_eq!(response1.earned_days, 13);

    // Test user with 15+ years service (8-hour tier)
    let mut request2: RegisterUserRequest = create_valid_request();
    request2.initials = String::from("U2");
    request2.service_computation_date = String::from("2010-01-15");

    let register_result2: Result<ApiResult<RegisterUserResponse>, ApiError> = register_user(
        &persistence,
        &metadata,
        &state1,
        request2,
        &admin,
        &create_test_admin_operator(),
        cause,
    );
    assert!(register_result2.is_ok());

    let state2: State = register_result2.unwrap().new_state;
    let initials2 = zab_bid_domain::Initials::new("U2");

    let result2: Result<GetLeaveAvailabilityResponse, ApiError> =
        get_leave_availability(&metadata, &canonical_bid_year, &area, &initials2, &state2);
    assert!(result2.is_ok());
    let response2: GetLeaveAvailabilityResponse = result2.unwrap();

    // 26 PPs * 8 hours = 208 hours = 26 days
    assert_eq!(response2.earned_hours, 208);
    assert_eq!(response2.earned_days, 26);
}
