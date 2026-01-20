// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Leave accrual calculation for a single user within a single bid year.
//!
//! This module provides pure, deterministic leave accrual calculations
//! based on anniversary-based service thresholds, pay period logic,
//! and explicit rounding rules.

use crate::bid_year::{CanonicalBidYear, PayPeriod};
use crate::error::DomainError;
use crate::types::User;
use serde::{Deserialize, Serialize};
use time::Date;

/// Reason for a specific accrual entry in the breakdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccrualReason {
    /// Normal pay period accrual.
    Normal,
    /// Pay period immediately following a service threshold transition.
    Transition,
    /// The 27th pay period in a 27-PP year.
    TwentySeventhPP,
    /// Bonus hours for the 6-hour tier.
    Bonus,
    /// Rounding adjustment to reach a full 8-hour day multiple.
    RoundingAdjustment,
}

/// A single entry in the leave accrual breakdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayPeriodAccrual {
    /// Pay period index (1-based). None for bonus and rounding entries.
    pub pay_period_index: Option<u8>,
    /// Pay period start date. None for bonus and rounding entries.
    pub start_date: Option<Date>,
    /// Pay period end date. None for bonus and rounding entries.
    pub end_date: Option<Date>,
    /// Accrual rate applied (hours per pay period).
    pub rate: u8,
    /// Hours earned in this entry.
    pub hours: u8,
    /// Reason for this accrual entry.
    pub reason: AccrualReason,
}

/// Result of leave accrual calculation for a single user and bid year.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeaveAccrualResult {
    /// Total accrued hours (after rounding).
    pub total_hours: u16,
    /// Total accrued days (`total_hours` / 8).
    pub total_days: u16,
    /// Whether rounding was applied.
    pub rounded_up: bool,
    /// Detailed breakdown explaining how the total was reached.
    pub breakdown: Vec<PayPeriodAccrual>,
}

/// Calculates leave accrual for a single user within a single bid year.
///
/// This is a pure, deterministic calculation that:
/// - Uses anniversary-based service thresholds
/// - Evaluates thresholds at pay period start dates
/// - Applies the prior rate when a threshold is crossed mid-pay-period
/// - Adds bonus hours for the 6-hour tier
/// - Rounds up to the next multiple of 8 if needed
///
/// # Arguments
///
/// * `user` - The user to calculate accrual for
/// * `bid_year` - The canonical bid year
///
/// # Returns
///
/// A rich `LeaveAccrualResult` containing total hours, days, and a detailed breakdown.
///
/// # Errors
///
/// Returns an error if:
/// - The service computation date is missing or invalid
/// - Date parsing fails
/// - Pay period derivation fails
/// - Date arithmetic overflows
pub fn calculate_leave_accrual(
    user: &User,
    bid_year: &CanonicalBidYear,
) -> Result<LeaveAccrualResult, DomainError> {
    // Parse the service computation date
    let scd: Date = parse_service_computation_date(user)?;

    // Get all pay periods
    let pay_periods: Vec<PayPeriod> = bid_year.pay_periods()?;

    // Calculate accrual for each pay period
    let mut breakdown: Vec<PayPeriodAccrual> = Vec::new();
    let mut total_hours: u16 = 0;
    let mut applied_bonus: bool = false;

    for (idx, period) in pay_periods.iter().enumerate() {
        let years_of_service: u16 = calculate_years_of_service(scd, period.start_date());
        let rate: u8 = determine_accrual_rate(years_of_service);

        // Determine the reason
        let reason: AccrualReason = if period.index() == 27 {
            AccrualReason::TwentySeventhPP
        } else if idx > 0 {
            // Check if this is immediately after a transition
            let prev_period: &PayPeriod = &pay_periods[idx - 1];
            let prev_years: u16 = calculate_years_of_service(scd, prev_period.start_date());
            let prev_rate: u8 = determine_accrual_rate(prev_years);

            if rate == prev_rate {
                AccrualReason::Normal
            } else {
                AccrualReason::Transition
            }
        } else {
            AccrualReason::Normal
        };

        breakdown.push(PayPeriodAccrual {
            pay_period_index: Some(period.index()),
            start_date: Some(period.start_date()),
            end_date: Some(period.end_date()),
            rate,
            hours: rate,
            reason,
        });

        total_hours += u16::from(rate);

        // Apply bonus hours if in the 6-hour tier and not yet applied
        if rate == 6 && !applied_bonus {
            breakdown.push(PayPeriodAccrual {
                pay_period_index: None,
                start_date: None,
                end_date: None,
                rate: 0,
                hours: 4,
                reason: AccrualReason::Bonus,
            });
            total_hours += 4;
            applied_bonus = true;
        }
    }

    // Apply rounding if needed
    let rounded_up: bool = !total_hours.is_multiple_of(8);
    if rounded_up {
        let remainder: u16 = total_hours % 8;
        let adjustment: u16 = 8 - remainder;

        breakdown.push(PayPeriodAccrual {
            pay_period_index: None,
            start_date: None,
            end_date: None,
            rate: 0,
            hours: u8::try_from(adjustment).unwrap_or(0),
            reason: AccrualReason::RoundingAdjustment,
        });

        total_hours += adjustment;
    }

    let total_days: u16 = total_hours / 8;

    Ok(LeaveAccrualResult {
        total_hours,
        total_days,
        rounded_up,
        breakdown,
    })
}

/// Parses the service computation date from a user's seniority data.
///
/// # Arguments
///
/// * `user` - The user whose SCD to parse
///
/// # Returns
///
/// The parsed `Date`.
///
/// # Errors
///
/// Returns an error if the SCD is empty or fails to parse.
fn parse_service_computation_date(user: &User) -> Result<Date, DomainError> {
    let scd_string: &str = &user.seniority_data.service_computation_date;

    if scd_string.is_empty() {
        return Err(DomainError::InvalidServiceComputationDate {
            reason: "Service computation date is empty".to_string(),
        });
    }

    Date::parse(
        scd_string,
        &time::format_description::well_known::Iso8601::DEFAULT,
    )
    .map_err(|e| DomainError::DateParseError {
        date_string: scd_string.to_string(),
        error: e.to_string(),
    })
}

/// Calculates the number of complete years of service between the SCD and a given date.
///
/// This uses anniversary-based logic: years are counted only when the full
/// calendar anniversary has been reached or passed.
///
/// # Arguments
///
/// * `scd` - The service computation date
/// * `as_of` - The date to calculate service as of
///
/// # Returns
///
/// The number of complete years of service.
fn calculate_years_of_service(scd: Date, as_of: Date) -> u16 {
    if as_of < scd {
        return 0;
    }

    let scd_year: i32 = scd.year();
    let as_of_year: i32 = as_of.year();
    let years_diff: i32 = as_of_year - scd_year;

    // Check if the anniversary has occurred in the as_of year
    let scd_month: time::Month = scd.month();
    let scd_day: u8 = scd.day();
    let as_of_month: time::Month = as_of.month();
    let as_of_day: u8 = as_of.day();

    let anniversary_reached: bool =
        (as_of_month > scd_month) || (as_of_month == scd_month && as_of_day >= scd_day);

    if anniversary_reached {
        u16::try_from(years_diff).unwrap_or(0)
    } else {
        u16::try_from((years_diff - 1).max(0)).unwrap_or(0)
    }
}

/// Determines the accrual rate based on years of service.
///
/// # Arguments
///
/// * `years_of_service` - The number of complete years of service
///
/// # Returns
///
/// The accrual rate in hours per pay period.
const fn determine_accrual_rate(years_of_service: u16) -> u8 {
    if years_of_service < 3 {
        4
    } else if years_of_service < 15 {
        6
    } else {
        8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Area, BidYear, Crew, Initials, SeniorityData, UserType};
    use time::macros::date;

    fn make_user(scd: &str) -> User {
        User::new(
            BidYear::new(2026),
            Initials::new("TS"),
            "Test User".to_string(),
            Area::new("North"),
            UserType::CPC,
            Some(Crew::new(1).unwrap()),
            SeniorityData::new(
                "2020-01-01".to_string(),
                "2020-01-01".to_string(),
                "2020-01-01".to_string(),
                scd.to_string(),
                None,
            ),
            false, // excluded_from_bidding
            false, // excluded_from_leave_calculation
        )
    }

    fn make_bid_year_26pp() -> CanonicalBidYear {
        CanonicalBidYear::new(2026, date!(2026 - 01 - 04), 26).unwrap()
    }

    fn make_bid_year_27pp() -> CanonicalBidYear {
        CanonicalBidYear::new(2026, date!(2026 - 01 - 04), 27).unwrap()
    }

    #[test]
    fn test_calculate_years_of_service_same_day() {
        let scd: Date = date!(2020 - 03 - 15);
        let as_of: Date = date!(2020 - 03 - 15);
        assert_eq!(calculate_years_of_service(scd, as_of), 0);
    }

    #[test]
    fn test_calculate_years_of_service_before_first_anniversary() {
        let scd: Date = date!(2020 - 03 - 15);
        let as_of: Date = date!(2021 - 03 - 14);
        assert_eq!(calculate_years_of_service(scd, as_of), 0);
    }

    #[test]
    fn test_calculate_years_of_service_on_first_anniversary() {
        let scd: Date = date!(2020 - 03 - 15);
        let as_of: Date = date!(2021 - 03 - 15);
        assert_eq!(calculate_years_of_service(scd, as_of), 1);
    }

    #[test]
    fn test_calculate_years_of_service_after_first_anniversary() {
        let scd: Date = date!(2020 - 03 - 15);
        let as_of: Date = date!(2021 - 03 - 16);
        assert_eq!(calculate_years_of_service(scd, as_of), 1);
    }

    #[test]
    fn test_calculate_years_of_service_multiple_years() {
        let scd: Date = date!(2020 - 03 - 15);
        let as_of: Date = date!(2025 - 03 - 15);
        assert_eq!(calculate_years_of_service(scd, as_of), 5);
    }

    #[test]
    fn test_calculate_years_of_service_before_scd() {
        let scd: Date = date!(2025 - 03 - 15);
        let as_of: Date = date!(2020 - 03 - 15);
        assert_eq!(calculate_years_of_service(scd, as_of), 0);
    }

    #[test]
    fn test_determine_accrual_rate_under_3_years() {
        assert_eq!(determine_accrual_rate(0), 4);
        assert_eq!(determine_accrual_rate(1), 4);
        assert_eq!(determine_accrual_rate(2), 4);
    }

    #[test]
    fn test_determine_accrual_rate_3_to_14_years() {
        assert_eq!(determine_accrual_rate(3), 6);
        assert_eq!(determine_accrual_rate(10), 6);
        assert_eq!(determine_accrual_rate(14), 6);
    }

    #[test]
    fn test_determine_accrual_rate_15_plus_years() {
        assert_eq!(determine_accrual_rate(15), 8);
        assert_eq!(determine_accrual_rate(20), 8);
        assert_eq!(determine_accrual_rate(30), 8);
    }

    #[test]
    fn test_accrual_user_under_3_years_26pp() {
        let user: User = make_user("2024-01-01");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // 26 PPs * 4 hours = 104 hours = 13 days (no rounding needed)
        assert_eq!(result.total_hours, 104);
        assert_eq!(result.total_days, 13);
        assert!(!result.rounded_up);
        assert_eq!(result.breakdown.len(), 26); // 26 PPs, no bonus, no rounding
    }

    #[test]
    fn test_accrual_user_3_to_14_years_26pp() {
        let user: User = make_user("2020-01-01");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // 26 PPs * 6 hours + 4 bonus = 156 + 4 = 160 hours = 20 days
        assert_eq!(result.total_hours, 160);
        assert_eq!(result.total_days, 20);
        assert!(!result.rounded_up);
        assert_eq!(result.breakdown.len(), 27); // 26 PPs + 1 bonus
    }

    #[test]
    fn test_accrual_user_15_plus_years_26pp() {
        let user: User = make_user("2010-01-01");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // 26 PPs * 8 hours = 208 hours = 26 days
        assert_eq!(result.total_hours, 208);
        assert_eq!(result.total_days, 26);
        assert!(!result.rounded_up);
        assert_eq!(result.breakdown.len(), 26); // 26 PPs, no bonus, no rounding
    }

    #[test]
    fn test_accrual_27pp_year() {
        let user: User = make_user("2020-01-01");
        let bid_year: CanonicalBidYear = make_bid_year_27pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // 27 PPs * 6 hours + 4 bonus = 162 + 4 = 166 hours
        // 166 % 8 = 6, so round up by 2 to 168 hours = 21 days
        assert_eq!(result.total_hours, 168);
        assert_eq!(result.total_days, 21);
        assert!(result.rounded_up);
        assert_eq!(result.breakdown.len(), 29); // 27 PPs + 1 bonus + 1 rounding

        // Check that PP 27 has the right reason
        let pp27_entry: &PayPeriodAccrual = result
            .breakdown
            .iter()
            .find(|e| e.pay_period_index == Some(27))
            .unwrap();
        assert_eq!(pp27_entry.reason, AccrualReason::TwentySeventhPP);
    }

    #[test]
    fn test_accrual_transition_at_3_years_mid_year() {
        // SCD is March 15, 2023 -> hits 3 years on March 15, 2026
        let user: User = make_user("2023-03-15");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // Bid year starts Jan 4, 2026
        // PP 1 starts Jan 4 (< 3 years) -> 4 hours
        // ...
        // Need to find which PP starts on or after March 15
        // PP 1: Jan 4 - Jan 17
        // PP 2: Jan 18 - Jan 31
        // PP 3: Feb 1 - Feb 14
        // PP 4: Feb 15 - Feb 28
        // PP 5: Mar 1 - Mar 14 (still < 3 years)
        // PP 6: Mar 15 - Mar 28 (NOW 3 years, but PP 6 starts on Mar 15, so it gets 6 hours)

        let pay_periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();

        // Find PP that starts on or after March 15
        let transition_pp: Option<&PayPeriod> = pay_periods
            .iter()
            .find(|pp| pp.start_date() >= date!(2026 - 03 - 15));

        assert!(transition_pp.is_some());
        let transition_index: u8 = transition_pp.unwrap().index();

        // PPs before transition should be 4 hours, at and after should be 6 hours
        let before_transition_count: usize = (transition_index - 1) as usize;
        let at_and_after_count: usize = (26 - transition_index + 1) as usize;

        let expected_hours_before_rounding: u16 =
            u16::try_from(before_transition_count).unwrap_or(0) * 4
                + u16::try_from(at_and_after_count).unwrap_or(0) * 6
                + 4; // bonus for 6-hour tier

        // 150 % 8 = 6, so needs to round up by 2 to 152
        assert_eq!(result.total_hours, 152);
        assert_eq!(result.total_days, 19);
        assert!(result.rounded_up);

        // Verify pre-rounding calculation
        let sum_from_breakdown: u16 = result
            .breakdown
            .iter()
            .filter(|e| e.reason != AccrualReason::RoundingAdjustment)
            .map(|e| u16::from(e.hours))
            .sum();
        assert_eq!(sum_from_breakdown, expected_hours_before_rounding);

        // Check that the transition PP has the Transition reason
        let transition_entry: Option<&PayPeriodAccrual> = result
            .breakdown
            .iter()
            .find(|e| e.pay_period_index == Some(transition_index));

        assert!(transition_entry.is_some());
        assert_eq!(transition_entry.unwrap().reason, AccrualReason::Transition);
    }

    #[test]
    fn test_accrual_transition_during_pay_period() {
        // SCD is March 10, 2023 -> hits 3 years on March 10, 2026
        // This falls DURING a pay period, not at the start
        let user: User = make_user("2023-03-10");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        let pay_periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();

        // Find the PP that contains March 10
        let pp_containing_anniversary: Option<&PayPeriod> = pay_periods.iter().find(|pp| {
            pp.start_date() <= date!(2026 - 03 - 10) && pp.end_date() >= date!(2026 - 03 - 10)
        });

        assert!(pp_containing_anniversary.is_some());
        let pp_with_anniversary: &PayPeriod = pp_containing_anniversary.unwrap();

        // This PP should still use the OLD rate (4 hours)
        let entry_with_anniversary: Option<&PayPeriodAccrual> = result
            .breakdown
            .iter()
            .find(|e| e.pay_period_index == Some(pp_with_anniversary.index()));

        assert!(entry_with_anniversary.is_some());
        assert_eq!(entry_with_anniversary.unwrap().rate, 4);
        assert_eq!(entry_with_anniversary.unwrap().hours, 4);

        // The NEXT PP should use the new rate (6 hours) and have Transition reason
        let next_pp_index: u8 = pp_with_anniversary.index() + 1;
        let next_pp_entry: Option<&PayPeriodAccrual> = result
            .breakdown
            .iter()
            .find(|e| e.pay_period_index == Some(next_pp_index));

        assert!(next_pp_entry.is_some());
        assert_eq!(next_pp_entry.unwrap().rate, 6);
        assert_eq!(next_pp_entry.unwrap().hours, 6);
        assert_eq!(next_pp_entry.unwrap().reason, AccrualReason::Transition);
    }

    #[test]
    fn test_accrual_transition_at_15_years() {
        // SCD is Jan 4, 2011 -> hits 15 years on Jan 4, 2026 (bid year start)
        let user: User = make_user("2011-01-04");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // All PPs should be at 8 hours (15+ years from the start)
        // 26 * 8 = 208 hours = 26 days
        assert_eq!(result.total_hours, 208);
        assert_eq!(result.total_days, 26);
        assert!(!result.rounded_up);

        // First PP should have 8 hour rate
        let first_pp: &PayPeriodAccrual = &result.breakdown[0];
        assert_eq!(first_pp.rate, 8);
        assert_eq!(first_pp.hours, 8);
    }

    #[test]
    fn test_accrual_rounding_behavior() {
        // User with 1 year of service, but in a 27 PP year without bonus
        // This is contrived, but let's test rounding explicitly
        // Actually, 4 hour rate never gets bonus, so let's use a different approach

        // Use 27 PP year with 6-hour rate
        let user: User = make_user("2020-01-01");
        let bid_year: CanonicalBidYear = make_bid_year_27pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // 27 * 6 + 4 = 166
        // 166 % 8 = 6, needs +2 to reach 168
        assert!(result.rounded_up);
        assert_eq!(result.total_hours % 8, 0);

        // Find the rounding entry
        let rounding_entry: Option<&PayPeriodAccrual> = result
            .breakdown
            .iter()
            .find(|e| e.reason == AccrualReason::RoundingAdjustment);

        assert!(rounding_entry.is_some());
        assert_eq!(rounding_entry.unwrap().hours, 2);
        assert_eq!(rounding_entry.unwrap().pay_period_index, None);
    }

    #[test]
    fn test_accrual_bonus_hours_applied_once() {
        let user: User = make_user("2020-01-01");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // Count bonus entries
        let bonus_count: usize = result
            .breakdown
            .iter()
            .filter(|e| e.reason == AccrualReason::Bonus)
            .count();

        assert_eq!(bonus_count, 1);

        let bonus_entry: &PayPeriodAccrual = result
            .breakdown
            .iter()
            .find(|e| e.reason == AccrualReason::Bonus)
            .unwrap();

        assert_eq!(bonus_entry.hours, 4);
        assert_eq!(bonus_entry.pay_period_index, None);
        assert_eq!(bonus_entry.start_date, None);
        assert_eq!(bonus_entry.end_date, None);
    }

    #[test]
    fn test_accrual_deterministic() {
        let user: User = make_user("2020-06-15");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result1: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();
        let result2: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_accrual_invalid_scd_empty() {
        let mut user: User = make_user("2020-01-01");
        user.seniority_data.service_computation_date = String::new();
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: Result<LeaveAccrualResult, DomainError> =
            calculate_leave_accrual(&user, &bid_year);

        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::InvalidServiceComputationDate { .. } => {}
            _ => panic!("Expected InvalidServiceComputationDate error"),
        }
    }

    #[test]
    fn test_accrual_invalid_scd_format() {
        let mut user: User = make_user("not-a-date");
        user.seniority_data.service_computation_date = "not-a-date".to_string();
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: Result<LeaveAccrualResult, DomainError> =
            calculate_leave_accrual(&user, &bid_year);

        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::DateParseError { .. } => {}
            _ => panic!("Expected DateParseError error"),
        }
    }

    #[test]
    fn test_accrual_breakdown_completeness() {
        let user: User = make_user("2015-01-01");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // User has 11 years of service, so 6-hour tier
        // Should have: 26 PPs + 1 bonus = 27 entries, no rounding

        assert_eq!(result.breakdown.len(), 27);

        // All PP entries should have valid dates and indices
        for entry in &result.breakdown {
            if entry.reason != AccrualReason::Bonus
                && entry.reason != AccrualReason::RoundingAdjustment
            {
                assert!(entry.pay_period_index.is_some());
                assert!(entry.start_date.is_some());
                assert!(entry.end_date.is_some());
                assert!(entry.rate > 0);
                assert!(entry.hours > 0);
            }
        }
    }

    #[test]
    fn test_accrual_no_bonus_for_4_hour_tier() {
        let user: User = make_user("2024-01-01");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // Should have no bonus entry
        let has_bonus: bool = result
            .breakdown
            .iter()
            .any(|e| e.reason == AccrualReason::Bonus);

        assert!(!has_bonus);
    }

    #[test]
    fn test_accrual_no_bonus_for_8_hour_tier() {
        let user: User = make_user("2010-01-01");
        let bid_year: CanonicalBidYear = make_bid_year_26pp();

        let result: LeaveAccrualResult = calculate_leave_accrual(&user, &bid_year).unwrap();

        // Should have no bonus entry
        let has_bonus: bool = result
            .breakdown
            .iter()
            .any(|e| e.reason == AccrualReason::Bonus);

        assert!(!has_bonus);
    }
}
