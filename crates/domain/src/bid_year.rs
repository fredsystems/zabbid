// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Canonical bid year domain model.
//!
//! This module defines the authoritative representation of a bid year,
//! including deterministic pay period derivation.

use crate::error::DomainError;
use serde::{Deserialize, Serialize};
use time::Date;

/// Represents a canonical bid year.
///
/// A bid year is defined by:
/// - A year identifier
/// - A start date
/// - A number of pay periods (26 or 27)
///
/// All other properties (end date, pay periods) are derived deterministically.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalBidYear {
    /// The year identifier (e.g., 2026).
    year: u16,
    /// The start date of the bid year (inclusive).
    start_date: Date,
    /// The number of pay periods (must be 26 or 27).
    num_pay_periods: u8,
}

impl CanonicalBidYear {
    /// Creates a new canonical `CanonicalBidYear`.
    ///
    /// # Arguments
    ///
    /// * `year` - The year identifier
    /// * `start_date` - The start date of the bid year (inclusive)
    /// * `num_pay_periods` - The number of pay periods (must be 26 or 27)
    ///
    /// # Returns
    ///
    /// * `Ok(CanonicalBidYear)` if all inputs are valid
    /// * `Err(DomainError)` if validation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The number of pay periods is not 26 or 27
    pub const fn new(
        year: u16,
        start_date: Date,
        num_pay_periods: u8,
    ) -> Result<Self, DomainError> {
        // Validate number of pay periods
        if num_pay_periods != 26 && num_pay_periods != 27 {
            return Err(DomainError::InvalidPayPeriodCount {
                count: num_pay_periods,
            });
        }

        Ok(Self {
            year,
            start_date,
            num_pay_periods,
        })
    }

    /// Returns the year identifier.
    #[must_use]
    pub const fn year(&self) -> u16 {
        self.year
    }

    /// Returns the start date of the bid year.
    #[must_use]
    pub const fn start_date(&self) -> Date {
        self.start_date
    }

    /// Returns the number of pay periods.
    #[must_use]
    pub const fn num_pay_periods(&self) -> u8 {
        self.num_pay_periods
    }

    /// Derives the end date of the bid year.
    ///
    /// The end date is calculated as:
    /// `start_date` + (`num_pay_periods` * 14 days) - 1 day
    ///
    /// # Returns
    ///
    /// The end date (inclusive) of the bid year.
    ///
    /// # Errors
    ///
    /// Returns an error if date arithmetic overflows.
    pub fn end_date(&self) -> Result<Date, DomainError> {
        let total_days: i64 = i64::from(self.num_pay_periods) * 14;
        self.start_date
            .checked_add(time::Duration::days(total_days - 1))
            .ok_or_else(|| DomainError::DateArithmeticOverflow {
                operation: "calculating bid year end date".to_string(),
            })
    }

    /// Derives all pay periods for this bid year.
    ///
    /// Pay periods are bi-weekly (14 days), contiguous, and non-overlapping.
    /// The first pay period starts on the bid year start date.
    ///
    /// # Returns
    ///
    /// A vector of `PayPeriod` instances in order.
    ///
    /// # Errors
    ///
    /// Returns an error if date arithmetic overflows.
    pub fn pay_periods(&self) -> Result<Vec<PayPeriod>, DomainError> {
        let mut periods: Vec<PayPeriod> = Vec::with_capacity(usize::from(self.num_pay_periods));

        for index in 1..=self.num_pay_periods {
            let period: PayPeriod = self.derive_pay_period(index)?;
            periods.push(period);
        }

        Ok(periods)
    }

    /// Derives a single pay period by index.
    ///
    /// # Arguments
    ///
    /// * `index` - The 1-based pay period index
    ///
    /// # Returns
    ///
    /// The derived `PayPeriod`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The index is out of range
    /// - Date arithmetic overflows
    fn derive_pay_period(&self, index: u8) -> Result<PayPeriod, DomainError> {
        if index < 1 || index > self.num_pay_periods {
            return Err(DomainError::InvalidPayPeriodIndex {
                index,
                max: self.num_pay_periods,
            });
        }

        // Calculate start date: start_date + ((index - 1) * 14 days)
        let offset_days: i64 = i64::from(index - 1) * 14;
        let period_start: Date = self
            .start_date
            .checked_add(time::Duration::days(offset_days))
            .ok_or_else(|| DomainError::DateArithmeticOverflow {
                operation: format!("calculating pay period {index} start date"),
            })?;

        // Calculate end date: period_start + 13 days (14 days inclusive)
        let period_end: Date = period_start
            .checked_add(time::Duration::days(13))
            .ok_or_else(|| DomainError::DateArithmeticOverflow {
                operation: format!("calculating pay period {index} end date"),
            })?;

        Ok(PayPeriod {
            index,
            start_date: period_start,
            end_date: period_end,
        })
    }
}

/// Represents a single pay period within a bid year.
///
/// Pay periods are bi-weekly (14 days), immutable, and derived
/// deterministically from the canonical bid year definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayPeriod {
    /// The 1-based index of this pay period.
    index: u8,
    /// The start date of this pay period (inclusive).
    start_date: Date,
    /// The end date of this pay period (inclusive).
    end_date: Date,
}

impl PayPeriod {
    /// Returns the pay period index (1-based).
    #[must_use]
    pub const fn index(&self) -> u8 {
        self.index
    }

    /// Returns the start date (inclusive).
    #[must_use]
    pub const fn start_date(&self) -> Date {
        self.start_date
    }

    /// Returns the end date (inclusive).
    #[must_use]
    pub const fn end_date(&self) -> Date {
        self.end_date
    }

    /// Returns the number of days in this pay period.
    ///
    /// This should always be 14 for valid pay periods.
    #[must_use]
    pub fn duration_days(&self) -> i64 {
        (self.end_date - self.start_date).whole_days() + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[test]
    fn test_bid_year_new_valid_26_periods() {
        let result: Result<CanonicalBidYear, DomainError> =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26);
        assert!(result.is_ok());
        let bid_year: CanonicalBidYear = result.unwrap();
        assert_eq!(bid_year.year(), 2026);
        assert_eq!(bid_year.start_date(), date!(2026 - 01 - 03));
        assert_eq!(bid_year.num_pay_periods(), 26);
    }

    #[test]
    fn test_bid_year_new_valid_27_periods() {
        let result: Result<CanonicalBidYear, DomainError> =
            CanonicalBidYear::new(2027, date!(2027 - 01 - 02), 27);
        assert!(result.is_ok());
        let bid_year: CanonicalBidYear = result.unwrap();
        assert_eq!(bid_year.year(), 2027);
        assert_eq!(bid_year.start_date(), date!(2027 - 01 - 02));
        assert_eq!(bid_year.num_pay_periods(), 27);
    }

    #[test]
    fn test_bid_year_new_invalid_pay_period_count_too_low() {
        let result: Result<CanonicalBidYear, DomainError> =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 25);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DomainError::InvalidPayPeriodCount { count: 25 }
        ));
    }

    #[test]
    fn test_bid_year_new_invalid_pay_period_count_too_high() {
        let result: Result<CanonicalBidYear, DomainError> =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 28);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DomainError::InvalidPayPeriodCount { count: 28 }
        ));
    }

    #[test]
    fn test_bid_year_new_invalid_pay_period_count_zero() {
        let result: Result<CanonicalBidYear, DomainError> =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DomainError::InvalidPayPeriodCount { count: 0 }
        ));
    }

    #[test]
    fn test_bid_year_end_date_26_periods() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let end_date: Date = bid_year.end_date().unwrap();
        // 26 periods * 14 days = 364 days
        // end_date = start_date + 364 - 1 = start_date + 363
        assert_eq!(end_date, date!(2027 - 01 - 01));
    }

    #[test]
    fn test_bid_year_end_date_27_periods() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2027, date!(2027 - 01 - 02), 27).unwrap();
        let end_date: Date = bid_year.end_date().unwrap();
        // 27 periods * 14 days = 378 days
        // end_date = start_date + 378 - 1 = start_date + 377
        assert_eq!(end_date, date!(2028 - 01 - 14));
    }

    #[test]
    fn test_pay_periods_count_26() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();
        assert_eq!(periods.len(), 26);
    }

    #[test]
    fn test_pay_periods_count_27() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2027, date!(2027 - 01 - 02), 27).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();
        assert_eq!(periods.len(), 27);
    }

    #[test]
    fn test_pay_periods_first_period_26() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();
        let first: &PayPeriod = &periods[0];

        assert_eq!(first.index(), 1);
        assert_eq!(first.start_date(), date!(2026 - 01 - 03));
        assert_eq!(first.end_date(), date!(2026 - 01 - 16));
        assert_eq!(first.duration_days(), 14);
    }

    #[test]
    fn test_pay_periods_last_period_26() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();
        let last: &PayPeriod = &periods[25];

        assert_eq!(last.index(), 26);
        assert_eq!(last.start_date(), date!(2026 - 12 - 19));
        assert_eq!(last.end_date(), date!(2027 - 01 - 01));
        assert_eq!(last.duration_days(), 14);
    }

    #[test]
    fn test_pay_periods_first_period_27() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2027, date!(2027 - 01 - 02), 27).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();
        let first: &PayPeriod = &periods[0];

        assert_eq!(first.index(), 1);
        assert_eq!(first.start_date(), date!(2027 - 01 - 02));
        assert_eq!(first.end_date(), date!(2027 - 01 - 15));
        assert_eq!(first.duration_days(), 14);
    }

    #[test]
    fn test_pay_periods_last_period_27() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2027, date!(2027 - 01 - 02), 27).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();
        let last: &PayPeriod = &periods[26];

        assert_eq!(last.index(), 27);
        assert_eq!(last.start_date(), date!(2028 - 01 - 01));
        assert_eq!(last.end_date(), date!(2028 - 01 - 14));
        assert_eq!(last.duration_days(), 14);
    }

    #[test]
    fn test_pay_periods_contiguous() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();

        for i in 0..periods.len() - 1 {
            let current: &PayPeriod = &periods[i];
            let next: &PayPeriod = &periods[i + 1];

            // Next period should start immediately after current period ends
            let expected_next_start: Date = current
                .end_date()
                .checked_add(time::Duration::days(1))
                .unwrap();
            assert_eq!(next.start_date(), expected_next_start);
        }
    }

    #[test]
    fn test_pay_periods_no_gaps_or_overlaps() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();

        for i in 0..periods.len() - 1 {
            let current: &PayPeriod = &periods[i];
            let next: &PayPeriod = &periods[i + 1];

            // Verify no overlap
            assert!(current.end_date() < next.start_date());

            // Verify no gap
            assert_eq!(
                (next.start_date() - current.end_date()).whole_days(),
                1,
                "Gap detected between period {} and {}",
                current.index(),
                next.index()
            );
        }
    }

    #[test]
    fn test_pay_periods_cover_entire_bid_year() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();

        let first: &PayPeriod = &periods[0];
        let last: &PayPeriod = &periods[periods.len() - 1];

        assert_eq!(first.start_date(), bid_year.start_date());
        assert_eq!(last.end_date(), bid_year.end_date().unwrap());
    }

    #[test]
    fn test_pay_period_all_14_days() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();

        for period in &periods {
            assert_eq!(
                period.duration_days(),
                14,
                "Period {} is not 14 days",
                period.index()
            );
        }
    }

    #[test]
    fn test_derive_pay_period_index_out_of_range_low() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let result: Result<PayPeriod, DomainError> = bid_year.derive_pay_period(0);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DomainError::InvalidPayPeriodIndex { index: 0, max: 26 }
        ));
    }

    #[test]
    fn test_derive_pay_period_index_out_of_range_high() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let result: Result<PayPeriod, DomainError> = bid_year.derive_pay_period(27);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DomainError::InvalidPayPeriodIndex { index: 27, max: 26 }
        ));
    }

    #[test]
    fn test_bid_year_deterministic() {
        // Creating the same bid year twice should produce identical results
        let bid_year1: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let bid_year2: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();

        assert_eq!(bid_year1, bid_year2);
        assert_eq!(bid_year1.end_date().unwrap(), bid_year2.end_date().unwrap());
        assert_eq!(
            bid_year1.pay_periods().unwrap(),
            bid_year2.pay_periods().unwrap()
        );
    }

    #[test]
    fn test_pay_period_indices_sequential() {
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2026, date!(2026 - 01 - 03), 26).unwrap();
        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();

        for (i, period) in periods.iter().enumerate() {
            assert_eq!(period.index(), u8::try_from(i + 1).unwrap());
        }
    }

    #[test]
    fn test_bid_year_leap_year_handling() {
        // 2024 is a leap year, test that date arithmetic handles it correctly
        let bid_year: CanonicalBidYear =
            CanonicalBidYear::new(2024, date!(2024 - 01 - 06), 26).unwrap();
        let end_date: Date = bid_year.end_date().unwrap();
        // 26 periods * 14 days = 364 days
        // 2024 is a leap year (366 days)
        assert_eq!(end_date, date!(2025 - 01 - 03));

        let periods: Vec<PayPeriod> = bid_year.pay_periods().unwrap();
        assert_eq!(periods.len(), 26);

        // Verify all periods are still 14 days even across leap year
        for period in &periods {
            assert_eq!(period.duration_days(), 14);
        }

        // Verify periods are contiguous across leap year boundary
        for i in 0..periods.len() - 1 {
            let current: &PayPeriod = &periods[i];
            let next: &PayPeriod = &periods[i + 1];
            assert_eq!(
                (next.start_date() - current.end_date()).whole_days(),
                1,
                "Gap detected between period {} and {} in leap year",
                current.index(),
                next.index()
            );
        }
    }
}
