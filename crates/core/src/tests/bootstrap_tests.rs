// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::tests::helpers::{create_test_actor, create_test_cause};
use crate::{BootstrapMetadata, BootstrapResult, Command, CoreError, apply_bootstrap};
use zab_bid_audit::{Actor, Cause};
use zab_bid_domain::{Area, BidYear, DomainError};

#[test]
fn test_create_bid_year_succeeds() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear { year: 2026 };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert_eq!(bootstrap_result.new_metadata.bid_years.len(), 1);
    assert_eq!(bootstrap_result.new_metadata.bid_years[0].year(), 2026);
}

#[test]
fn test_create_bid_year_emits_audit_event() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear { year: 2026 };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert_eq!(bootstrap_result.audit_event.action.name, "CreateBidYear");
    assert!(
        bootstrap_result
            .audit_event
            .action
            .details
            .as_ref()
            .unwrap()
            .contains("2026")
    );
}

#[test]
fn test_create_duplicate_bid_year_fails() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.add_bid_year(BidYear::new(2026));

    let command: Command = Command::CreateBidYear { year: 2026 };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::DuplicateBidYear(2026))
    ));
}

#[test]
fn test_create_invalid_bid_year_fails() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear { year: 1800 };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::InvalidBidYear(_))
    ));
}

#[test]
fn test_create_area_succeeds() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.add_bid_year(BidYear::new(2026));

    let command: Command = Command::CreateArea {
        bid_year: BidYear::new(2026),
        area_id: String::from("North"),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert_eq!(bootstrap_result.new_metadata.areas.len(), 1);
    assert_eq!(bootstrap_result.new_metadata.areas[0].0.year(), 2026);
    assert_eq!(bootstrap_result.new_metadata.areas[0].1.id(), "NORTH");
}

#[test]
fn test_create_area_emits_audit_event() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.add_bid_year(BidYear::new(2026));

    let command: Command = Command::CreateArea {
        bid_year: BidYear::new(2026),
        area_id: String::from("North"),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert_eq!(bootstrap_result.audit_event.action.name, "CreateArea");
    assert!(
        bootstrap_result
            .audit_event
            .action
            .details
            .as_ref()
            .unwrap()
            .contains("NORTH")
    );
    assert_eq!(bootstrap_result.audit_event.bid_year.year(), 2026);
    assert_eq!(bootstrap_result.audit_event.area.id(), "NORTH");
}

#[test]
fn test_create_area_without_bid_year_fails() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateArea {
        bid_year: BidYear::new(2026),
        area_id: String::from("North"),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::BidYearNotFound(2026))
    ));
}

#[test]
fn test_create_duplicate_area_fails() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.add_bid_year(BidYear::new(2026));
    metadata.add_area(BidYear::new(2026), Area::new("North"));

    let command: Command = Command::CreateArea {
        bid_year: BidYear::new(2026),
        area_id: String::from("North"),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::DuplicateArea { .. })
    ));
}

#[test]
fn test_bootstrap_does_not_mutate_on_failure() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.add_bid_year(BidYear::new(2026));

    let command: Command = Command::CreateBidYear { year: 2026 }; // Duplicate
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, command, actor, cause);

    assert!(result.is_err());
    // Metadata should remain unchanged
    assert_eq!(metadata.bid_years.len(), 1);
    assert_eq!(metadata.areas.len(), 0);
}

#[test]
fn test_multiple_bid_years_and_areas() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Create first bid year
    let command1: Command = Command::CreateBidYear { year: 2026 };
    let result1: Result<BootstrapResult, CoreError> = apply_bootstrap(
        &metadata,
        command1,
        create_test_actor(),
        create_test_cause(),
    );
    assert!(result1.is_ok());
    metadata = result1.unwrap().new_metadata;

    // Create second bid year
    let command2: Command = Command::CreateBidYear { year: 2027 };
    let result2: Result<BootstrapResult, CoreError> = apply_bootstrap(
        &metadata,
        command2,
        create_test_actor(),
        create_test_cause(),
    );
    assert!(result2.is_ok());
    metadata = result2.unwrap().new_metadata;

    assert_eq!(metadata.bid_years.len(), 2);

    // Create areas in different bid years
    let command3: Command = Command::CreateArea {
        bid_year: BidYear::new(2026),
        area_id: String::from("North"),
    };
    let result3: Result<BootstrapResult, CoreError> = apply_bootstrap(
        &metadata,
        command3,
        create_test_actor(),
        create_test_cause(),
    );
    assert!(result3.is_ok());
    metadata = result3.unwrap().new_metadata;

    let command4: Command = Command::CreateArea {
        bid_year: BidYear::new(2027),
        area_id: String::from("North"),
    };
    let result4: Result<BootstrapResult, CoreError> = apply_bootstrap(
        &metadata,
        command4,
        create_test_actor(),
        create_test_cause(),
    );
    assert!(result4.is_ok());
    metadata = result4.unwrap().new_metadata;

    assert_eq!(metadata.areas.len(), 2);
    assert!(metadata.has_area(&BidYear::new(2026), &Area::new("North")));
    assert!(metadata.has_area(&BidYear::new(2027), &Area::new("North")));
}
