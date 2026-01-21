// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for lifecycle enforcement of editing locks post-confirmation.
//!
//! These tests verify that structural changes (area creation, user registration,
//! participation flag updates, round configuration) are blocked after a bid year
//! transitions to `Canonicalized` state.

use zab_bid::{BootstrapMetadata, State};
use zab_bid_domain::{Area, BidYear};
use zab_bid_persistence::SqlitePersistence;

use crate::{
    ApiError, AuthenticatedActor, CreateAreaRequest, RegisterUserRequest, Role,
    UpdateUserParticipationRequest, create_area, register_user, update_user_participation,
};

use super::helpers::{
    bootstrap_with_ids, create_test_admin, create_test_admin_operator, create_test_cause,
};

/// Test that area creation is blocked after `Canonicalized` state.
#[test]
fn test_area_creation_blocked_after_canonicalized() {
    let mut persistence =
        SqlitePersistence::new_in_memory().expect("Failed to create in-memory persistence");

    // Create test operator
    let operator_id = persistence
        .create_operator("test-admin", "Test Admin", "password", "Admin")
        .expect("Failed to create operator");

    // Bootstrap with a bid year and area
    let ids = bootstrap_with_ids(&mut persistence, 2026, "TestArea", operator_id)
        .expect("Failed to bootstrap");

    // Set as active bid year
    persistence
        .set_active_bid_year(&BidYear::new(2026))
        .expect("Failed to set active bid year");

    // Transition to Canonicalized state
    persistence
        .update_lifecycle_state(ids.bid_year_id, "Canonicalized")
        .expect("Failed to set lifecycle state");

    // Construct metadata with bid year that has an ID
    let mut metadata = BootstrapMetadata::new();
    let bid_year = BidYear::with_id(ids.bid_year_id, 2026);
    metadata.bid_years.push(bid_year);

    let request = CreateAreaRequest {
        area_id: String::from("NewArea"),
    };
    let admin = create_test_admin();
    let cause = create_test_cause();

    // Attempt to create area - should fail
    let result = create_area(
        &mut persistence,
        &metadata,
        &request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    // Verify the operation is blocked
    assert!(result.is_err());
    match result {
        Err(ApiError::DomainRuleViolation { rule, message }) => {
            assert_eq!(rule, "area_creation_lifecycle");
            assert!(message.contains("structural changes locked"));
            assert!(message.contains("Canonicalized"));
        }
        Err(e) => panic!("Expected DomainRuleViolation error, got: {e:?}"),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

/// Test that user registration is blocked after `Canonicalized` state.
#[test]
fn test_user_registration_blocked_after_canonicalized() {
    let mut persistence =
        SqlitePersistence::new_in_memory().expect("Failed to create in-memory persistence");

    // Create test operator
    let operator_id = persistence
        .create_operator("test-admin", "Test Admin", "password", "Admin")
        .expect("Failed to create operator");

    // Bootstrap with a bid year and area
    let ids = bootstrap_with_ids(&mut persistence, 2026, "TestArea", operator_id)
        .expect("Failed to bootstrap");

    // Set as active bid year
    persistence
        .set_active_bid_year(&BidYear::new(2026))
        .expect("Failed to set active bid year");

    // Transition to Canonicalized state
    persistence
        .update_lifecycle_state(ids.bid_year_id, "Canonicalized")
        .expect("Failed to set lifecycle state");

    // Construct metadata with bid year that has an ID
    let mut metadata = BootstrapMetadata::new();
    let bid_year = BidYear::with_id(ids.bid_year_id, 2026);
    metadata.bid_years.push(bid_year);
    let state = State::new(BidYear::new(2026), Area::new("TestArea"));

    let request = RegisterUserRequest {
        initials: String::from("AB"),
        name: String::from("Test User"),
        area: String::from("TestArea"),
        user_type: String::from("Member"),
        crew: Some(1),
        cumulative_natca_bu_date: String::from("2020-01-01"),
        natca_bu_date: String::from("2020-01-01"),
        eod_faa_date: String::from("2020-01-01"),
        service_computation_date: String::from("2020-01-01"),
        lottery_value: None,
    };
    let admin = create_test_admin();
    let cause = create_test_cause();

    // Attempt to register user - should fail
    let result = register_user(
        &mut persistence,
        &metadata,
        &state,
        request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    // Verify the operation is blocked
    assert!(result.is_err());
    match result {
        Err(ApiError::DomainRuleViolation { rule, message }) => {
            assert_eq!(rule, "user_registration_lifecycle");
            assert!(message.contains("structural changes locked"));
            assert!(message.contains("Canonicalized"));
        }
        Err(e) => panic!("Expected DomainRuleViolation error, got: {e:?}"),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

/// Test that participation flag updates are blocked after `Canonicalized` state.
#[test]
fn test_participation_flag_updates_blocked_after_canonicalized() {
    let mut persistence =
        SqlitePersistence::new_in_memory().expect("Failed to create in-memory persistence");

    // Create test operator
    let operator_id = persistence
        .create_operator("test-admin", "Test Admin", "password", "Admin")
        .expect("Failed to create operator");

    // Bootstrap with a bid year and area
    let ids = bootstrap_with_ids(&mut persistence, 2026, "TestArea", operator_id)
        .expect("Failed to bootstrap");

    // Set as active bid year
    persistence
        .set_active_bid_year(&BidYear::new(2026))
        .expect("Failed to set active bid year");

    // Transition to Canonicalized state
    persistence
        .update_lifecycle_state(ids.bid_year_id, "Canonicalized")
        .expect("Failed to set lifecycle state");

    // Construct metadata with bid year that has an ID
    let mut metadata = BootstrapMetadata::new();
    let bid_year = BidYear::with_id(ids.bid_year_id, 2026);
    metadata.bid_years.push(bid_year);

    let request = UpdateUserParticipationRequest {
        user_id: 1,
        excluded_from_bidding: true,
        excluded_from_leave_calculation: true,
    };

    let admin = AuthenticatedActor::new(String::from("admin-1"), Role::Admin);
    let admin_actor = admin.to_audit_actor(&create_test_admin_operator());
    let lifecycle_state = "Canonicalized"
        .parse::<zab_bid_domain::BidYearLifecycle>()
        .expect("Failed to parse lifecycle state");

    // Attempt to update participation flags - should fail
    let result = update_user_participation(
        &metadata,
        &mut persistence,
        &request,
        &admin_actor,
        lifecycle_state,
    );

    // Verify the operation is blocked
    assert!(result.is_err());
    match result {
        Err(ApiError::DomainRuleViolation { rule, message }) => {
            assert_eq!(rule, "participation_flags_lifecycle");
            assert!(message.contains("structural changes locked"));
            assert!(message.contains("Canonicalized"));
        }
        Err(e) => panic!("Expected DomainRuleViolation error, got: {e:?}"),
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

/// Test that area creation is allowed in `Draft` state.
#[test]
fn test_area_creation_allowed_in_draft() {
    let mut persistence =
        SqlitePersistence::new_in_memory().expect("Failed to create in-memory persistence");

    // Create test operator
    let operator_id = persistence
        .create_operator("test-admin", "Test Admin", "password", "Admin")
        .expect("Failed to create operator");

    // Bootstrap with a bid year and area
    let ids = bootstrap_with_ids(&mut persistence, 2026, "TestArea", operator_id)
        .expect("Failed to bootstrap");

    // Set as active bid year
    persistence
        .set_active_bid_year(&BidYear::new(2026))
        .expect("Failed to set active bid year");

    // Verify state is `Draft` (default)
    let lifecycle_state = persistence
        .get_lifecycle_state(ids.bid_year_id)
        .expect("Failed to get lifecycle state");
    assert_eq!(lifecycle_state, "Draft");

    // Construct metadata with bid year that has an ID
    let mut metadata = BootstrapMetadata::new();
    let bid_year = BidYear::with_id(ids.bid_year_id, 2026);
    metadata.bid_years.push(bid_year);

    let request = CreateAreaRequest {
        area_id: String::from("NewArea"),
    };
    let admin = create_test_admin();
    let cause = create_test_cause();

    // Attempt to create area - should succeed (Draft state allows)
    let result = create_area(
        &mut persistence,
        &metadata,
        &request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    // Verify the operation is allowed in `Draft` state
    assert!(result.is_ok());
}

/// Test that area creation is allowed in `BootstrapComplete` state.
#[test]
fn test_area_creation_allowed_in_bootstrap_complete() {
    let mut persistence =
        SqlitePersistence::new_in_memory().expect("Failed to create in-memory persistence");

    // Create test operator
    let operator_id = persistence
        .create_operator("test-admin", "Test Admin", "password", "Admin")
        .expect("Failed to create operator");

    // Bootstrap with a bid year and area
    let ids = bootstrap_with_ids(&mut persistence, 2026, "TestArea", operator_id)
        .expect("Failed to bootstrap");

    // Set as active bid year
    persistence
        .set_active_bid_year(&BidYear::new(2026))
        .expect("Failed to set active bid year");

    // Transition to `BootstrapComplete` state
    persistence
        .update_lifecycle_state(ids.bid_year_id, "BootstrapComplete")
        .expect("Failed to set lifecycle state");

    // Construct metadata with bid year that has an ID
    let mut metadata = BootstrapMetadata::new();
    let bid_year = BidYear::with_id(ids.bid_year_id, 2026);
    metadata.bid_years.push(bid_year);

    let request = CreateAreaRequest {
        area_id: String::from("NewArea"),
    };
    let admin = create_test_admin();
    let cause = create_test_cause();

    // Attempt to create area - should succeed (BootstrapComplete allows)
    let result = create_area(
        &mut persistence,
        &metadata,
        &request,
        &admin,
        &create_test_admin_operator(),
        cause,
    );

    // Verify the operation is allowed in `BootstrapComplete` state
    assert!(result.is_ok());
}
