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
    ListAreasRequest, ListAreasResponse, ListBidYearsResponse, ListUsersResponse,
    RegisterUserRequest, RegisterUserResponse, Role, authenticate_stub, checkpoint, create_area,
    create_bid_year, finalize, get_current_state, get_historical_state, list_areas, list_bid_years,
    list_users, register_user, rollback,
};

use super::helpers::{
    create_test_admin, create_test_bidder, create_test_cause, create_test_metadata,
    create_valid_request,
};

// ============================================================================
// Authentication Tests
// ============================================================================

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

    let result: Result<TransitionResult, ApiError> = checkpoint(&metadata, &state, &admin, cause);

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

    let result: Result<TransitionResult, ApiError> = checkpoint(&metadata, &state, &bidder, cause);

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

    let result: Result<TransitionResult, ApiError> = finalize(&metadata, &state, &bidder, cause);

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

    let result: Result<TransitionResult, ApiError> = rollback(&metadata, &state, 1, &admin, cause);

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

    let result: Result<TransitionResult, ApiError> = rollback(&metadata, &state, 1, &bidder, cause);

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
    // Empty area won't exist in metadata, so we get ResourceNotFound
    assert!(matches!(err, ApiError::ResourceNotFound { .. }));
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

// ============================================================================
// Bootstrap Tests (Bid Year and Area Creation)
// ============================================================================

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

    let result: Result<BootstrapResult, ApiError> = create_area(&metadata, request, &admin, cause);

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

    let result: Result<BootstrapResult, ApiError> = create_area(&metadata, request, &bidder, cause);

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

    let result: Result<BootstrapResult, ApiError> = create_area(&metadata, request, &admin, cause);

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
    let response_2026: ListAreasResponse = list_areas(&metadata, &request_2026).unwrap();

    assert_eq!(response_2026.areas.len(), 1);
    assert_eq!(response_2026.areas[0], "NORTH");

    let request_2027: ListAreasRequest = ListAreasRequest { bid_year: 2027 };
    let response_2027: ListAreasResponse = list_areas(&metadata, &request_2027).unwrap();

    assert_eq!(response_2027.areas.len(), 1);
    assert_eq!(response_2027.areas[0], "SOUTH");
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

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());
    let response: ListUsersResponse = list_users(&metadata, &bid_year, &area, &state).unwrap();

    assert_eq!(response.bid_year, 2026);
    assert_eq!(response.area, "NORTH");
    assert_eq!(response.users.len(), 0);
}

#[test]
fn test_list_users_with_users() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());
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
    let response: ListUsersResponse =
        list_users(&metadata, &bid_year, &area, &final_state).unwrap();

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
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());
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
    let response: ListUsersResponse =
        list_users(&metadata, &bid_year, &area, &final_state).unwrap();

    assert_eq!(response.users.len(), 1);
    assert_eq!(response.users[0].initials, "EF");
    assert_eq!(response.users[0].name, "Eve Foster");
    assert_eq!(response.users[0].crew, None);
}

#[test]
fn test_list_users_nonexistent_bid_year() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let bid_year: BidYear = BidYear::new(9999);
    let area: Area = Area::new("North");
    let state: State = State::new(bid_year.clone(), area.clone());

    let result = list_users(&metadata, &bid_year, &area, &state);

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

    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("NonExistent");
    let state: State = State::new(bid_year.clone(), area.clone());

    let result = list_users(&metadata, &bid_year, &area, &state);

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
