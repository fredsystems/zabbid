// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

mod backend_validation_tests;
mod bootstrap_tests;
mod canonical_tests;
mod initialization_tests;
mod mutation_error_tests;
mod operator_tests;
mod override_tests;
mod state_tests;

use time::Date;
use zab_bid::BootstrapMetadata;
use zab_bid_audit::{Actor, Cause};
use zab_bid_domain::{Area, BidYear, SeniorityData};

pub fn create_test_actor() -> Actor {
    Actor::with_operator(
        String::from("test-actor"),
        String::from("admin"),
        1,
        String::from("test-operator"),
        String::from("Test Operator"),
    )
}

pub fn create_test_cause() -> Cause {
    Cause::new(String::from("test-cause"), String::from("Test operation"))
}

pub fn create_test_seniority_data() -> SeniorityData {
    SeniorityData::new(
        String::from("2019-01-15"),
        String::from("2019-06-01"),
        String::from("2020-01-15"),
        String::from("2020-01-15"),
        Some(42),
    )
}

pub fn create_test_metadata() -> BootstrapMetadata {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    metadata.bid_years.push(BidYear::new(2026));
    metadata
        .areas
        .push((BidYear::new(2026), Area::new("North")));
    metadata
}

/// Creates a test start date for bid year tests.
///
/// Returns January 4, 2026 (a Sunday) as a valid test start date.
pub fn create_test_start_date() -> Date {
    Date::from_calendar_date(2026, time::Month::January, 4).expect("Valid test date")
}

/// Creates test start date for a specific year.
///
/// Returns the first Sunday of January for the given year.
pub fn create_test_start_date_for_year(year: i32) -> Date {
    // Start with January 1st
    let jan_1 = Date::from_calendar_date(year, time::Month::January, 1).expect("Valid January 1st");

    // Find the first Sunday
    let weekday = jan_1.weekday();
    let days_until_sunday: i64 = match weekday {
        time::Weekday::Sunday => 0,
        time::Weekday::Monday => 6,
        time::Weekday::Tuesday => 5,
        time::Weekday::Wednesday => 4,
        time::Weekday::Thursday => 3,
        time::Weekday::Friday => 2,
        time::Weekday::Saturday => 1,
    };

    jan_1
        .checked_add(time::Duration::days(days_until_sunday))
        .expect("Valid date calculation")
}

/// Returns a valid test pay period count (26).
pub fn create_test_pay_periods() -> u8 {
    26
}

/// Creates a test operator in the persistence layer and returns the operator ID.
///
/// This must be called before persisting any audit events to satisfy foreign key constraints.
pub fn create_test_operator(persistence: &mut crate::SqlitePersistence) -> i64 {
    persistence
        .create_operator("test-operator", "Test Operator", "password", "Admin")
        .expect("Failed to create test operator")
}

/// Creates a test bid year and area in the persistence layer.
///
/// This bootstraps the canonical bid year and area, which is required for Phase 23A+
/// since audit events must reference existing canonical entities.
///
/// # Arguments
///
/// * `persistence` - The persistence layer
/// * `year` - The bid year number
/// * `area_code` - The area code
///
/// # Panics
///
/// Panics if bootstrap fails.
pub fn create_test_bid_year_and_area(
    persistence: &mut crate::SqlitePersistence,
    year: u16,
    area_code: &str,
) {
    use zab_bid::{Command, apply_bootstrap};

    let metadata: BootstrapMetadata = BootstrapMetadata::new();

    // Bootstrap bid year
    let create_bid_year_cmd: Command = Command::CreateBidYear {
        year,
        start_date: create_test_start_date_for_year(i32::from(year)),
        num_pay_periods: create_test_pay_periods(),
    };
    let bid_year_result = apply_bootstrap(
        &metadata,
        &BidYear::new(year),
        create_bid_year_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .expect("Failed to create test bid year");

    persistence
        .persist_bootstrap(&bid_year_result)
        .expect("Failed to persist test bid year");

    // Update metadata for area creation
    let mut metadata = metadata;
    metadata.bid_years.push(BidYear::new(year));

    // Bootstrap area
    let create_area_cmd: Command = Command::CreateArea {
        area_id: area_code.to_string(),
    };
    let area_result = apply_bootstrap(
        &metadata,
        &BidYear::new(year),
        create_area_cmd,
        create_test_actor(),
        create_test_cause(),
    )
    .expect("Failed to create test area");

    persistence
        .persist_bootstrap(&area_result)
        .expect("Failed to persist test area");
}
