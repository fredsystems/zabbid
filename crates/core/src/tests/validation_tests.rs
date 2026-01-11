// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for read-only validation functions.

use crate::{BootstrapMetadata, validate_area_exists, validate_bid_year_exists};
use zab_bid_domain::{Area, BidYear, DomainError};

use super::helpers::create_test_metadata;

#[test]
fn test_validate_bid_year_exists_succeeds() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(2026);

    let result: Result<(), DomainError> = validate_bid_year_exists(&metadata, &bid_year);

    assert!(result.is_ok());
}

#[test]
fn test_validate_bid_year_exists_fails_for_nonexistent_year() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(9999);

    let result: Result<(), DomainError> = validate_bid_year_exists(&metadata, &bid_year);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        DomainError::BidYearNotFound(9999)
    ));
}

#[test]
fn test_validate_bid_year_exists_fails_for_empty_metadata() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let bid_year: BidYear = BidYear::new(2026);

    let result: Result<(), DomainError> = validate_bid_year_exists(&metadata, &bid_year);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        DomainError::BidYearNotFound(2026)
    ));
}

#[test]
fn test_validate_area_exists_succeeds() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let result: Result<(), DomainError> = validate_area_exists(&metadata, &bid_year, &area);

    assert!(result.is_ok());
}

#[test]
fn test_validate_area_exists_fails_for_nonexistent_area() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("NonExistent");

    let result: Result<(), DomainError> = validate_area_exists(&metadata, &bid_year, &area);

    assert!(result.is_err());
    let err: DomainError = result.unwrap_err();
    assert!(matches!(err, DomainError::AreaNotFound { .. }));
    if let DomainError::AreaNotFound { bid_year, area } = err {
        assert_eq!(bid_year, 2026);
        assert_eq!(area, "NONEXISTENT");
    }
}

#[test]
fn test_validate_area_exists_fails_for_nonexistent_bid_year() {
    let metadata: BootstrapMetadata = create_test_metadata();
    let bid_year: BidYear = BidYear::new(9999);
    let area: Area = Area::new("North");

    let result: Result<(), DomainError> = validate_area_exists(&metadata, &bid_year, &area);

    assert!(result.is_err());
    // Should fail on bid year check first
    assert!(matches!(
        result.unwrap_err(),
        DomainError::BidYearNotFound(9999)
    ));
}

#[test]
fn test_validate_area_exists_fails_for_empty_metadata() {
    let metadata: BootstrapMetadata = BootstrapMetadata::new();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");

    let result: Result<(), DomainError> = validate_area_exists(&metadata, &bid_year, &area);

    assert!(result.is_err());
    // Should fail on bid year check first
    assert!(matches!(
        result.unwrap_err(),
        DomainError::BidYearNotFound(2026)
    ));
}

#[test]
fn test_validate_area_exists_with_multiple_bid_years() {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));
    metadata.bid_years.push(BidYear::new(2027));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("North")));
    metadata
        .areas
        .push((BidYear::new(2027), Area::new("South")));

    // Area exists in 2026
    let result1: Result<(), DomainError> =
        validate_area_exists(&metadata, &BidYear::new(2026), &Area::new("North"));
    assert!(result1.is_ok());

    // Area exists in 2027
    let result2: Result<(), DomainError> =
        validate_area_exists(&metadata, &BidYear::new(2027), &Area::new("South"));
    assert!(result2.is_ok());

    // Area from 2026 doesn't exist in 2027
    let result3: Result<(), DomainError> =
        validate_area_exists(&metadata, &BidYear::new(2027), &Area::new("North"));
    assert!(result3.is_err());
    assert!(matches!(
        result3.unwrap_err(),
        DomainError::AreaNotFound { .. }
    ));

    // Area from 2027 doesn't exist in 2026
    let result4: Result<(), DomainError> =
        validate_area_exists(&metadata, &BidYear::new(2026), &Area::new("South"));
    assert!(result4.is_err());
    assert!(matches!(
        result4.unwrap_err(),
        DomainError::AreaNotFound { .. }
    ));
}
