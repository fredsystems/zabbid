// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Bid window calculation for confirmed bid years.
//!
//! This module calculates individual bid windows based on:
//! - Bid order position
//! - Bidders per area per day
//! - Bid start date (Monday)
//! - Daily bid window times (start/end)
//! - Declared timezone
//!
//! ## Invariants
//!
//! - Bid windows are calculated only after confirmation
//! - Windows are stored as UTC timestamps (ISO 8601)
//! - Bidding occurs Monday-Friday only (weekends are skipped)
//! - All times are wall-clock times in the declared timezone
//! - DST transitions do not make users early or late (nominal labels are stable)
//!
//! ## Usage
//!
//! This logic is used by:
//! - Confirmation process (to materialize bid windows)
//! - Post-confirmation adjustments (to recalculate windows)

use crate::error::DomainError;
use crate::types::BidSchedule;
use chrono::{Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Weekday};
use chrono_tz::Tz;

/// Represents a calculated bid window for a user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidWindow {
    /// The user's canonical ID.
    pub user_id: i64,
    /// Bid order position (1-based).
    pub position: usize,
    /// Window start datetime (UTC, ISO 8601).
    pub window_start_datetime: String,
    /// Window end datetime (UTC, ISO 8601).
    pub window_end_datetime: String,
}

/// Calculates bid windows for users based on their bid order positions.
///
/// # Arguments
///
/// * `user_positions` - User IDs and their 1-based bid order positions
/// * `schedule` - Bid schedule parameters (from `BidYear`)
///
/// # Returns
///
/// A vector of `BidWindow` structs with UTC timestamps.
///
/// # Errors
///
/// Returns an error if:
/// - Timezone is invalid
/// - Date/time conversion fails
///
/// # Window Calculation
///
/// - Users are assigned to days based on `bidders_per_day`
/// - Days are counted Monday-Friday only (weekends are skipped)
/// - Each user gets a window on their assigned day from `window_start_time` to `window_end_time`
/// - Times are converted from declared timezone to UTC for storage
///
/// # Example
///
/// ```text
/// bidders_per_day = 5
/// bid_start_date = 2026-03-02 (Monday)
/// window_start_time = 08:00:00
/// window_end_time = 18:00:00
/// timezone = America/New_York
///
/// Users 1-5:   Monday Mar 2, 08:00-18:00 ET
/// Users 6-10:  Tuesday Mar 3, 08:00-18:00 ET
/// Users 11-15: Wednesday Mar 4, 08:00-18:00 ET
/// Users 16-20: Thursday Mar 5, 08:00-18:00 ET
/// Users 21-25: Friday Mar 6, 08:00-18:00 ET
/// Users 26-30: Monday Mar 9, 08:00-18:00 ET (skip weekend)
/// ```
pub fn calculate_bid_windows(
    user_positions: &[(i64, usize)],
    schedule: &BidSchedule,
) -> Result<Vec<BidWindow>, DomainError> {
    // Parse timezone
    let tz: Tz = schedule
        .timezone()
        .parse()
        .map_err(|_| DomainError::InvalidTimezone(schedule.timezone().to_string()))?;

    // Convert time::Date to chrono::NaiveDate
    let start_date = NaiveDate::from_ymd_opt(
        schedule.start_date().year(),
        schedule.start_date().month() as u32,
        u32::from(schedule.start_date().day()),
    )
    .ok_or_else(|| DomainError::InvalidBidSchedule {
        reason: format!("Invalid bid start date: {}", schedule.start_date()),
    })?;

    // Convert time::Time to chrono::NaiveTime
    let window_start_time = NaiveTime::from_hms_opt(
        u32::from(schedule.window_start_time().hour()),
        u32::from(schedule.window_start_time().minute()),
        u32::from(schedule.window_start_time().second()),
    )
    .ok_or_else(|| DomainError::InvalidBidSchedule {
        reason: format!(
            "Invalid window start time: {}",
            schedule.window_start_time()
        ),
    })?;

    let window_end_time = NaiveTime::from_hms_opt(
        u32::from(schedule.window_end_time().hour()),
        u32::from(schedule.window_end_time().minute()),
        u32::from(schedule.window_end_time().second()),
    )
    .ok_or_else(|| DomainError::InvalidBidSchedule {
        reason: format!("Invalid window end time: {}", schedule.window_end_time()),
    })?;

    // Calculate windows
    let mut windows = Vec::new();

    for (user_id, position) in user_positions {
        let window = calculate_window_for_position(
            *user_id,
            *position,
            start_date,
            window_start_time,
            window_end_time,
            schedule.bidders_per_day(),
            tz,
        )?;
        windows.push(window);
    }

    Ok(windows)
}

/// Calculates the bid window for a single user at a given position.
fn calculate_window_for_position(
    user_id: i64,
    position: usize,
    start_date: NaiveDate,
    window_start_time: NaiveTime,
    window_end_time: NaiveTime,
    bidders_per_day: u32,
    tz: Tz,
) -> Result<BidWindow, DomainError> {
    // Calculate which day this user bids on (0-based weekday index)
    let day_offset = calculate_weekday_offset(position, bidders_per_day);

    // Calculate the actual calendar date
    let bid_date = add_weekdays(start_date, day_offset);

    // Construct wall-clock datetime in declared timezone
    let naive_start = bid_date.and_time(window_start_time);
    let naive_end = bid_date.and_time(window_end_time);

    let start_dt = tz
        .from_local_datetime(&naive_start)
        .single()
        .ok_or_else(|| DomainError::InvalidBidSchedule {
            reason: format!(
                "Could not resolve timezone for date {bid_date} at time {window_start_time} (ambiguous or non-existent due to DST)"
            ),
        })?;

    let end_dt = tz
        .from_local_datetime(&naive_end)
        .single()
        .ok_or_else(|| DomainError::InvalidBidSchedule {
            reason: format!(
                "Could not resolve timezone for date {bid_date} at time {window_end_time} (ambiguous or non-existent due to DST)"
            ),
        })?;

    // Convert to UTC and format as ISO 8601
    let start_utc = start_dt.with_timezone(&chrono::Utc).to_rfc3339();
    let end_utc = end_dt.with_timezone(&chrono::Utc).to_rfc3339();

    Ok(BidWindow {
        user_id,
        position,
        window_start_datetime: start_utc,
        window_end_datetime: end_utc,
    })
}

/// Calculates how many weekdays (Mon-Fri) to skip from start date.
///
/// Position is 1-based. Returns 0-based weekday offset.
#[allow(clippy::cast_possible_wrap)]
const fn calculate_weekday_offset(position: usize, bidders_per_day: u32) -> i64 {
    // Convert position to 0-based index
    let index = position.saturating_sub(1);

    // Calculate how many full days have passed
    let days = index / (bidders_per_day as usize);

    // Safe cast: bid order positions won't exceed i64::MAX
    days as i64
}

/// Adds a number of weekdays (Mon-Fri) to a date, skipping weekends.
fn add_weekdays(start: NaiveDate, weekdays: i64) -> NaiveDate {
    let mut current = start;
    let mut remaining = weekdays;

    while remaining > 0 {
        current += Duration::days(1);

        // Skip weekends
        if current.weekday() != Weekday::Sat && current.weekday() != Weekday::Sun {
            remaining -= 1;
        }
    }

    current
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_add_weekdays_no_offset() {
        let start = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(); // Monday
        let result = add_weekdays(start, 0);
        assert_eq!(result, start);
    }

    #[test]
    fn test_add_weekdays_within_week() {
        let start = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(); // Monday
        let result = add_weekdays(start, 2);
        assert_eq!(result, NaiveDate::from_ymd_opt(2026, 3, 4).unwrap()); // Wednesday
    }

    #[test]
    fn test_add_weekdays_skip_weekend() {
        let start = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(); // Monday
        let result = add_weekdays(start, 5);
        assert_eq!(result, NaiveDate::from_ymd_opt(2026, 3, 9).unwrap()); // Next Monday
    }

    #[test]
    fn test_calculate_weekday_offset() {
        assert_eq!(calculate_weekday_offset(1, 5), 0); // Position 1, day 0
        assert_eq!(calculate_weekday_offset(5, 5), 0); // Position 5, day 0
        assert_eq!(calculate_weekday_offset(6, 5), 1); // Position 6, day 1
        assert_eq!(calculate_weekday_offset(10, 5), 1); // Position 10, day 1
        assert_eq!(calculate_weekday_offset(11, 5), 2); // Position 11, day 2
    }

    #[test]
    fn test_calculate_bid_windows_single_user() {
        let schedule = BidSchedule::new(
            String::from("America/New_York"),
            time::Date::from_calendar_date(2026, time::Month::March, 2).unwrap(),
            time::Time::from_hms(8, 0, 0).unwrap(),
            time::Time::from_hms(18, 0, 0).unwrap(),
            5,
        )
        .unwrap();

        let user_positions = vec![(1001, 1)];

        let windows = calculate_bid_windows(&user_positions, &schedule).unwrap();

        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].user_id, 1001);
        assert_eq!(windows[0].position, 1);
        // Window should be on Monday March 2, 2026
        assert!(windows[0].window_start_datetime.contains("2026-03-02"));
    }

    #[test]
    fn test_calculate_bid_windows_multiple_days() {
        let schedule = BidSchedule::new(
            String::from("America/New_York"),
            time::Date::from_calendar_date(2026, time::Month::March, 2).unwrap(),
            time::Time::from_hms(8, 0, 0).unwrap(),
            time::Time::from_hms(18, 0, 0).unwrap(),
            5,
        )
        .unwrap();

        let user_positions = vec![
            (1001, 1),  // Monday
            (1002, 5),  // Monday
            (1003, 6),  // Tuesday
            (1004, 11), // Wednesday
        ];

        let windows = calculate_bid_windows(&user_positions, &schedule).unwrap();

        assert_eq!(windows.len(), 4);

        // User 1 and 2 on Monday
        assert!(windows[0].window_start_datetime.contains("2026-03-02"));
        assert!(windows[1].window_start_datetime.contains("2026-03-02"));

        // User 3 on Tuesday
        assert!(windows[2].window_start_datetime.contains("2026-03-03"));

        // User 4 on Wednesday
        assert!(windows[3].window_start_datetime.contains("2026-03-04"));
    }

    #[test]
    fn test_calculate_bid_windows_skip_weekend() {
        let schedule = BidSchedule::new(
            String::from("America/New_York"),
            time::Date::from_calendar_date(2026, time::Month::March, 2).unwrap(),
            time::Time::from_hms(8, 0, 0).unwrap(),
            time::Time::from_hms(18, 0, 0).unwrap(),
            5,
        )
        .unwrap();

        let user_positions = vec![
            (1001, 21), // Friday week 1
            (1002, 26), // Monday week 2 (skip weekend)
        ];

        let windows = calculate_bid_windows(&user_positions, &schedule).unwrap();

        assert_eq!(windows.len(), 2);

        // User 1 on Friday March 6
        assert!(windows[0].window_start_datetime.contains("2026-03-06"));

        // User 2 on Monday March 9 (weekend skipped)
        assert!(windows[1].window_start_datetime.contains("2026-03-09"));
    }

    #[test]
    fn test_calculate_bid_windows_invalid_timezone() {
        // BidSchedule::new() should fail validation for invalid timezone
        let result = BidSchedule::new(
            String::from("Invalid/Timezone"),
            time::Date::from_calendar_date(2026, time::Month::March, 2).unwrap(),
            time::Time::from_hms(8, 0, 0).unwrap(),
            time::Time::from_hms(18, 0, 0).unwrap(),
            5,
        );

        assert!(result.is_err());
    }
}
