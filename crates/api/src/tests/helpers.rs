// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Test helper functions and fixtures.

use time::Date;
use zab_bid::BootstrapMetadata;
use zab_bid_audit::Cause;
use zab_bid_domain::{Area, BidYear, CanonicalBidYear};
use zab_bid_persistence::{OperatorData, SqlitePersistence};

use crate::{AuthenticatedActor, RegisterUserRequest, Role};

/// Creates a test admin authenticated actor (for unit tests).
pub fn create_test_admin() -> AuthenticatedActor {
    AuthenticatedActor::new(String::from("admin-123"), Role::Admin)
}

/// Creates a test bidder authenticated actor (for unit tests).
pub fn create_test_bidder() -> AuthenticatedActor {
    AuthenticatedActor::new(String::from("bidder-456"), Role::Bidder)
}

/// Creates a test cause.
pub fn create_test_cause() -> Cause {
    Cause::new(String::from("api-req-456"), String::from("API request"))
}

/// Creates a test admin operator data structure.
pub fn create_test_admin_operator() -> OperatorData {
    OperatorData {
        operator_id: 1,
        login_name: String::from("ADMIN-123"),
        display_name: String::from("Test Admin"),
        password_hash: String::from("$2b$12$test_hash"),
        role: String::from("Admin"),
        is_disabled: false,
        created_at: String::from("2026-01-01T00:00:00Z"),
        disabled_at: None,
        last_login_at: Some(String::from("2026-01-01T00:00:00Z")),
    }
}

/// Creates a test bidder operator data structure.
pub fn create_test_bidder_operator() -> OperatorData {
    OperatorData {
        operator_id: 2,
        login_name: String::from("BIDDER-456"),
        display_name: String::from("Test Bidder"),
        password_hash: String::from("$2b$12$test_hash"),
        role: String::from("Bidder"),
        is_disabled: false,
        created_at: String::from("2026-01-01T00:00:00Z"),
        disabled_at: None,
        last_login_at: Some(String::from("2026-01-01T00:00:00Z")),
    }
}

/// Creates a test metadata with a bid year and area.
pub fn create_test_metadata() -> BootstrapMetadata {
    let mut metadata: BootstrapMetadata = BootstrapMetadata::new();
    let bid_year: BidYear = BidYear::new(2026);
    let area: Area = Area::new("North");
    metadata.bid_years.push(bid_year.clone());
    metadata.areas.push((bid_year, area));
    metadata
}

/// Creates a valid user registration request.
pub fn create_valid_request() -> RegisterUserRequest {
    RegisterUserRequest {
        initials: String::from("AB"),
        name: String::from("John Doe"),
        area: String::from("North"),
        user_type: String::from("CPC"),
        crew: Some(1),
        cumulative_natca_bu_date: String::from("2019-01-15"),
        natca_bu_date: String::from("2019-06-01"),
        eod_faa_date: String::from("2020-01-15"),
        service_computation_date: String::from("2020-01-15"),
        lottery_value: Some(42),
    }
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

/// Creates a test canonical bid year.
///
/// Returns a canonical bid year for 2026 with standard test parameters.
pub fn create_test_canonical_bid_year() -> CanonicalBidYear {
    CanonicalBidYear::new(2026, create_test_start_date(), create_test_pay_periods())
        .expect("Valid test canonical bid year")
}

/// Session-based authentication test helper.
///
/// Represents an authenticated test session with operator credentials.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TestSession {
    /// The session token (for Authorization: Bearer header).
    pub session_token: String,
    /// The operator's login name.
    pub login_name: String,
    /// The operator's role.
    pub role: String,
}

/// Creates and persists a test admin operator, returning the operator ID.
///
/// # Errors
///
/// Returns an error if the operator cannot be created.
#[allow(dead_code)]
pub fn create_persisted_admin_operator(
    persistence: &mut SqlitePersistence,
) -> Result<i64, zab_bid_persistence::PersistenceError> {
    persistence.create_operator("test-admin", "Test Admin", "password", "Admin")
}

/// Creates and persists a test bidder operator, returning the operator ID.
///
/// # Errors
///
/// Returns an error if the operator cannot be created.
#[allow(dead_code)]
pub fn create_persisted_bidder_operator(
    persistence: &mut SqlitePersistence,
) -> Result<i64, zab_bid_persistence::PersistenceError> {
    persistence.create_operator("test-bidder", "Test Bidder", "password", "Bidder")
}

/// Creates a test session for an admin operator.
///
/// This creates both the operator and a valid session in the database.
///
/// # Errors
///
/// Returns an error if the operator or session cannot be created.
#[allow(dead_code)]
pub fn create_admin_session(
    persistence: &mut SqlitePersistence,
) -> Result<TestSession, zab_bid_persistence::PersistenceError> {
    let operator_id: i64 = create_persisted_admin_operator(persistence)?;

    let session_token: String = format!("admin-session-{operator_id}");
    let expires_at: String = String::from("2026-12-31T23:59:59Z");

    persistence.create_session(&session_token, operator_id, &expires_at)?;

    Ok(TestSession {
        session_token,
        login_name: String::from("test-admin"),
        role: String::from("Admin"),
    })
}

/// Creates a test session for a bidder operator.
///
/// This creates both the operator and a valid session in the database.
///
/// # Errors
///
/// Returns an error if the operator or session cannot be created.
#[allow(dead_code)]
pub fn create_bidder_session(
    persistence: &mut SqlitePersistence,
) -> Result<TestSession, zab_bid_persistence::PersistenceError> {
    let operator_id: i64 = create_persisted_bidder_operator(persistence)?;

    let session_token: String = format!("bidder-session-{operator_id}");
    let expires_at: String = String::from("2026-12-31T23:59:59Z");

    persistence.create_session(&session_token, operator_id, &expires_at)?;

    Ok(TestSession {
        session_token,
        login_name: String::from("test-bidder"),
        role: String::from("Bidder"),
    })
}

/// Creates a test session with a custom login name and role.
///
/// # Errors
///
/// Returns an error if the operator or session cannot be created.
#[allow(dead_code)]
pub fn create_custom_session(
    persistence: &mut SqlitePersistence,
    login_name: &str,
    display_name: &str,
    role: &str,
) -> Result<TestSession, zab_bid_persistence::PersistenceError> {
    let operator_id: i64 =
        persistence.create_operator(login_name, display_name, "password", role)?;

    let session_token: String = format!("session-{operator_id}");
    let expires_at: String = String::from("2026-12-31T23:59:59Z");

    persistence.create_session(&session_token, operator_id, &expires_at)?;

    Ok(TestSession {
        session_token,
        login_name: login_name.to_string(),
        role: role.to_string(),
    })
}

/// Creates a test persistence instance with an active bid year and area set up.
///
/// This helper creates an in-memory `SQLite` database, initializes it, creates a bid year,
/// creates an area, and sets the bid year as active.
///
/// # Errors
///
/// Returns an error if database initialization fails.
pub fn setup_test_persistence() -> Result<SqlitePersistence, zab_bid_persistence::PersistenceError>
{
    use zab_bid::{BootstrapMetadata, BootstrapResult, Command, apply_bootstrap};
    use zab_bid_audit::{Actor, Cause};

    let mut persistence = SqlitePersistence::new_in_memory()?;

    // Create a test operator (required for foreign keys)
    let operator_id = persistence
        .create_operator("test-operator", "Test Operator", "password", "Admin")
        .map_err(|e| {
            zab_bid_persistence::PersistenceError::Other(format!("Failed to create operator: {e}"))
        })?;

    let mut metadata = BootstrapMetadata::new();

    // Create a canonical bid year
    let create_bid_year_cmd = Command::CreateBidYear {
        year: 2026,
        start_date: create_test_start_date(),
        num_pay_periods: create_test_pay_periods(),
    };

    let placeholder_bid_year = BidYear::new(2026);
    let actor = Actor::with_operator(
        String::from("test-admin"),
        String::from("admin"),
        operator_id,
        String::from("test-operator"),
        String::from("Test Operator"),
    );
    let cause = Cause::new(String::from("test-setup"), String::from("Test setup"));

    let bid_year_result: BootstrapResult = apply_bootstrap(
        &metadata,
        &placeholder_bid_year,
        create_bid_year_cmd,
        actor.clone(),
        cause.clone(),
    )
    .map_err(|e| zab_bid_persistence::PersistenceError::Other(format!("Bootstrap failed: {e}")))?;

    persistence.persist_bootstrap(&bid_year_result)?;
    metadata = bid_year_result.new_metadata;

    // Create an area
    let create_area_cmd = Command::CreateArea {
        area_id: String::from("North"),
    };

    let active_bid_year = BidYear::new(2026);
    let area_result: BootstrapResult =
        apply_bootstrap(&metadata, &active_bid_year, create_area_cmd, actor, cause).map_err(
            |e| zab_bid_persistence::PersistenceError::Other(format!("Bootstrap failed: {e}")),
        )?;

    persistence.persist_bootstrap(&area_result)?;

    // Set the bid year as active
    persistence.set_active_bid_year(2026)?;

    Ok(persistence)
}
