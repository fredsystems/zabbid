// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for override operations on canonical tables.

use crate::Persistence;
use diesel::prelude::*;
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::BidYear;

#[test]
fn test_override_area_assignment_sqlite() {
    let mut persistence = setup_area_assignment_test();

    // Perform override
    let result = persistence.override_area_assignment(
        1,
        1,
        2,
        "User requested transfer due to personal circumstances",
    );

    assert!(result.is_ok(), "Override should succeed");
    let (previous_area_id, was_overridden) = result.unwrap();
    assert_eq!(previous_area_id, 1, "Previous area should be 1");
    assert!(
        !was_overridden,
        "Should not have been previously overridden"
    );

    verify_area_assignment_override(&mut persistence);
}

fn setup_area_assignment_test() -> Persistence {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");
    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            use crate::diesel_schema::{areas, bid_years, users};
            use crate::mutations::bootstrap::canonicalize_bid_year_sqlite;

            diesel::insert_into(bid_years::table)
                .values((
                    bid_years::bid_year_id.eq(1),
                    bid_years::year.eq(2026),
                    bid_years::start_date.eq("2026-01-04"),
                    bid_years::num_pay_periods.eq(26),
                    bid_years::is_active.eq(1),
                    bid_years::lifecycle_state.eq("BootstrapComplete"),
                ))
                .execute(conn)
                .expect("Failed to insert bid year");

            // Create areas
            diesel::insert_into(areas::table)
                .values(vec![
                    (
                        areas::area_id.eq(1),
                        areas::bid_year_id.eq(1),
                        areas::area_code.eq("ABC"),
                        areas::area_name.eq(Some("Area ABC")),
                        areas::is_system_area.eq(0),
                    ),
                    (
                        areas::area_id.eq(2),
                        areas::bid_year_id.eq(1),
                        areas::area_code.eq("XYZ"),
                        areas::area_name.eq(Some("Area XYZ")),
                        areas::is_system_area.eq(0),
                    ),
                ])
                .execute(conn)
                .expect("Failed to insert areas");

            // Create user
            diesel::insert_into(users::table)
                .values((
                    users::user_id.eq(1),
                    users::bid_year_id.eq(1),
                    users::area_id.eq(1),
                    users::initials.eq("ABC"),
                    users::name.eq("Test User"),
                    users::user_type.eq("CPC"),
                    users::crew.eq(None::<i32>),
                    users::cumulative_natca_bu_date.eq("2020-01-01"),
                    users::natca_bu_date.eq("2020-01-01"),
                    users::eod_faa_date.eq("2020-01-01"),
                    users::service_computation_date.eq("2020-01-01"),
                ))
                .execute(conn)
                .expect("Failed to insert user");

            // Create operator for audit
            diesel::sql_query(
                "INSERT INTO operators (operator_id, login_name, display_name, password_hash, role, is_disabled, created_at)
                 VALUES (1, 'admin', 'Admin', 'hash', 'Admin', 0, '2026-01-01T00:00:00')",
            )
            .execute(conn)
            .expect("Failed to insert operator");

            // Create audit event for canonicalization
            let audit_event = AuditEvent {
                event_id: None,
                actor: Actor {
                    actor_type: String::from("Operator"),
                    id: String::from("1"),
                    operator_id: Some(1),
                    operator_login_name: Some(String::from("admin")),
                    operator_display_name: Some(String::from("Admin")),
                },
                cause: Cause {
                    id: String::from("test"),
                    description: String::from("Test canonicalization"),
                },
                action: Action {
                    name: String::from("CanonicalizeBidYear"),
                    details: Some(String::from("Test")),
                },
                before: StateSnapshot::new(String::from("lifecycle_state=BootstrapComplete")),
                after: StateSnapshot::new(String::from("lifecycle_state=Canonicalized")),
                bid_year: Some(BidYear::new(2026)),
                area: None,
            };

            // Canonicalize to create canonical tables properly
            canonicalize_bid_year_sqlite(conn, 1, &audit_event)
                .expect("Failed to canonicalize bid year");
        }
        crate::BackendConnection::Mysql(_) => panic!("Expected SQLite connection"),
    }
    persistence
}

fn verify_area_assignment_override(persistence: &mut Persistence) {
    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            use crate::diesel_schema::canonical_area_membership;

            let (area_id, is_overridden, reason): (i64, i32, Option<String>) =
                canonical_area_membership::table
                    .filter(canonical_area_membership::user_id.eq(1))
                    .select((
                        canonical_area_membership::area_id,
                        canonical_area_membership::is_overridden,
                        canonical_area_membership::override_reason,
                    ))
                    .first(conn)
                    .expect("Failed to query canonical_area_membership");

            assert_eq!(area_id, 2, "Area should be updated to 2");
            assert_eq!(is_overridden, 1, "is_overridden should be 1");
            assert!(reason.is_some(), "override_reason should be set");
            assert_eq!(
                reason.unwrap(),
                "User requested transfer due to personal circumstances"
            );
        }
        crate::BackendConnection::Mysql(_) => panic!("Expected SQLite connection"),
    }
}

#[test]
fn test_override_eligibility_sqlite() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    // Set up test data
    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            use crate::diesel_schema::{areas, bid_years, users};
            use crate::mutations::bootstrap::canonicalize_bid_year_sqlite;

            diesel::insert_into(bid_years::table)
                .values((
                    bid_years::bid_year_id.eq(1),
                    bid_years::year.eq(2026),
                    bid_years::start_date.eq("2026-01-04"),
                    bid_years::num_pay_periods.eq(26),
                    bid_years::is_active.eq(1),
                    bid_years::lifecycle_state.eq("BootstrapComplete"),
                ))
                .execute(conn)
                .expect("Failed to insert bid year");

            diesel::insert_into(areas::table)
                .values((
                    areas::area_id.eq(1),
                    areas::bid_year_id.eq(1),
                    areas::area_code.eq("ABC"),
                    areas::area_name.eq(Some("Area ABC")),
                    areas::is_system_area.eq(0),
                ))
                .execute(conn)
                .expect("Failed to insert area");

            diesel::insert_into(users::table)
                .values((
                    users::user_id.eq(1),
                    users::bid_year_id.eq(1),
                    users::area_id.eq(1),
                    users::initials.eq("ABC"),
                    users::name.eq("Test User"),
                    users::user_type.eq("CPC"),
                    users::crew.eq(None::<i32>),
                    users::cumulative_natca_bu_date.eq("2020-01-01"),
                    users::natca_bu_date.eq("2020-01-01"),
                    users::eod_faa_date.eq("2020-01-01"),
                    users::service_computation_date.eq("2020-01-01"),
                ))
                .execute(conn)
                .expect("Failed to insert user");

            diesel::sql_query(
                "INSERT INTO operators (operator_id, login_name, display_name, password_hash, role, is_disabled, created_at)
                 VALUES (1, 'admin', 'Admin', 'hash', 'Admin', 0, '2026-01-01T00:00:00')",
            )
            .execute(conn)
            .expect("Failed to insert operator");

            let audit_event = AuditEvent {
                event_id: None,
                actor: Actor {
                    actor_type: String::from("Operator"),
                    id: String::from("1"),
                    operator_id: Some(1),
                    operator_login_name: Some(String::from("admin")),
                    operator_display_name: Some(String::from("Admin")),
                },
                cause: Cause {
                    id: String::from("test"),
                    description: String::from("Test canonicalization"),
                },
                action: Action {
                    name: String::from("CanonicalizeBidYear"),
                    details: Some(String::from("Test")),
                },
                before: StateSnapshot::new(String::from("lifecycle_state=BootstrapComplete")),
                after: StateSnapshot::new(String::from("lifecycle_state=Canonicalized")),
                bid_year: Some(BidYear::new(2026)),
                area: None,
            };

            canonicalize_bid_year_sqlite(conn, 1, &audit_event)
                .expect("Failed to canonicalize bid year");
        }
        crate::BackendConnection::Mysql(_) => panic!("Expected SQLite connection"),
    }

    // Perform override to set eligibility to false
    let result = persistence.override_eligibility(
        1,
        1,
        false,
        "User on extended leave, ineligible for this bid year",
    );

    assert!(result.is_ok(), "Override should succeed");
    let (previous_eligibility, was_overridden) = result.unwrap();
    assert!(previous_eligibility, "Previous eligibility should be true");
    assert!(
        !was_overridden,
        "Should not have been previously overridden"
    );

    verify_eligibility_override_applied(&mut persistence);
}

fn verify_eligibility_override_applied(persistence: &mut Persistence) {
    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            use crate::diesel_schema::canonical_eligibility;

            let (can_bid, is_overridden, reason): (i32, i32, Option<String>) =
                canonical_eligibility::table
                    .filter(canonical_eligibility::user_id.eq(1))
                    .select((
                        canonical_eligibility::can_bid,
                        canonical_eligibility::is_overridden,
                        canonical_eligibility::override_reason,
                    ))
                    .first(conn)
                    .expect("Failed to query canonical_eligibility");

            assert_eq!(can_bid, 0, "can_bid should be 0 (false)");
            assert_eq!(is_overridden, 1, "is_overridden should be 1");
            assert!(reason.is_some(), "override_reason should be set");
        }
        crate::BackendConnection::Mysql(_) => panic!("Expected SQLite connection"),
    }
}

#[test]
fn test_override_bid_order_sqlite() {
    let mut persistence = setup_bid_order_test();

    // Perform override to change bid order
    let result = persistence.override_bid_order(
        1,
        1,
        Some(42),
        "Seniority calculation corrected per union agreement",
    );

    assert!(result.is_ok(), "Override should succeed");
    let (previous_bid_order, was_overridden) = result.unwrap();
    assert_eq!(
        previous_bid_order, None,
        "Previous bid order should be None"
    );
    assert!(
        !was_overridden,
        "Should not have been previously overridden"
    );

    verify_bid_order_override(&mut persistence);
}

fn setup_bid_order_test() -> Persistence {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");
    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            use crate::diesel_schema::{areas, bid_years, users};
            use crate::mutations::bootstrap::canonicalize_bid_year_sqlite;

            diesel::insert_into(bid_years::table)
                .values((
                    bid_years::bid_year_id.eq(1),
                    bid_years::year.eq(2026),
                    bid_years::start_date.eq("2026-01-04"),
                    bid_years::num_pay_periods.eq(26),
                    bid_years::is_active.eq(1),
                    bid_years::lifecycle_state.eq("BootstrapComplete"),
                ))
                .execute(conn)
                .expect("Failed to insert bid year");

            diesel::insert_into(areas::table)
                .values((
                    areas::area_id.eq(1),
                    areas::bid_year_id.eq(1),
                    areas::area_code.eq("ABC"),
                    areas::area_name.eq(Some("Area ABC")),
                    areas::is_system_area.eq(0),
                ))
                .execute(conn)
                .expect("Failed to insert area");

            diesel::insert_into(users::table)
                .values((
                    users::user_id.eq(1),
                    users::bid_year_id.eq(1),
                    users::area_id.eq(1),
                    users::initials.eq("ABC"),
                    users::name.eq("Test User"),
                    users::user_type.eq("CPC"),
                    users::crew.eq(None::<i32>),
                    users::cumulative_natca_bu_date.eq("2020-01-01"),
                    users::natca_bu_date.eq("2020-01-01"),
                    users::eod_faa_date.eq("2020-01-01"),
                    users::service_computation_date.eq("2020-01-01"),
                ))
                .execute(conn)
                .expect("Failed to insert user");

            diesel::sql_query(
                "INSERT INTO operators (operator_id, login_name, display_name, password_hash, role, is_disabled, created_at)
                 VALUES (1, 'admin', 'Admin', 'hash', 'Admin', 0, '2026-01-01T00:00:00')",
            )
            .execute(conn)
            .expect("Failed to insert operator");

            let audit_event = AuditEvent {
                event_id: None,
                actor: Actor {
                    actor_type: String::from("Operator"),
                    id: String::from("1"),
                    operator_id: Some(1),
                    operator_login_name: Some(String::from("admin")),
                    operator_display_name: Some(String::from("Admin")),
                },
                cause: Cause {
                    id: String::from("test"),
                    description: String::from("Test canonicalization"),
                },
                action: Action {
                    name: String::from("CanonicalizeBidYear"),
                    details: Some(String::from("Test")),
                },
                before: StateSnapshot::new(String::from("lifecycle_state=BootstrapComplete")),
                after: StateSnapshot::new(String::from("lifecycle_state=Canonicalized")),
                bid_year: Some(BidYear::new(2026)),
                area: None,
            };

            canonicalize_bid_year_sqlite(conn, 1, &audit_event)
                .expect("Failed to canonicalize bid year");
        }
        crate::BackendConnection::Mysql(_) => panic!("Expected SQLite connection"),
    }
    persistence
}

fn verify_bid_order_override(persistence: &mut Persistence) {
    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            use crate::diesel_schema::canonical_bid_order;

            let (bid_order, is_overridden, reason): (Option<i32>, i32, Option<String>) =
                canonical_bid_order::table
                    .filter(canonical_bid_order::user_id.eq(1))
                    .select((
                        canonical_bid_order::bid_order,
                        canonical_bid_order::is_overridden,
                        canonical_bid_order::override_reason,
                    ))
                    .first(conn)
                    .expect("Failed to query canonical_bid_order");

            assert_eq!(bid_order, Some(42), "bid_order should be 42");
            assert_eq!(is_overridden, 1, "is_overridden should be 1");
            assert!(reason.is_some(), "override_reason should be set");
        }
        crate::BackendConnection::Mysql(_) => panic!("Expected SQLite connection"),
    }
}

#[test]
fn test_override_bid_window_sqlite() {
    let mut persistence = setup_bid_window_test();

    // Perform override to change bid window
    let result = persistence.override_bid_window(
        1,
        1,
        Some("2026-03-01".to_string()).as_ref(),
        Some("2026-03-10".to_string()).as_ref(),
        "Extended window due to leave during standard window",
    );

    assert!(result.is_ok(), "Override should succeed");
    let (previous_start, previous_end, was_overridden) = result.unwrap();
    assert_eq!(previous_start, None);
    assert_eq!(previous_end, None);
    assert!(
        !was_overridden,
        "Should not have been previously overridden"
    );

    verify_bid_window_override(&mut persistence);
}

fn setup_bid_window_test() -> Persistence {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");
    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            use crate::diesel_schema::{areas, bid_years, users};
            use crate::mutations::bootstrap::canonicalize_bid_year_sqlite;

            diesel::insert_into(bid_years::table)
                .values((
                    bid_years::bid_year_id.eq(1),
                    bid_years::year.eq(2026),
                    bid_years::start_date.eq("2026-01-04"),
                    bid_years::num_pay_periods.eq(26),
                    bid_years::is_active.eq(1),
                    bid_years::lifecycle_state.eq("BootstrapComplete"),
                ))
                .execute(conn)
                .expect("Failed to insert bid year");

            diesel::insert_into(areas::table)
                .values((
                    areas::area_id.eq(1),
                    areas::bid_year_id.eq(1),
                    areas::area_code.eq("ABC"),
                    areas::area_name.eq(Some("Area ABC")),
                    areas::is_system_area.eq(0),
                ))
                .execute(conn)
                .expect("Failed to insert area");

            diesel::insert_into(users::table)
                .values((
                    users::user_id.eq(1),
                    users::bid_year_id.eq(1),
                    users::area_id.eq(1),
                    users::initials.eq("ABC"),
                    users::name.eq("Test User"),
                    users::user_type.eq("CPC"),
                    users::crew.eq(None::<i32>),
                    users::cumulative_natca_bu_date.eq("2020-01-01"),
                    users::natca_bu_date.eq("2020-01-01"),
                    users::eod_faa_date.eq("2020-01-01"),
                    users::service_computation_date.eq("2020-01-01"),
                ))
                .execute(conn)
                .expect("Failed to insert user");

            diesel::sql_query(
                "INSERT INTO operators (operator_id, login_name, display_name, password_hash, role, is_disabled, created_at)
                 VALUES (1, 'admin', 'Admin', 'hash', 'Admin', 0, '2026-01-01T00:00:00')",
            )
            .execute(conn)
            .expect("Failed to insert operator");

            let audit_event = AuditEvent {
                event_id: None,
                actor: Actor {
                    actor_type: String::from("Operator"),
                    id: String::from("1"),
                    operator_id: Some(1),
                    operator_login_name: Some(String::from("admin")),
                    operator_display_name: Some(String::from("Admin")),
                },
                cause: Cause {
                    id: String::from("test"),
                    description: String::from("Test canonicalization"),
                },
                action: Action {
                    name: String::from("CanonicalizeBidYear"),
                    details: Some(String::from("Test")),
                },
                before: StateSnapshot::new(String::from("lifecycle_state=BootstrapComplete")),
                after: StateSnapshot::new(String::from("lifecycle_state=Canonicalized")),
                bid_year: Some(BidYear::new(2026)),
                area: None,
            };

            canonicalize_bid_year_sqlite(conn, 1, &audit_event)
                .expect("Failed to canonicalize bid year");
        }
        crate::BackendConnection::Mysql(_) => panic!("Expected SQLite connection"),
    }

    persistence
}

fn verify_bid_window_override(persistence: &mut Persistence) {
    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            use crate::diesel_schema::canonical_bid_windows;

            let (start, end, is_overridden, reason): (
                Option<String>,
                Option<String>,
                i32,
                Option<String>,
            ) = canonical_bid_windows::table
                .filter(canonical_bid_windows::user_id.eq(1))
                .select((
                    canonical_bid_windows::window_start_date,
                    canonical_bid_windows::window_end_date,
                    canonical_bid_windows::is_overridden,
                    canonical_bid_windows::override_reason,
                ))
                .first(conn)
                .expect("Failed to query canonical_bid_windows");

            assert_eq!(start, Some("2026-03-01".to_string()));
            assert_eq!(end, Some("2026-03-10".to_string()));
            assert_eq!(is_overridden, 1, "is_overridden should be 1");
            assert!(reason.is_some(), "override_reason should be set");
        }
        crate::BackendConnection::Mysql(_) => panic!("Expected SQLite connection"),
    }
}

#[test]
fn test_override_twice_tracks_was_overridden() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    // Set up test data
    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            use crate::diesel_schema::{areas, bid_years, users};
            use crate::mutations::bootstrap::canonicalize_bid_year_sqlite;

            diesel::insert_into(bid_years::table)
                .values((
                    bid_years::bid_year_id.eq(1),
                    bid_years::year.eq(2026),
                    bid_years::start_date.eq("2026-01-04"),
                    bid_years::num_pay_periods.eq(26),
                    bid_years::is_active.eq(1),
                    bid_years::lifecycle_state.eq("BootstrapComplete"),
                ))
                .execute(conn)
                .expect("Failed to insert bid year");

            diesel::insert_into(areas::table)
                .values((
                    areas::area_id.eq(1),
                    areas::bid_year_id.eq(1),
                    areas::area_code.eq("ABC"),
                    areas::area_name.eq(None::<&str>),
                    areas::is_system_area.eq(0),
                ))
                .execute(conn)
                .expect("Failed to insert area");

            diesel::insert_into(users::table)
                .values((
                    users::user_id.eq(1),
                    users::bid_year_id.eq(1),
                    users::area_id.eq(1),
                    users::initials.eq("ABC"),
                    users::name.eq("Test User"),
                    users::user_type.eq("CPC"),
                    users::crew.eq(None::<i32>),
                    users::cumulative_natca_bu_date.eq("2020-01-01"),
                    users::natca_bu_date.eq("2020-01-01"),
                    users::eod_faa_date.eq("2020-01-01"),
                    users::service_computation_date.eq("2020-01-01"),
                ))
                .execute(conn)
                .expect("Failed to insert user");

            diesel::sql_query(
                "INSERT INTO operators (operator_id, login_name, display_name, password_hash, role, is_disabled, created_at)
                 VALUES (1, 'admin', 'Admin', 'hash', 'Admin', 0, '2026-01-01T00:00:00')",
            )
            .execute(conn)
            .expect("Failed to insert operator");

            let audit_event = AuditEvent {
                event_id: None,
                actor: Actor {
                    actor_type: String::from("Operator"),
                    id: String::from("1"),
                    operator_id: Some(1),
                    operator_login_name: Some(String::from("admin")),
                    operator_display_name: Some(String::from("Admin")),
                },
                cause: Cause {
                    id: String::from("test"),
                    description: String::from("Test canonicalization"),
                },
                action: Action {
                    name: String::from("CanonicalizeBidYear"),
                    details: Some(String::from("Test")),
                },
                before: StateSnapshot::new(String::from("lifecycle_state=BootstrapComplete")),
                after: StateSnapshot::new(String::from("lifecycle_state=Canonicalized")),
                bid_year: Some(BidYear::new(2026)),
                area: None,
            };

            canonicalize_bid_year_sqlite(conn, 1, &audit_event)
                .expect("Failed to canonicalize bid year");
        }
        crate::BackendConnection::Mysql(_) => panic!("Expected SQLite connection"),
    }

    // First override
    let result1 = persistence.override_eligibility(
        1,
        1,
        false,
        "First override reason with sufficient length",
    );
    assert!(result1.is_ok());
    let (_, was_overridden1) = result1.unwrap();
    assert!(
        !was_overridden1,
        "First override should report not previously overridden"
    );

    // Second override
    let result2 = persistence.override_eligibility(
        1,
        1,
        true,
        "Second override reason with sufficient length",
    );
    assert!(result2.is_ok());
    let (_, was_overridden2) = result2.unwrap();
    assert!(
        was_overridden2,
        "Second override should report previously overridden"
    );
}
