// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Canonical bid year validation for core transitions.
//!
//! This module provides validation-only construction of canonical bid years.
//! Canonical bid years are not persisted; they exist only to validate that
//! a bid year definition would be valid under canonical domain rules.
//!
//! # Placeholder Logic
//!
//! Since the API does not yet provide start dates or pay period counts,
//! this module uses explicit, deterministic placeholder values for validation:
//!
//! - **Start Date**: First Saturday of January in the given year
//! - **Pay Period Count**: Always 26 (the most common case)
//!
//! These placeholders are temporary and exist only in this module.
//! They will be removed when the API provides canonical bid year inputs.

use crate::error::CoreError;
use time::{Date, Month, Weekday};
use zab_bid_domain::CanonicalBidYear;

/// Validates that a bid year identifier could form a valid canonical bid year.
///
/// This function constructs a temporary `CanonicalBidYear` using placeholder
/// values for start date and pay period count. The canonical instance is used
/// only for validation and is immediately discarded.
///
/// # Placeholder Values
///
/// - **Start Date**: First Saturday of January in the given year
/// - **Pay Period Count**: 26
///
/// These are deterministic placeholders, not FAA policy assumptions.
///
/// # Arguments
///
/// * `year` - The year identifier to validate
///
/// # Returns
///
/// * `Ok(())` if the year could form a valid canonical bid year
/// * `Err(CoreError)` if canonical validation fails
///
/// # Errors
///
/// Returns an error if:
/// - The start date cannot be calculated
/// - The canonical bid year construction fails
/// - Any canonical invariant is violated
pub fn validate_canonical_bid_year(year: u16) -> Result<(), CoreError> {
    // Calculate placeholder start date: first Saturday of January
    let start_date: Date = first_saturday_of_january(year)?;

    // Use placeholder pay period count: 26 (most common)
    let pay_period_count: u8 = 26;

    // Attempt to construct canonical bid year (validation only)
    // The instance is immediately discarded after this validation
    let _canonical: CanonicalBidYear = CanonicalBidYear::new(year, start_date, pay_period_count)
        .map_err(CoreError::DomainViolation)?;

    // If construction succeeded, validation passed
    Ok(())
}

/// Calculates the first Saturday of January for a given year.
///
/// This is a deterministic placeholder calculation used only for validation.
///
/// # Arguments
///
/// * `year` - The year to calculate for
///
/// # Returns
///
/// The date of the first Saturday in January of the given year.
///
/// # Errors
///
/// Returns an error if date construction fails.
fn first_saturday_of_january(year: u16) -> Result<Date, CoreError> {
    // Start with January 1st
    let jan_1: Date = Date::from_calendar_date(i32::from(year), Month::January, 1)
        .map_err(|e| CoreError::Internal(format!("Failed to construct January 1st: {e}")))?;

    // Find the weekday of January 1st
    let weekday: Weekday = jan_1.weekday();

    // Calculate days until Saturday
    let days_until_saturday: u8 = match weekday {
        Weekday::Sunday => 6,
        Weekday::Monday => 5,
        Weekday::Tuesday => 4,
        Weekday::Wednesday => 3,
        Weekday::Thursday => 2,
        Weekday::Friday => 1,
        Weekday::Saturday => 0,
    };

    // Add days to get to the first Saturday
    let first_saturday: Date = jan_1
        .checked_add(time::Duration::days(i64::from(days_until_saturday)))
        .ok_or_else(|| CoreError::Internal(String::from("Date arithmetic overflow")))?;

    Ok(first_saturday)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_canonical_bid_year_succeeds_for_valid_year() {
        // 2026 should produce a valid canonical bid year with placeholder values
        let result: Result<(), CoreError> = validate_canonical_bid_year(2026);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_canonical_bid_year_succeeds_for_multiple_years() {
        // Test several different years
        for year in 2020..=2030 {
            let result: Result<(), CoreError> = validate_canonical_bid_year(year);
            assert!(result.is_ok(), "Year {year} should validate successfully");
        }
    }

    #[test]
    fn test_first_saturday_of_january_2026() {
        // January 1, 2026 is a Thursday
        // First Saturday should be January 3, 2026
        let result: Result<Date, CoreError> = first_saturday_of_january(2026);
        assert!(result.is_ok());
        let date: Date = result.unwrap();
        assert_eq!(date.year(), 2026);
        assert_eq!(date.month(), Month::January);
        assert_eq!(date.day(), 3);
        assert_eq!(date.weekday(), Weekday::Saturday);
    }

    #[test]
    fn test_first_saturday_of_january_2027() {
        // January 1, 2027 is a Friday
        // First Saturday should be January 2, 2027
        let result: Result<Date, CoreError> = first_saturday_of_january(2027);
        assert!(result.is_ok());
        let date: Date = result.unwrap();
        assert_eq!(date.year(), 2027);
        assert_eq!(date.month(), Month::January);
        assert_eq!(date.day(), 2);
        assert_eq!(date.weekday(), Weekday::Saturday);
    }

    #[test]
    fn test_first_saturday_of_january_2024() {
        // January 1, 2024 is a Monday
        // First Saturday should be January 6, 2024
        let result: Result<Date, CoreError> = first_saturday_of_january(2024);
        assert!(result.is_ok());
        let date: Date = result.unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), Month::January);
        assert_eq!(date.day(), 6);
        assert_eq!(date.weekday(), Weekday::Saturday);
    }

    #[test]
    fn test_first_saturday_when_january_1_is_saturday() {
        // January 1, 2022 is a Saturday
        // First Saturday should be January 1, 2022 itself
        let result: Result<Date, CoreError> = first_saturday_of_january(2022);
        assert!(result.is_ok());
        let date: Date = result.unwrap();
        assert_eq!(date.year(), 2022);
        assert_eq!(date.month(), Month::January);
        assert_eq!(date.day(), 1);
        assert_eq!(date.weekday(), Weekday::Saturday);
    }

    #[test]
    fn test_canonical_validation_is_deterministic() {
        // Calling validation multiple times should always succeed
        for _ in 0..5 {
            let result: Result<(), CoreError> = validate_canonical_bid_year(2026);
            assert!(result.is_ok());
        }
    }
}
