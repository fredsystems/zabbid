// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for lifecycle constraint violations (Phase 27H, Gap 3).
//!
//! These tests verify that invalid state transitions and wrong-state operations
//! are properly rejected with specific error kinds.

use crate::{BootstrapMetadata, BootstrapResult, Command, CoreError, apply_bootstrap};

use zab_bid_domain::{Area, BidYear, DomainError, validate_bid_year};

use super::helpers::{create_test_actor, create_test_cause};

/// Helper to create minimal bootstrap metadata with a bid year.
fn create_metadata_with_bid_year(year: u16) -> BootstrapMetadata {
    let mut metadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(year));
    metadata
}

/// Helper to create metadata with bid year and areas.
fn create_metadata_with_areas(year: u16, area_codes: &[&str]) -> BootstrapMetadata {
    let mut metadata = create_metadata_with_bid_year(year);
    let bid_year = BidYear::new(year);
    for &code in area_codes {
        metadata.areas.push((bid_year.clone(), Area::new(code)));
    }
    metadata
}

// ============================================================================
// Invalid State Transition Tests
// ============================================================================

#[test]
fn test_validate_bid_year_rejects_year_zero() {
    let result = validate_bid_year(0);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        DomainError::InvalidBidYear(_)
    ));
}

#[test]
fn test_validate_bid_year_rejects_year_too_low() {
    let result = validate_bid_year(1899);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, DomainError::InvalidBidYear(_)));
}

#[test]
fn test_validate_bid_year_rejects_year_too_high() {
    let result = validate_bid_year(2201);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, DomainError::InvalidBidYear(_)));
}

#[test]
fn test_validate_bid_year_accepts_minimum_valid_year() {
    let result = validate_bid_year(2000);
    assert!(result.is_ok());
}

#[test]
fn test_validate_bid_year_accepts_maximum_valid_year() {
    let result = validate_bid_year(2200);
    assert!(result.is_ok());
}

#[test]
fn test_validate_bid_year_accepts_current_era_year() {
    let result = validate_bid_year(2026);
    assert!(result.is_ok());
}

// ============================================================================
// Duplicate Bid Year Tests
// ============================================================================

#[test]
fn test_create_bid_year_rejects_duplicate() {
    let metadata = create_metadata_with_bid_year(2026);
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::CreateBidYear {
        year: 2026,
        start_date: time::Date::from_calendar_date(2026, time::Month::January, 4).unwrap(),
        num_pay_periods: 26,
    };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::DuplicateBidYear(2026))
    ));
}

#[test]
fn test_create_bid_year_succeeds_for_different_year() {
    let metadata = create_metadata_with_bid_year(2026);
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::CreateBidYear {
        year: 2027,
        start_date: time::Date::from_calendar_date(2027, time::Month::January, 3).unwrap(),
        num_pay_periods: 26,
    };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert!(
        bootstrap_result
            .new_metadata
            .has_bid_year(&BidYear::new(2027))
    );
}

// ============================================================================
// Duplicate Area Tests
// ============================================================================

#[test]
fn test_create_area_rejects_duplicate() {
    let metadata = create_metadata_with_areas(2026, &["NORTH"]);
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::CreateArea {
        area_id: String::from("North"),
    };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::DuplicateArea { bid_year: 2026, .. })
    ));
}

#[test]
fn test_create_area_succeeds_for_different_area() {
    let metadata = create_metadata_with_areas(2026, &["NORTH"]);
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::CreateArea {
        area_id: String::from("South"),
    };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_ok());
}

#[test]
fn test_create_area_succeeds_for_same_code_different_bid_year() {
    let mut metadata = create_metadata_with_bid_year(2026);
    metadata.bid_years.push(BidYear::new(2027));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("North")));

    let bid_year = BidYear::new(2027);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::CreateArea {
        area_id: String::from("North"),
    };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_ok());
}

// ============================================================================
// Missing Bid Year Tests
// ============================================================================

#[test]
fn test_create_area_fails_for_nonexistent_bid_year() {
    let metadata = BootstrapMetadata::new();
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::CreateArea {
        area_id: String::from("North"),
    };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::BidYearNotFound(2026))
    ));
}

#[test]
fn test_set_active_bid_year_fails_for_nonexistent_year() {
    let metadata = BootstrapMetadata::new();
    let active_bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::SetActiveBidYear { year: 2026 };

    let result = apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::BidYearNotFound(2026))
    ));
}

#[test]
fn test_transition_to_bootstrap_complete_fails_for_nonexistent_year() {
    let metadata = BootstrapMetadata::new();
    let active_bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::TransitionToBootstrapComplete { year: 2026 };

    let result = apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::BidYearNotFound(2026))
    ));
}

#[test]
fn test_transition_to_canonicalized_fails_for_nonexistent_year() {
    let metadata = BootstrapMetadata::new();
    let active_bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::TransitionToCanonicalized { year: 2026 };

    let result = apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::BidYearNotFound(2026))
    ));
}

#[test]
fn test_transition_to_bidding_active_fails_for_nonexistent_year() {
    let metadata = BootstrapMetadata::new();
    let active_bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::TransitionToBiddingActive { year: 2026 };

    let result = apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::BidYearNotFound(2026))
    ));
}

#[test]
fn test_transition_to_bidding_closed_fails_for_nonexistent_year() {
    let metadata = BootstrapMetadata::new();
    let active_bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::TransitionToBiddingClosed { year: 2026 };

    let result = apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::BidYearNotFound(2026))
    ));
}

// ============================================================================
// Invalid Expected Count Tests
// ============================================================================

#[test]
fn test_set_expected_area_count_rejects_zero() {
    let metadata = create_metadata_with_bid_year(2026);
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::SetExpectedAreaCount { expected_count: 0 };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::InvalidExpectedAreaCount { count: 0 })
    ));
}

#[test]
fn test_set_expected_area_count_accepts_positive() {
    let metadata = create_metadata_with_bid_year(2026);
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::SetExpectedAreaCount { expected_count: 5 };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_ok());
}

#[test]
fn test_set_expected_user_count_rejects_zero() {
    let metadata = create_metadata_with_areas(2026, &["NORTH"]);
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::SetExpectedUserCount {
        area: Area::new("North"),
        expected_count: 0,
    };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::InvalidExpectedUserCount { count: 0 })
    ));
}

#[test]
fn test_set_expected_user_count_accepts_positive() {
    let metadata = create_metadata_with_areas(2026, &["NORTH"]);
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::SetExpectedUserCount {
        area: Area::new("North"),
        expected_count: 10,
    };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_ok());
}

#[test]
fn test_set_expected_user_count_fails_for_nonexistent_area() {
    let metadata = create_metadata_with_bid_year(2026);
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::SetExpectedUserCount {
        area: Area::new("Nonexistent"),
        expected_count: 10,
    };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::AreaNotFound { bid_year: 2026, .. })
    ));
}

#[test]
fn test_set_expected_user_count_fails_for_nonexistent_bid_year() {
    let metadata = BootstrapMetadata::new();
    let bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::SetExpectedUserCount {
        area: Area::new("North"),
        expected_count: 10,
    };

    let result = apply_bootstrap(&metadata, &bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::BidYearNotFound(2026))
    ));
}

// ============================================================================
// Lifecycle State Transition Tests
// ============================================================================

#[test]
fn test_set_active_bid_year_succeeds_for_existing_year() {
    let metadata = create_metadata_with_bid_year(2026);
    let active_bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::SetActiveBidYear { year: 2026 };

    let result = apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
}

#[test]
fn test_transition_to_bootstrap_complete_succeeds_for_existing_year() {
    let metadata = create_metadata_with_bid_year(2026);
    let active_bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::TransitionToBootstrapComplete { year: 2026 };

    let result = apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
}

#[test]
fn test_transition_to_canonicalized_succeeds_for_existing_year() {
    let metadata = create_metadata_with_bid_year(2026);
    let active_bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::TransitionToCanonicalized { year: 2026 };

    let result = apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
}

#[test]
fn test_transition_to_bidding_active_succeeds_for_existing_year() {
    let metadata = create_metadata_with_bid_year(2026);
    let active_bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::TransitionToBiddingActive { year: 2026 };

    let result = apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
}

#[test]
fn test_transition_to_bidding_closed_succeeds_for_existing_year() {
    let metadata = create_metadata_with_bid_year(2026);
    let active_bid_year = BidYear::new(2026);
    let actor = create_test_actor();
    let cause = create_test_cause();

    let command = Command::TransitionToBiddingClosed { year: 2026 };

    let result = apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
}
