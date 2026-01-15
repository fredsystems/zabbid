// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use crate::tests::helpers::{
    create_test_actor, create_test_cause, create_test_pay_periods, create_test_start_date,
    create_test_start_date_for_year,
};
use crate::{BootstrapMetadata, BootstrapResult, Command, CoreError, apply_bootstrap};
use zab_bid_audit::{Actor, Cause};
use zab_bid_domain::{Area, BidYear, DomainError};

#[test]
fn test_create_bid_year_succeeds() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert_eq!(bootstrap_result.new_metadata.bid_years.len(), 1);
    assert_eq!(bootstrap_result.new_metadata.bid_years[0].year(), 2026);
}

#[test]
fn test_create_bid_year_emits_audit_event() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

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

    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CoreError::DomainViolation(DomainError::DuplicateBidYear(2026))
    ));
}

#[test]
fn test_create_invalid_bid_year_fails() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear {
        year: 1800,
        start_date: create_test_start_date_for_year(1800),
        num_pay_periods: create_test_pay_periods(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(1800);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

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
        area_id: String::from("North"),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let active_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

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
        area_id: String::from("North"),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let active_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

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
    assert_eq!(
        bootstrap_result
            .audit_event
            .bid_year
            .as_ref()
            .unwrap()
            .year(),
        2026
    );
    assert_eq!(
        bootstrap_result.audit_event.area.as_ref().unwrap().id(),
        "NORTH"
    );
}

#[test]
fn test_create_area_without_bid_year_fails() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let active_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

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
        area_id: String::from("North"),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let active_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

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

    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    }; // Duplicate
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let active_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_err());
    // Metadata should remain unchanged
    assert_eq!(metadata.bid_years.len(), 1);
    assert_eq!(metadata.areas.len(), 0);
}

#[test]
fn test_multiple_bid_years_and_areas() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Create first bid year
    let command1: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let placeholder_bid_year_2026 = BidYear::new(2026);
    let result1: Result<BootstrapResult, CoreError> = apply_bootstrap(
        &metadata,
        &placeholder_bid_year_2026,
        command1,
        create_test_actor(),
        create_test_cause(),
    );
    assert!(result1.is_ok());
    metadata = result1.unwrap().new_metadata;

    // Create second bid year
    let command2: Command = Command::CreateBidYear {
        year: 2027,
        start_date: create_test_start_date_for_year(2027),
        num_pay_periods: create_test_pay_periods(),
    };
    let placeholder_bid_year_2027 = BidYear::new(2027);
    let result2: Result<BootstrapResult, CoreError> = apply_bootstrap(
        &metadata,
        &placeholder_bid_year_2027,
        command2,
        create_test_actor(),
        create_test_cause(),
    );
    assert!(result2.is_ok());
    metadata = result2.unwrap().new_metadata;

    assert_eq!(metadata.bid_years.len(), 2);

    // Create areas in different bid years
    let command3: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let active_bid_year_2026 = BidYear::new(2026);
    let result3: Result<BootstrapResult, CoreError> = apply_bootstrap(
        &metadata,
        &active_bid_year_2026,
        command3,
        create_test_actor(),
        create_test_cause(),
    );
    assert!(result3.is_ok());
    metadata = result3.unwrap().new_metadata;

    let command4: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let active_bid_year_2027 = BidYear::new(2027);
    let result4: Result<BootstrapResult, CoreError> = apply_bootstrap(
        &metadata,
        &active_bid_year_2027,
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

#[test]
fn test_canonical_validation_runs_for_valid_year() {
    // This test verifies that canonical validation is executed
    // Valid years should pass canonical validation
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

    // Should succeed - canonical validation passed
    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert_eq!(bootstrap_result.new_metadata.bid_years.len(), 1);
}

#[test]
fn test_canonical_validation_does_not_persist_canonical_model() {
    // This test verifies that only the simple BidYear identifier is persisted,
    // not the canonical bid year model
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();

    // Only the simple BidYear identifier is stored
    assert_eq!(bootstrap_result.new_metadata.bid_years.len(), 1);
    assert_eq!(bootstrap_result.new_metadata.bid_years[0].year(), 2026);

    // The stored type is BidYear (identifier), not CanonicalBidYear
    // This is verified by the fact that BidYear::new() only takes a year
}

#[test]
fn test_canonical_validation_failure_prevents_creation() {
    // This test would verify that canonical validation failures block creation,
    // but with our placeholder logic (first Saturday + 26 periods),
    // all reasonable years pass validation.
    // This test documents the behavior for future phases when real validation occurs.
    let metadata: BootstrapMetadata = BootstrapMetadata::new();

    // With current placeholder logic, all valid years (2000-2099) pass
    for year in 2020..=2030 {
        let command: Command = Command::CreateBidYear {
            year,
            start_date: create_test_start_date_for_year(i32::from(year)),
            num_pay_periods: create_test_pay_periods(),
        };
        let placeholder_bid_year = BidYear::new(year);
        let result: Result<BootstrapResult, CoreError> = apply_bootstrap(
            &metadata,
            &placeholder_bid_year,
            command,
            create_test_actor(),
            create_test_cause(),
        );
        assert!(
            result.is_ok(),
            "Year {year} should pass canonical validation"
        );
    }
}

#[test]
fn test_canonical_validation_no_audit_event_on_failure() {
    // This test verifies that validation failures do not emit audit events
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear {
        year: 1800,
        start_date: create_test_start_date_for_year(1800),
        num_pay_periods: create_test_pay_periods(),
    }; // Invalid year
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

    // Should fail before reaching audit event creation
    assert!(result.is_err());

    // No BootstrapResult means no audit event was created
    // The error is returned before any state change or audit event emission
}

#[test]
fn test_canonical_validation_is_deterministic() {
    // Canonical validation should be deterministic - same input, same output
    let metadata: BootstrapMetadata = BootstrapMetadata::new();

    for _ in 0..5 {
        let command: Command = Command::CreateBidYear {
            year: 2026,
            start_date: create_test_start_date(),
            num_pay_periods: create_test_pay_periods(),
        };
        let placeholder_bid_year = BidYear::new(2026);
        let result: Result<BootstrapResult, CoreError> = apply_bootstrap(
            &metadata,
            &placeholder_bid_year,
            command.clone(),
            create_test_actor(),
            create_test_cause(),
        );

        // First attempt succeeds
        assert!(result.is_ok());

        // Subsequent attempts fail due to duplicate, not canonical validation
        let metadata_with_2026: BootstrapMetadata = result.unwrap().new_metadata;
        let duplicate_command: Command = Command::CreateBidYear {
            year: 2026,
            start_date: create_test_start_date(),
            num_pay_periods: create_test_pay_periods(),
        };
        let duplicate_result: Result<BootstrapResult, CoreError> = apply_bootstrap(
            &metadata_with_2026,
            &placeholder_bid_year,
            duplicate_command,
            create_test_actor(),
            create_test_cause(),
        );

        assert!(matches!(
            duplicate_result.unwrap_err(),
            CoreError::DomainViolation(DomainError::DuplicateBidYear(_))
        ));
    }
}

#[test]
fn test_create_bid_year_with_26_pay_periods_succeeds() {
    // Test that bid years with 26 pay periods are accepted
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: 26,
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert_eq!(bootstrap_result.new_metadata.bid_years.len(), 1);
    assert!(bootstrap_result.canonical_bid_year.is_some());
    assert_eq!(
        bootstrap_result
            .canonical_bid_year
            .unwrap()
            .num_pay_periods(),
        26
    );
}

#[test]
fn test_create_bid_year_with_27_pay_periods_succeeds() {
    // Test that bid years with 27 pay periods are accepted
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: 27,
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();
    assert_eq!(bootstrap_result.new_metadata.bid_years.len(), 1);
    assert!(bootstrap_result.canonical_bid_year.is_some());
    assert_eq!(
        bootstrap_result
            .canonical_bid_year
            .unwrap()
            .num_pay_periods(),
        27
    );
}

#[test]
fn test_create_bid_year_with_invalid_pay_periods_fails() {
    // Test that invalid pay period counts are rejected
    let metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Test various invalid counts
    for invalid_count in [0, 1, 25, 28, 52, 255] {
        let command: Command = Command::CreateBidYear {
            year: 2026,
            start_date: create_test_start_date(),
            num_pay_periods: invalid_count,
        };
        let actor: Actor = create_test_actor();
        let cause: Cause = create_test_cause();

        let placeholder_bid_year = BidYear::new(2026);
        let result: Result<BootstrapResult, CoreError> =
            apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

        assert!(
            result.is_err(),
            "Pay period count {invalid_count} should be rejected"
        );
        assert!(
            matches!(result.unwrap_err(), CoreError::DomainViolation(_)),
            "Should fail with domain violation for invalid pay periods"
        );
    }
}

#[test]
fn test_canonical_metadata_persisted_on_success() {
    // Test that canonical metadata is included in BootstrapResult
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let start_date = create_test_start_date();
    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date,
        num_pay_periods: 26,
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();

    // Canonical metadata should be present
    assert!(bootstrap_result.canonical_bid_year.is_some());
    let canonical = bootstrap_result.canonical_bid_year.unwrap();
    assert_eq!(canonical.year(), 2026);
    assert_eq!(canonical.start_date(), start_date);
    assert_eq!(canonical.num_pay_periods(), 26);
}

#[test]
fn test_canonical_metadata_not_included_for_non_bid_year_operations() {
    // Test that canonical_bid_year is None for operations other than CreateBidYear
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.add_bid_year(BidYear::new(2026));

    let command: Command = Command::CreateArea {
        area_id: String::from("North"),
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let active_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &active_bid_year, command, actor, cause);

    assert!(result.is_ok());
    let bootstrap_result: BootstrapResult = result.unwrap();

    // canonical_bid_year should be None for non-CreateBidYear operations
    assert!(bootstrap_result.canonical_bid_year.is_none());
}

#[test]
fn test_no_audit_event_on_canonical_validation_failure() {
    // Verify that canonical validation failures prevent audit event creation
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let command: Command = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: 25, // Invalid - must be 26 or 27
    };
    let actor: Actor = create_test_actor();
    let cause: Cause = create_test_cause();

    let placeholder_bid_year = BidYear::new(2026);
    let result: Result<BootstrapResult, CoreError> =
        apply_bootstrap(&metadata, &placeholder_bid_year, command, actor, cause);

    // Should fail validation
    assert!(result.is_err());
    // No BootstrapResult means no audit event was created
}
