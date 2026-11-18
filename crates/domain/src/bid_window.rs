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
use crate::timezone::resolve_timezone;
use crate::types::BidSchedule;
use jiff::civil::{Date, DateTime, Time, Weekday};
use jiff::tz::TimeZone;
use num_traits::ToPrimitive;

/// UTC timestamp format for stored bid windows.
///
/// Matches the RFC 3339 form previously produced by `chrono`'s `to_rfc3339()`
/// (e.g. `2026-03-02T13:00:00+00:00`). Persisted windows must keep a single,
/// stable textual representation.
const UTC_TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%:z";

/// Parameters for calculating a single bid window.
struct WindowCalculationParams<'a> {
    user_id: i64,
    round_id: i64,
    position: usize,
    start_date: Date,
    window_start_time: Time,
    window_end_time: Time,
    bidders_per_day: u32,
    tz: &'a TimeZone,
}

/// Represents a calculated bid window for a user in a specific round.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidWindow {
    /// The user's canonical ID.
    pub user_id: i64,
    /// The round's canonical ID.
    pub round_id: i64,
    /// Bid order position (1-based).
    pub position: usize,
    /// Window start datetime (UTC, ISO 8601).
    pub window_start_datetime: String,
    /// Window end datetime (UTC, ISO 8601).
    pub window_end_datetime: String,
}

/// Calculates bid windows for users in specific rounds based on their bid order positions.
///
/// # Arguments
///
/// * `user_positions` - User IDs and their 1-based bid order positions
/// * `round_ids` - Round IDs to generate windows for
/// * `schedule` - Bid schedule parameters (from `BidYear`)
///
/// # Returns
///
/// A vector of `BidWindow` structs with UTC timestamps, one per (user, round) combination.
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
/// - The same window is replicated across all rounds (per-round adjustments come later)
///
/// # Example
///
/// ```text
/// bidders_per_day = 5
/// bid_start_date = 2026-03-02 (Monday)
/// window_start_time = 08:00:00
/// window_end_time = 18:00:00
/// timezone = America/New_York
/// rounds = [1, 2, 3] (RDO, Leave, Travel)
///
/// User 1 in round 1: Monday Mar 2, 08:00-18:00 ET
/// User 1 in round 2: Monday Mar 2, 08:00-18:00 ET
/// User 1 in round 3: Monday Mar 2, 08:00-18:00 ET
/// Users 2-5 in all rounds: Monday Mar 2, 08:00-18:00 ET
/// Users 6-10 in all rounds: Tuesday Mar 3, 08:00-18:00 ET
/// Users 11-15 in all rounds: Wednesday Mar 4, 08:00-18:00 ET
/// ```
pub fn calculate_bid_windows(
    user_positions: &[(i64, usize)],
    round_ids: &[i64],
    schedule: &BidSchedule,
) -> Result<Vec<BidWindow>, DomainError> {
    let tz: TimeZone = resolve_timezone(schedule.timezone())?;

    let start_date: Date = to_civil_date(schedule.start_date())?;
    let window_start_time: Time = to_civil_time(schedule.window_start_time(), "start")?;
    let window_end_time: Time = to_civil_time(schedule.window_end_time(), "end")?;

    // Calculate windows for each (user, round) combination
    let mut windows: Vec<BidWindow> = Vec::new();

    for (user_id, position) in user_positions {
        for round_id in round_ids {
            let params = WindowCalculationParams {
                user_id: *user_id,
                round_id: *round_id,
                position: *position,
                start_date,
                window_start_time,
                window_end_time,
                bidders_per_day: schedule.bidders_per_day(),
                tz: &tz,
            };
            let window = calculate_window_for_position(&params)?;
            windows.push(window);
        }
    }

    Ok(windows)
}

/// Converts a `time::Date` into a `jiff` civil date.
fn to_civil_date(date: time::Date) -> Result<Date, DomainError> {
    let invalid = || DomainError::InvalidBidSchedule {
        reason: format!("Invalid bid start date: {date}"),
    };

    let year: i16 = date.year().to_i16().ok_or_else(invalid)?;
    let month: i8 = u8::from(date.month()).to_i8().ok_or_else(invalid)?;
    let day: i8 = date.day().to_i8().ok_or_else(invalid)?;

    Date::new(year, month, day).map_err(|_| invalid())
}

/// Converts a `time::Time` into a `jiff` civil time (second precision).
///
/// `label` identifies which window boundary is being converted for error messages.
fn to_civil_time(time: time::Time, label: &str) -> Result<Time, DomainError> {
    let invalid = || DomainError::InvalidBidSchedule {
        reason: format!("Invalid window {label} time: {time}"),
    };

    let hour: i8 = time.hour().to_i8().ok_or_else(invalid)?;
    let minute: i8 = time.minute().to_i8().ok_or_else(invalid)?;
    let second: i8 = time.second().to_i8().ok_or_else(invalid)?;

    Time::new(hour, minute, second, 0).map_err(|_| invalid())
}

/// Resolves a wall-clock datetime in `tz` and formats it as a UTC timestamp.
///
/// Fails if the local time is ambiguous (DST fold) or does not exist (DST gap).
fn to_utc_timestamp(local: DateTime, tz: &TimeZone) -> Result<String, DomainError> {
    let zoned = tz.to_ambiguous_zoned_datetime(local).unambiguous().map_err(|_| {
        DomainError::InvalidBidSchedule {
            reason: format!(
                "Could not resolve timezone for date {} at time {} (ambiguous or non-existent due to DST)",
                local.date(),
                local.time()
            ),
        }
    })?;

    Ok(zoned
        .with_time_zone(TimeZone::UTC)
        .strftime(UTC_TIMESTAMP_FORMAT)
        .to_string())
}

/// Calculates the bid window for a single user in a specific round at a given position.
fn calculate_window_for_position(
    params: &WindowCalculationParams<'_>,
) -> Result<BidWindow, DomainError> {
    // Calculate which day this user bids on (0-based weekday index)
    let day_offset: i64 = calculate_weekday_offset(params.position, params.bidders_per_day);

    // Calculate the actual calendar date
    let bid_date: Date = add_weekdays(params.start_date, day_offset)?;

    // Construct wall-clock datetime in declared timezone and convert to UTC
    let start_utc: String =
        to_utc_timestamp(bid_date.to_datetime(params.window_start_time), params.tz)?;
    let end_utc: String =
        to_utc_timestamp(bid_date.to_datetime(params.window_end_time), params.tz)?;

    Ok(BidWindow {
        user_id: params.user_id,
        round_id: params.round_id,
        position: params.position,
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
///
/// # Errors
///
/// Returns an error if the resulting date is outside the supported calendar range.
fn add_weekdays(start: Date, weekdays: i64) -> Result<Date, DomainError> {
    let mut current: Date = start;
    let mut remaining: i64 = weekdays;

    while remaining > 0 {
        current = current
            .tomorrow()
            .map_err(|_| DomainError::InvalidBidSchedule {
                reason: format!("Bid window date out of range after {current}"),
            })?;

        // Skip weekends
        if current.weekday() != Weekday::Saturday && current.weekday() != Weekday::Sunday {
            remaining -= 1;
        }
    }

    Ok(current)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    use jiff::civil::date;

    fn new_york_schedule(
        start: time::Date,
        window_start: time::Time,
        window_end: time::Time,
    ) -> BidSchedule {
        BidSchedule::new(
            String::from("America/New_York"),
            start,
            window_start,
            window_end,
            5,
        )
        .unwrap()
    }

    #[test]
    fn test_add_weekdays_no_offset() {
        let start = date(2026, 3, 2); // Monday
        let result = add_weekdays(start, 0).unwrap();
        assert_eq!(result, start);
    }

    #[test]
    fn test_add_weekdays_within_week() {
        let start = date(2026, 3, 2); // Monday
        let result = add_weekdays(start, 2).unwrap();
        assert_eq!(result, date(2026, 3, 4)); // Wednesday
    }

    #[test]
    fn test_add_weekdays_skip_weekend() {
        let start = date(2026, 3, 2); // Monday
        let result = add_weekdays(start, 5).unwrap();
        assert_eq!(result, date(2026, 3, 9)); // Next Monday
    }

    #[test]
    fn test_add_weekdays_out_of_range_is_error() {
        let result = add_weekdays(date(9999, 12, 31), 1);
        assert!(matches!(
            result,
            Err(DomainError::InvalidBidSchedule { .. })
        ));
    }

    /// Stored windows must keep the exact RFC 3339 form `YYYY-MM-DDTHH:MM:SS+00:00`.
    /// Changing it (e.g. to a `Z` suffix) would mix formats in persisted data.
    #[test]
    fn test_window_timestamps_use_stable_utc_format() {
        // March 2, 2026 is EST (UTC-5): 08:00 local = 13:00 UTC.
        let schedule = new_york_schedule(
            time::Date::from_calendar_date(2026, time::Month::March, 2).unwrap(),
            time::Time::from_hms(8, 0, 0).unwrap(),
            time::Time::from_hms(18, 0, 0).unwrap(),
        );

        let windows = calculate_bid_windows(&[(1001, 1)], &[1], &schedule).unwrap();

        assert_eq!(
            windows[0].window_start_datetime,
            "2026-03-02T13:00:00+00:00"
        );
        assert_eq!(windows[0].window_end_datetime, "2026-03-02T23:00:00+00:00");
    }

    /// Wall-clock times are stable across DST: after the spring-forward change
    /// (2026-03-08 in `America/New_York`) the same 08:00 local maps to 12:00 UTC.
    #[test]
    fn test_window_after_dst_change_keeps_wall_clock_time() {
        let schedule = new_york_schedule(
            time::Date::from_calendar_date(2026, time::Month::March, 2).unwrap(),
            time::Time::from_hms(8, 0, 0).unwrap(),
            time::Time::from_hms(18, 0, 0).unwrap(),
        );

        // Position 26 with 5 per day => day offset 5 => Monday March 9 (EDT, UTC-4).
        let windows = calculate_bid_windows(&[(1001, 26)], &[1], &schedule).unwrap();

        assert_eq!(
            windows[0].window_start_datetime,
            "2026-03-09T12:00:00+00:00"
        );
        assert_eq!(windows[0].window_end_datetime, "2026-03-09T22:00:00+00:00");
    }

    /// A local time that does not exist (DST gap) must be rejected, not shifted.
    #[test]
    fn test_nonexistent_local_time_is_error() {
        let tz = resolve_timezone("America/New_York").unwrap();
        // 02:30 on 2026-03-08 is skipped by the spring-forward transition.
        let local = date(2026, 3, 8).at(2, 30, 0, 0);

        let result = to_utc_timestamp(local, &tz);

        assert!(matches!(
            result,
            Err(DomainError::InvalidBidSchedule { .. })
        ));
    }

    /// A local time that occurs twice (DST fold) must be rejected, not guessed.
    #[test]
    fn test_ambiguous_local_time_is_error() {
        let tz = resolve_timezone("America/New_York").unwrap();
        // 01:30 on 2026-11-01 occurs twice due to the fall-back transition.
        let local = date(2026, 11, 1).at(1, 30, 0, 0);

        let result = to_utc_timestamp(local, &tz);

        assert!(matches!(
            result,
            Err(DomainError::InvalidBidSchedule { .. })
        ));
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
        let round_ids = vec![1];

        let windows = calculate_bid_windows(&user_positions, &round_ids, &schedule).unwrap();

        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].user_id, 1001);
        assert_eq!(windows[0].round_id, 1);
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
        let round_ids = vec![1];

        let windows = calculate_bid_windows(&user_positions, &round_ids, &schedule).unwrap();

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
        let round_ids = vec![1];

        let windows = calculate_bid_windows(&user_positions, &round_ids, &schedule).unwrap();

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

    #[test]
    fn test_calculate_bid_windows_multiple_rounds() {
        let schedule = BidSchedule::new(
            String::from("America/New_York"),
            time::Date::from_calendar_date(2026, time::Month::March, 2).unwrap(),
            time::Time::from_hms(8, 0, 0).unwrap(),
            time::Time::from_hms(18, 0, 0).unwrap(),
            5,
        )
        .unwrap();

        let user_positions = vec![(1001, 1), (1002, 6)];
        let round_ids = vec![10, 20, 30]; // Three rounds

        let windows = calculate_bid_windows(&user_positions, &round_ids, &schedule).unwrap();

        // Should get 2 users * 3 rounds = 6 windows
        assert_eq!(windows.len(), 6);

        // User 1001 should have windows in all three rounds
        let user_1001_windows: Vec<_> = windows.iter().filter(|w| w.user_id == 1001).collect();
        assert_eq!(user_1001_windows.len(), 3);
        assert_eq!(user_1001_windows[0].round_id, 10);
        assert_eq!(user_1001_windows[1].round_id, 20);
        assert_eq!(user_1001_windows[2].round_id, 30);

        // All windows for user 1001 should be on Monday (position 1)
        assert!(
            user_1001_windows[0]
                .window_start_datetime
                .contains("2026-03-02")
        );
        assert!(
            user_1001_windows[1]
                .window_start_datetime
                .contains("2026-03-02")
        );
        assert!(
            user_1001_windows[2]
                .window_start_datetime
                .contains("2026-03-02")
        );

        // User 1002 should have windows in all three rounds
        let user_1002_windows: Vec<_> = windows.iter().filter(|w| w.user_id == 1002).collect();
        assert_eq!(user_1002_windows.len(), 3);

        // All windows for user 1002 should be on Tuesday (position 6)
        assert!(
            user_1002_windows[0]
                .window_start_datetime
                .contains("2026-03-03")
        );
        assert!(
            user_1002_windows[1]
                .window_start_datetime
                .contains("2026-03-03")
        );
        assert!(
            user_1002_windows[2]
                .window_start_datetime
                .contains("2026-03-03")
        );
    }
}
