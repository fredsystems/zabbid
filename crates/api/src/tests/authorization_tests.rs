// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Authorization failure tests - Gap 1 remediation.
//!
//! Tests that admin-only endpoints correctly reject bidder access.

use zab_bid::BootstrapMetadata;

use crate::{
    ApiError, CreateAreaRequest, CreateBidYearRequest, SetActiveBidYearRequest,
    TransitionToBiddingActiveRequest, TransitionToBiddingClosedRequest,
    TransitionToBootstrapCompleteRequest, TransitionToCanonicalizedRequest, UpdateAreaRequest,
    UpdateBidYearMetadataRequest, UpdateUserRequest, checkpoint, create_area, create_bid_year,
    finalize, rollback, set_active_bid_year, transition_to_bidding_active,
    transition_to_bidding_closed, transition_to_bootstrap_complete, transition_to_canonicalized,
    update_area, update_bid_year_metadata, update_user,
};

use super::helpers::{
    create_test_bidder, create_test_bidder_operator, create_test_cause, create_test_pay_periods,
    create_test_start_date, setup_test_persistence,
};

// ============================================================================
// Gap 1: Handler Authorization Failures (Bidder Rejection)
// ============================================================================

#[test]
fn test_update_user_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    // Get the bootstrapped bid year and area (2026, North from setup)
    let bid_year = metadata.bid_years.first().expect("Bid year not found");
    let area = metadata
        .areas
        .first()
        .map(|(_, a)| a)
        .expect("Area not found");
    let area_id = area.area_id().expect("Area ID not found");

    let state = persistence
        .get_current_state(bid_year, area)
        .expect("Failed to get current state");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause = create_test_cause();

    let request = UpdateUserRequest {
        user_id: 1,
        initials: String::from("AB"),
        name: String::from("Test User"),
        area_id,
        user_type: String::from("CPC"),
        crew: Some(1),
        cumulative_natca_bu_date: String::from("2020-01-01"),
        natca_bu_date: String::from("2020-01-01"),
        eod_faa_date: String::from("2020-01-01"),
        service_computation_date: String::from("2020-01-01"),
        lottery_value: None,
    };

    let result = update_user(
        &mut persistence,
        &metadata,
        &state,
        &request,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized {
            action,
            required_role,
        } => {
            assert_eq!(action, "register_user");
            assert_eq!(required_role, "Admin");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_update_area_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let area = metadata
        .areas
        .first()
        .map(|(_, a)| a)
        .expect("Area not found");
    let area_id = area.area_id().expect("Area ID not found");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();

    let request = UpdateAreaRequest {
        area_id,
        area_name: Some(String::from("Updated Area")),
    };

    let result = update_area(&mut persistence, &metadata, &request, &bidder, &operator);

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized {
            action,
            required_role,
        } => {
            assert_eq!(action, "update_area");
            assert_eq!(required_role, "Admin");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_update_bid_year_metadata_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let bid_year = metadata.bid_years.first().expect("Bid year not found");
    let bid_year_id = bid_year.bid_year_id().expect("Bid year ID not found");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();

    let request = UpdateBidYearMetadataRequest {
        bid_year_id,
        label: Some(String::from("Updated Label")),
        notes: Some(String::from("Updated Notes")),
    };

    let cause = create_test_cause();

    let result = update_bid_year_metadata(
        &mut persistence,
        &metadata,
        &request,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized {
            action,
            required_role,
        } => {
            assert_eq!(action, "update bid year metadata");
            assert_eq!(required_role, "Admin");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_set_active_bid_year_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let bid_year = metadata.bid_years.first().expect("Bid year not found");
    let bid_year_id = bid_year.bid_year_id().expect("Bid year ID not found");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();

    let cause = create_test_cause();

    let request = SetActiveBidYearRequest { bid_year_id };

    let result = set_active_bid_year(
        &mut persistence,
        &metadata,
        &request,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized {
            action,
            required_role,
        } => {
            assert_eq!(action, "set_active_bid_year");
            assert_eq!(required_role, "Admin");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_transition_to_bootstrap_complete_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let bid_year = metadata.bid_years.first().expect("Bid year not found");
    let bid_year_id = bid_year.bid_year_id().expect("Bid year ID not found");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause = create_test_cause();

    let request = TransitionToBootstrapCompleteRequest { bid_year_id };

    let result = transition_to_bootstrap_complete(
        &mut persistence,
        &metadata,
        &request,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized {
            action,
            required_role,
        } => {
            assert_eq!(action, "transition_to_bootstrap_complete");
            assert_eq!(required_role, "Admin");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_transition_to_canonicalized_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let bid_year = metadata.bid_years.first().expect("Bid year not found");
    let bid_year_id = bid_year.bid_year_id().expect("Bid year ID not found");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause = create_test_cause();

    let request = TransitionToCanonicalizedRequest { bid_year_id };

    let result = transition_to_canonicalized(
        &mut persistence,
        &metadata,
        &request,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized {
            action,
            required_role,
        } => {
            assert_eq!(action, "transition_to_canonicalized");
            assert_eq!(required_role, "Admin");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_transition_to_bidding_active_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let bid_year = metadata.bid_years.first().expect("Bid year not found");
    let bid_year_id = bid_year.bid_year_id().expect("Bid year ID not found");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause = create_test_cause();

    let request = TransitionToBiddingActiveRequest { bid_year_id };

    let result = transition_to_bidding_active(
        &mut persistence,
        &metadata,
        &request,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized {
            action,
            required_role,
        } => {
            assert_eq!(action, "transition_to_bidding_active");
            assert_eq!(required_role, "Admin");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_transition_to_bidding_closed_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let bid_year = metadata.bid_years.first().expect("Bid year not found");
    let bid_year_id = bid_year.bid_year_id().expect("Bid year ID not found");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause = create_test_cause();

    let request = TransitionToBiddingClosedRequest { bid_year_id };

    let result = transition_to_bidding_closed(
        &mut persistence,
        &metadata,
        &request,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::Unauthorized {
            action,
            required_role,
        } => {
            assert_eq!(action, "transition_to_bidding_closed");
            assert_eq!(required_role, "Admin");
        }
        other => panic!("Expected Unauthorized error, got: {other:?}"),
    }
}

#[test]
fn test_checkpoint_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let bid_year = metadata.bid_years.first().expect("Bid year not found");
    let area = metadata
        .areas
        .first()
        .map(|(_, a)| a)
        .expect("Area not found");

    let state = persistence
        .get_current_state(bid_year, area)
        .expect("Failed to get current state");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause = create_test_cause();

    let result = checkpoint(
        &mut persistence,
        &metadata,
        &state,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ApiError::Unauthorized { .. }),
        "Expected Unauthorized, got: {err:?}"
    );
}

#[test]
fn test_finalize_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let bid_year = metadata.bid_years.first().expect("Bid year not found");
    let area = metadata
        .areas
        .first()
        .map(|(_, a)| a)
        .expect("Area not found");

    let state = persistence
        .get_current_state(bid_year, area)
        .expect("Failed to get current state");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause = create_test_cause();

    let result = finalize(
        &mut persistence,
        &metadata,
        &state,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ApiError::Unauthorized { .. }),
        "Expected Unauthorized, got: {err:?}"
    );
}

#[test]
fn test_rollback_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let bid_year = metadata.bid_years.first().expect("Bid year not found");
    let area = metadata
        .areas
        .first()
        .map(|(_, a)| a)
        .expect("Area not found");

    let state = persistence
        .get_current_state(bid_year, area)
        .expect("Failed to get current state");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause = create_test_cause();

    let result = rollback(
        &mut persistence,
        &metadata,
        &state,
        1,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ApiError::Unauthorized { .. }),
        "Expected Unauthorized, got: {err:?}"
    );
}

#[test]
fn test_create_bid_year_rejects_bidder() {
    let metadata = BootstrapMetadata::new();

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause = create_test_cause();

    let request = CreateBidYearRequest {
        year: 2027,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };

    let result = create_bid_year(&metadata, &request, &bidder, &operator, cause);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ApiError::Unauthorized { .. }),
        "Expected Unauthorized, got: {err:?}"
    );
}

#[test]
fn test_create_area_rejects_bidder() {
    let mut persistence = setup_test_persistence().expect("Failed to setup test persistence");

    let metadata = persistence
        .get_bootstrap_metadata()
        .expect("Failed to get metadata");

    let bidder = create_test_bidder();
    let operator = create_test_bidder_operator();
    let cause = create_test_cause();

    let request = CreateAreaRequest {
        area_id: String::from("SOUTH"),
    };

    let result = create_area(
        &mut persistence,
        &metadata,
        &request,
        &bidder,
        &operator,
        cause,
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ApiError::Unauthorized { .. }),
        "Expected Unauthorized, got: {err:?}"
    );
}
