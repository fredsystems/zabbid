// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Leave availability calculation.
//!
//! This module provides read-only aggregation of leave accrual and usage
//! to compute available leave balances.

use crate::error::DomainError;
use crate::leave_accrual::LeaveAccrualResult;
use crate::types::{BidYear, Initials};
use serde::{Deserialize, Serialize};

/// Represents a single leave usage record.
///
/// Leave usage records represent hours consumed by a user within a bid year.
/// Usage records are:
/// - Additive
/// - Immutable once written
/// - Assumed valid for availability calculation purposes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaveUsage {
    /// The bid year this usage applies to.
    pub bid_year: BidYear,
    /// The user's initials.
    pub user_initials: Initials,
    /// Hours of leave used.
    pub hours_used: u16,
}

impl LeaveUsage {
    /// Creates a new `LeaveUsage`.
    ///
    /// # Arguments
    ///
    /// * `bid_year` - The bid year
    /// * `user_initials` - The user's initials
    /// * `hours_used` - Hours of leave used
    #[must_use]
    pub const fn new(bid_year: BidYear, user_initials: Initials, hours_used: u16) -> Self {
        Self {
            bid_year,
            user_initials,
            hours_used,
        }
    }
}

/// Result of leave availability calculation.
///
/// This represents the current leave balance for a user, combining
/// accrued leave (from Phase 9) with recorded usage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaveAvailabilityResult {
    /// Total hours earned (from Phase 9, post-rounding).
    pub earned_hours: u16,
    /// Total days earned (`earned_hours` / 8).
    pub earned_days: u16,
    /// Total hours used.
    pub used_hours: u16,
    /// Remaining hours available (`earned_hours` - `used_hours`).
    /// May be negative if overdrawn.
    pub remaining_hours: i32,
    /// Remaining days available (`remaining_hours` / 8).
    /// May be negative if overdrawn.
    pub remaining_days: i32,
    /// Whether all leave has been exhausted (`remaining_hours` == 0).
    pub is_exhausted: bool,
    /// Whether leave balance is overdrawn (`remaining_hours` < 0).
    pub is_overdrawn: bool,
}

/// Calculates leave availability for a user.
///
/// This function combines accrual results from Phase 9 with usage records
/// to produce a complete availability picture.
///
/// # Arguments
///
/// * `accrual` - The leave accrual result from Phase 9
/// * `usage_records` - Iterator of leave usage records for the user
///
/// # Returns
///
/// A `LeaveAvailabilityResult` containing earned, used, and remaining leave.
///
/// # Errors
///
/// Returns an error if usage records cannot be summed deterministically.
///
/// # Panics
///
/// This function does not panic.
pub fn calculate_leave_availability<I>(
    accrual: &LeaveAccrualResult,
    usage_records: I,
) -> Result<LeaveAvailabilityResult, DomainError>
where
    I: IntoIterator<Item = LeaveUsage>,
{
    // Sum all usage hours deterministically
    let used_hours: u16 = usage_records
        .into_iter()
        .fold(0_u16, |acc, record| acc.saturating_add(record.hours_used));

    // Calculate remaining hours (may be negative)
    let earned_hours_i32: i32 = i32::from(accrual.total_hours);
    let used_hours_i32: i32 = i32::from(used_hours);
    let remaining_hours: i32 = earned_hours_i32 - used_hours_i32;

    // Calculate remaining days (truncating division for consistency)
    let remaining_days: i32 = remaining_hours / 8;

    // Determine exhaustion and overdraw status
    let is_exhausted: bool = remaining_hours == 0;
    let is_overdrawn: bool = remaining_hours < 0;

    Ok(LeaveAvailabilityResult {
        earned_hours: accrual.total_hours,
        earned_days: accrual.total_days,
        used_hours,
        remaining_hours,
        remaining_days,
        is_exhausted,
        is_overdrawn,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::leave_accrual::{AccrualReason, PayPeriodAccrual};
    use time::Date;

    /// Helper to create a simple accrual result.
    fn make_accrual(total_hours: u16) -> LeaveAccrualResult {
        let total_days: u16 = total_hours / 8;
        let hours_u8: u8 = u8::try_from(total_hours).unwrap_or(255);
        LeaveAccrualResult {
            total_hours,
            total_days,
            rounded_up: false,
            breakdown: vec![PayPeriodAccrual {
                pay_period_index: Some(1),
                start_date: Some(Date::from_calendar_date(2026, time::Month::January, 4).unwrap()),
                end_date: Some(Date::from_calendar_date(2026, time::Month::January, 17).unwrap()),
                rate: hours_u8,
                hours: hours_u8,
                reason: AccrualReason::Normal,
            }],
        }
    }

    /// Helper to create a usage record.
    fn make_usage(hours: u16) -> LeaveUsage {
        LeaveUsage::new(BidYear::new(2026), Initials::new("AB"), hours)
    }

    #[test]
    fn test_zero_usage_full_availability() {
        let accrual: LeaveAccrualResult = make_accrual(160);
        let usage: Vec<LeaveUsage> = vec![];

        let result: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        assert_eq!(result.earned_hours, 160);
        assert_eq!(result.earned_days, 20);
        assert_eq!(result.used_hours, 0);
        assert_eq!(result.remaining_hours, 160);
        assert_eq!(result.remaining_days, 20);
        assert!(!result.is_exhausted);
        assert!(!result.is_overdrawn);
    }

    #[test]
    fn test_partial_usage_reduced_availability() {
        let accrual: LeaveAccrualResult = make_accrual(160);
        let usage: Vec<LeaveUsage> = vec![make_usage(24), make_usage(16)];

        let result: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        assert_eq!(result.earned_hours, 160);
        assert_eq!(result.earned_days, 20);
        assert_eq!(result.used_hours, 40);
        assert_eq!(result.remaining_hours, 120);
        assert_eq!(result.remaining_days, 15);
        assert!(!result.is_exhausted);
        assert!(!result.is_overdrawn);
    }

    #[test]
    fn test_full_usage_exhausted_balance() {
        let accrual: LeaveAccrualResult = make_accrual(160);
        let usage: Vec<LeaveUsage> = vec![make_usage(80), make_usage(80)];

        let result: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        assert_eq!(result.earned_hours, 160);
        assert_eq!(result.earned_days, 20);
        assert_eq!(result.used_hours, 160);
        assert_eq!(result.remaining_hours, 0);
        assert_eq!(result.remaining_days, 0);
        assert!(result.is_exhausted);
        assert!(!result.is_overdrawn);
    }

    #[test]
    fn test_overdrawn_usage_negative_balance() {
        let accrual: LeaveAccrualResult = make_accrual(160);
        let usage: Vec<LeaveUsage> = vec![make_usage(100), make_usage(80)];

        let result: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        assert_eq!(result.earned_hours, 160);
        assert_eq!(result.earned_days, 20);
        assert_eq!(result.used_hours, 180);
        assert_eq!(result.remaining_hours, -20);
        assert_eq!(result.remaining_days, -2);
        assert!(!result.is_exhausted);
        assert!(result.is_overdrawn);
    }

    #[test]
    fn test_deterministic_calculation() {
        let accrual: LeaveAccrualResult = make_accrual(208);
        let usage: Vec<LeaveUsage> = vec![make_usage(32), make_usage(16), make_usage(24)];

        let result1: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage.clone()).unwrap();
        let result2: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_no_rounding_after_subtraction() {
        // Accrual is already rounded to 8-hour days
        let accrual: LeaveAccrualResult = make_accrual(168);
        // Use an odd number of hours
        let usage: Vec<LeaveUsage> = vec![make_usage(35)];

        let result: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        // Remaining should be exactly 133 hours, not rounded
        assert_eq!(result.remaining_hours, 133);
        // Days uses truncating division
        assert_eq!(result.remaining_days, 16);
    }

    #[test]
    fn test_alignment_with_phase9_output() {
        // Use a realistic Phase 9 output
        let accrual: LeaveAccrualResult = LeaveAccrualResult {
            total_hours: 168,
            total_days: 21,
            rounded_up: true,
            breakdown: vec![],
        };

        let usage: Vec<LeaveUsage> = vec![make_usage(48)];

        let result: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        // Earned values match Phase 9 output exactly
        assert_eq!(result.earned_hours, accrual.total_hours);
        assert_eq!(result.earned_days, accrual.total_days);
        assert_eq!(result.remaining_hours, 120);
        assert_eq!(result.remaining_days, 15);
    }

    #[test]
    fn test_single_usage_record() {
        let accrual: LeaveAccrualResult = make_accrual(104);
        let usage: Vec<LeaveUsage> = vec![make_usage(8)];

        let result: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        assert_eq!(result.used_hours, 8);
        assert_eq!(result.remaining_hours, 96);
        assert_eq!(result.remaining_days, 12);
    }

    #[test]
    fn test_many_small_usage_records() {
        let accrual: LeaveAccrualResult = make_accrual(160);
        // Simulate many small usages
        let usage: Vec<LeaveUsage> = (0..20).map(|_| make_usage(4)).collect();

        let result: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        assert_eq!(result.used_hours, 80);
        assert_eq!(result.remaining_hours, 80);
        assert_eq!(result.remaining_days, 10);
    }

    #[test]
    fn test_exact_exhaustion_boundary() {
        let accrual: LeaveAccrualResult = make_accrual(64);
        let usage: Vec<LeaveUsage> = vec![make_usage(64)];

        let result: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        assert_eq!(result.remaining_hours, 0);
        assert!(result.is_exhausted);
        assert!(!result.is_overdrawn);
    }

    #[test]
    fn test_exact_overdraw_boundary() {
        let accrual: LeaveAccrualResult = make_accrual(64);
        let usage: Vec<LeaveUsage> = vec![make_usage(65)];

        let result: LeaveAvailabilityResult =
            calculate_leave_availability(&accrual, usage).unwrap();

        assert_eq!(result.remaining_hours, -1);
        assert!(!result.is_exhausted);
        assert!(result.is_overdrawn);
    }
}
