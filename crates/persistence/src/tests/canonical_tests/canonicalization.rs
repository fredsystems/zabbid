// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for bid year canonicalization.
//!
//! These tests verify that the canonicalization persistence layer works correctly.

use diesel::prelude::*;
use zab_bid_audit::{Action, Actor, AuditEvent, Cause, StateSnapshot};
use zab_bid_domain::BidYear;

use crate::Persistence;
use crate::diesel_schema::{
    canonical_area_membership, canonical_bid_order, canonical_bid_windows, canonical_eligibility,
};
use crate::mutations::bootstrap::canonicalize_bid_year_sqlite;

/// Test that canonicalization creates all required canonical tables for `SQLite`.
#[test]
#[allow(clippy::too_many_lines)]
fn test_canonicalize_creates_tables_sqlite() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    // Set up minimal test data using raw SQL
    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            diesel::sql_query(
                "INSERT INTO bid_years (bid_year_id, year, start_date, num_pay_periods, is_active, lifecycle_state)
                 VALUES (1, 2026, '2026-01-04', 26, 1, 'BootstrapComplete')",
            )
            .execute(conn)
            .expect("Failed to insert bid year");

            diesel::sql_query(
                "INSERT INTO areas (area_id, bid_year_id, area_code, area_name, is_system_area)
                 VALUES (1, 1, 'AREA1', 'Test Area', 0)",
            )
            .execute(conn)
            .expect("Failed to insert area");

            diesel::sql_query(
                "INSERT INTO users (user_id, bid_year_id, area_id, initials, name, user_type, cumulative_natca_bu_date, natca_bu_date, eod_faa_date, service_computation_date, excluded_from_bidding, excluded_from_leave_calculation)
                 VALUES
                 (1, 1, 1, 'ABC', 'User One', 'CPC', '2020-01-01', '2020-01-01', '2020-01-01', '2020-01-01', 0, 0),
                 (2, 1, 1, 'DEF', 'User Two', 'CPC', '2021-01-01', '2021-01-01', '2021-01-01', '2021-01-01', 0, 0)",
            )
            .execute(conn)
            .expect("Failed to insert users");

            diesel::sql_query(
                "INSERT INTO operators (operator_id, login_name, display_name, password_hash, role, is_disabled, created_at)
                 VALUES (1, 'admin', 'Admin', 'hash', 'Admin', 0, '2026-01-01T00:00:00')",
            )
            .execute(conn)
            .expect("Failed to insert operator");

            // Create audit event
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

            // Canonicalize
            let event_id = canonicalize_bid_year_sqlite(conn, 1, &audit_event)
                .expect("Canonicalization failed");

            assert!(event_id > 0, "Event ID should be assigned");

            // Verify canonical rows were created
            let area_count: i64 = canonical_area_membership::table
                .filter(canonical_area_membership::bid_year_id.eq(1))
                .count()
                .get_result(conn)
                .expect("Failed to count area membership");
            assert_eq!(area_count, 2, "Should have 2 area membership rows");

            let eligibility_count: i64 = canonical_eligibility::table
                .filter(canonical_eligibility::bid_year_id.eq(1))
                .count()
                .get_result(conn)
                .expect("Failed to count eligibility");
            assert_eq!(eligibility_count, 2, "Should have 2 eligibility rows");

            let bid_order_count: i64 = canonical_bid_order::table
                .filter(canonical_bid_order::bid_year_id.eq(1))
                .count()
                .get_result(conn)
                .expect("Failed to count bid order");
            assert_eq!(bid_order_count, 2, "Should have 2 bid order rows");

            let bid_windows_count: i64 = canonical_bid_windows::table
                .filter(canonical_bid_windows::bid_year_id.eq(1))
                .count()
                .get_result(conn)
                .expect("Failed to count bid windows");
            assert_eq!(bid_windows_count, 2, "Should have 2 bid windows rows");

            // Verify eligibility defaults to 1 (true)
            let can_bid: i32 = canonical_eligibility::table
                .filter(canonical_eligibility::bid_year_id.eq(1))
                .select(canonical_eligibility::can_bid)
                .first(conn)
                .expect("Failed to query eligibility");
            assert_eq!(can_bid, 1, "Eligibility should default to 1");

            // Verify bid_order is NULL
            let bid_order: Option<i32> = canonical_bid_order::table
                .filter(canonical_bid_order::bid_year_id.eq(1))
                .select(canonical_bid_order::bid_order)
                .first(conn)
                .expect("Failed to query bid order");
            assert_eq!(bid_order, None, "Bid order should be NULL");

            // Verify bid windows are NULL
            let (start, end): (Option<String>, Option<String>) = canonical_bid_windows::table
                .filter(canonical_bid_windows::bid_year_id.eq(1))
                .select((
                    canonical_bid_windows::window_start_date,
                    canonical_bid_windows::window_end_date,
                ))
                .first(conn)
                .expect("Failed to query bid windows");
            assert_eq!(start, None, "Window start should be NULL");
            assert_eq!(end, None, "Window end should be NULL");
        }
        crate::BackendConnection::Mysql(_) => {
            panic!("This test is SQLite-specific");
        }
    }
}

/// Test that canonicalization works with zero users (`SQLite`).
#[test]
fn test_canonicalize_with_no_users_sqlite() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            diesel::sql_query(
                "INSERT INTO bid_years (bid_year_id, year, start_date, num_pay_periods, is_active, lifecycle_state)
                 VALUES (1, 2026, '2026-01-04', 26, 1, 'BootstrapComplete')",
            )
            .execute(conn)
            .expect("Failed to insert bid year");

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
                    description: String::from("Test"),
                },
                action: Action {
                    name: String::from("CanonicalizeBidYear"),
                    details: None,
                },
                before: StateSnapshot::new(String::from("before")),
                after: StateSnapshot::new(String::from("after")),
                bid_year: Some(BidYear::new(2026)),
                area: None,
            };

            let event_id = canonicalize_bid_year_sqlite(conn, 1, &audit_event)
                .expect("Canonicalization failed");

            assert!(event_id > 0);

            // Verify no canonical rows were created
            let count: i64 = canonical_area_membership::table
                .filter(canonical_area_membership::bid_year_id.eq(1))
                .count()
                .get_result(conn)
                .expect("Failed to count");
            assert_eq!(count, 0, "Should have 0 rows when no users");
        }
        crate::BackendConnection::Mysql(_) => {
            panic!("This test is SQLite-specific");
        }
    }
}

/// Test that canonicalization is idempotent (`SQLite`).
#[test]
fn test_canonicalize_idempotent_sqlite() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            diesel::sql_query(
                "INSERT INTO bid_years (bid_year_id, year, start_date, num_pay_periods, is_active, lifecycle_state)
                 VALUES (1, 2026, '2026-01-04', 26, 1, 'BootstrapComplete')",
            )
            .execute(conn)
            .expect("Failed to insert bid year");

            diesel::sql_query(
                "INSERT INTO areas (area_id, bid_year_id, area_code, area_name, is_system_area)
                 VALUES (1, 1, 'AREA1', 'Test Area', 0)",
            )
            .execute(conn)
            .expect("Failed to insert area");

            diesel::sql_query(
                "INSERT INTO users (user_id, bid_year_id, area_id, initials, name, user_type, cumulative_natca_bu_date, natca_bu_date, eod_faa_date, service_computation_date, excluded_from_bidding, excluded_from_leave_calculation)
                 VALUES (1, 1, 1, 'ABC', 'User One', 'CPC', '2020-01-01', '2020-01-01', '2020-01-01', '2020-01-01', 0, 0)",
            )
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
                    description: String::from("Test"),
                },
                action: Action {
                    name: String::from("CanonicalizeBidYear"),
                    details: None,
                },
                before: StateSnapshot::new(String::from("before")),
                after: StateSnapshot::new(String::from("after")),
                bid_year: Some(BidYear::new(2026)),
                area: None,
            };

            // First canonicalization
            let event_id_1 = canonicalize_bid_year_sqlite(conn, 1, &audit_event)
                .expect("First canonicalization failed");
            assert!(event_id_1 > 0, "First event ID should be positive");

            // Second canonicalization - should be idempotent
            let event_id_2 = canonicalize_bid_year_sqlite(conn, 1, &audit_event)
                .expect("Second canonicalization failed");
            assert_eq!(
                event_id_1, event_id_2,
                "Second canonicalization should return same event ID"
            );

            // Verify only one set of canonical rows exists
            let area_count: i64 = canonical_area_membership::table
                .filter(canonical_area_membership::bid_year_id.eq(1))
                .count()
                .get_result(conn)
                .expect("Failed to count area membership");
            assert_eq!(area_count, 1, "Should have exactly 1 area membership row");
        }
        crate::BackendConnection::Mysql(_) => {
            panic!("This test is SQLite-specific");
        }
    }
}

/// Test lifecycle-aware read routing before canonicalization (`SQLite`).
#[test]
fn test_read_routing_before_canonicalization_sqlite() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            diesel::sql_query(
                "INSERT INTO bid_years (bid_year_id, year, start_date, num_pay_periods, is_active, lifecycle_state)
                 VALUES (1, 2026, '2026-01-04', 26, 1, 'BootstrapComplete')",
            )
            .execute(conn)
            .expect("Failed to insert bid year");

            diesel::sql_query(
                "INSERT INTO areas (area_id, bid_year_id, area_code, area_name, is_system_area)
                 VALUES (1, 1, 'AREA1', 'Test Area', 0)",
            )
            .execute(conn)
            .expect("Failed to insert area");

            diesel::sql_query(
                "INSERT INTO users (user_id, bid_year_id, area_id, initials, name, user_type, cumulative_natca_bu_date, natca_bu_date, eod_faa_date, service_computation_date, excluded_from_bidding, excluded_from_leave_calculation)
                 VALUES (1, 1, 1, 'ABC', 'User One', 'CPC', '2020-01-01', '2020-01-01', '2020-01-01', '2020-01-01', 0, 0)",
            )
            .execute(conn)
            .expect("Failed to insert user");

            diesel::sql_query(
                "INSERT INTO operators (operator_id, login_name, display_name, password_hash, role, is_disabled, created_at)
                 VALUES (1, 'admin', 'Admin', 'hash', 'Admin', 0, '2026-01-01T00:00:00')",
            )
            .execute(conn)
            .expect("Failed to insert operator");

            // Before canonicalization, reads should come from users table
            let bid_year = BidYear::new(2026);
            let area = zab_bid_domain::Area::new("AREA1");

            let users = crate::queries::canonical::list_users_with_routing_sqlite(
                conn, 1, 1, &bid_year, &area,
            )
            .expect("Failed to list users");

            assert_eq!(users.len(), 1, "Should have 1 user from derived table");
            assert_eq!(users[0].initials.value(), "ABC");
        }
        crate::BackendConnection::Mysql(_) => {
            panic!("This test is SQLite-specific");
        }
    }
}

/// Test lifecycle-aware read routing after canonicalization (`SQLite`).
#[test]
fn test_read_routing_after_canonicalization_sqlite() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            diesel::sql_query(
                "INSERT INTO bid_years (bid_year_id, year, start_date, num_pay_periods, is_active, lifecycle_state)
                 VALUES (1, 2026, '2026-01-04', 26, 1, 'BootstrapComplete')",
            )
            .execute(conn)
            .expect("Failed to insert bid year");

            diesel::sql_query(
                "INSERT INTO areas (area_id, bid_year_id, area_code, area_name, is_system_area)
                 VALUES (1, 1, 'AREA1', 'Test Area', 0)",
            )
            .execute(conn)
            .expect("Failed to insert area");

            diesel::sql_query(
                "INSERT INTO users (user_id, bid_year_id, area_id, initials, name, user_type, cumulative_natca_bu_date, natca_bu_date, eod_faa_date, service_computation_date, excluded_from_bidding, excluded_from_leave_calculation)
                 VALUES (1, 1, 1, 'ABC', 'User One', 'CPC', '2020-01-01', '2020-01-01', '2020-01-01', '2020-01-01', 0, 0)",
            )
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
                    description: String::from("Test"),
                },
                action: Action {
                    name: String::from("CanonicalizeBidYear"),
                    details: None,
                },
                before: StateSnapshot::new(String::from("before")),
                after: StateSnapshot::new(String::from("after")),
                bid_year: Some(BidYear::new(2026)),
                area: None,
            };

            // Canonicalize
            canonicalize_bid_year_sqlite(conn, 1, &audit_event).expect("Canonicalization failed");

            // Update lifecycle state to Canonicalized
            diesel::sql_query(
                "UPDATE bid_years SET lifecycle_state = 'Canonicalized' WHERE bid_year_id = 1",
            )
            .execute(conn)
            .expect("Failed to update lifecycle state");

            // After canonicalization, reads should come from canonical tables
            let bid_year = BidYear::new(2026);
            let area = zab_bid_domain::Area::new("AREA1");

            let users = crate::queries::canonical::list_users_with_routing_sqlite(
                conn, 1, 1, &bid_year, &area,
            )
            .expect("Failed to list users");

            assert_eq!(users.len(), 1, "Should have 1 user from canonical table");
            assert_eq!(users[0].initials.value(), "ABC");

            // Now change the derived state (users table) - should NOT affect reads
            diesel::sql_query("UPDATE users SET area_id = 2 WHERE user_id = 1")
                .execute(conn)
                .ok(); // Might fail FK, that's fine

            // Reads should still come from canonical tables
            let users_after = crate::queries::canonical::list_users_with_routing_sqlite(
                conn, 1, 1, &bid_year, &area,
            )
            .expect("Failed to list users after derived change");

            assert_eq!(
                users_after.len(),
                1,
                "Canonical reads should be unaffected by derived changes"
            );
        }
        crate::BackendConnection::Mysql(_) => {
            panic!("This test is SQLite-specific");
        }
    }
}

/// Test that audit snapshot contains complete data (`SQLite`).
#[test]
fn test_canonicalize_audit_snapshot_sqlite() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        crate::BackendConnection::Sqlite(conn) => {
            diesel::sql_query(
                "INSERT INTO bid_years (bid_year_id, year, start_date, num_pay_periods, is_active, lifecycle_state)
                 VALUES (1, 2026, '2026-01-04', 26, 1, 'BootstrapComplete')",
            )
            .execute(conn)
            .expect("Failed to insert bid year");

            diesel::sql_query(
                "INSERT INTO areas (area_id, bid_year_id, area_code, area_name, is_system_area)
                 VALUES (1, 1, 'AREA1', 'Test Area 1', 0), (2, 1, 'AREA2', 'Test Area 2', 0)",
            )
            .execute(conn)
            .expect("Failed to insert areas");

            diesel::sql_query(
                "INSERT INTO users (user_id, bid_year_id, area_id, initials, name, user_type, cumulative_natca_bu_date, natca_bu_date, eod_faa_date, service_computation_date, excluded_from_bidding, excluded_from_leave_calculation)
                 VALUES
                 (1, 1, 1, 'ABC', 'User One', 'CPC', '2020-01-01', '2020-01-01', '2020-01-01', '2020-01-01', 0, 0),
                 (2, 1, 1, 'DEF', 'User Two', 'CPC', '2021-01-01', '2021-01-01', '2021-01-01', '2021-01-01', 0, 0),
                 (3, 1, 2, 'GHI', 'User Three', 'CPC', '2022-01-01', '2022-01-01', '2022-01-01', '2022-01-01', 0, 0)",
            )
            .execute(conn)
            .expect("Failed to insert users");

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
                    description: String::from("Test"),
                },
                action: Action {
                    name: String::from("CanonicalizeBidYear"),
                    details: None,
                },
                before: StateSnapshot::new(String::from("before")),
                after: StateSnapshot::new(String::from("after")),
                bid_year: Some(BidYear::new(2026)),
                area: None,
            };

            let event_id = canonicalize_bid_year_sqlite(conn, 1, &audit_event)
                .expect("Canonicalization failed");

            // Retrieve the audit event
            let after_json: String = crate::diesel_schema::audit_events::table
                .filter(crate::diesel_schema::audit_events::event_id.eq(event_id))
                .select(crate::diesel_schema::audit_events::after_snapshot_json)
                .first(conn)
                .expect("Failed to query audit event");

            // Parse the StateSnapshotData wrapper first
            let snapshot_wrapper: crate::data_models::StateSnapshotData =
                serde_json::from_str(&after_json).expect("Failed to parse StateSnapshotData");

            // Parse the actual snapshot from the wrapped data field
            let snapshot: crate::data_models::CanonicalizationSnapshot =
                serde_json::from_str(&snapshot_wrapper.data)
                    .expect("Failed to parse snapshot JSON");

            // Verify snapshot contents
            assert_eq!(snapshot.bid_year_id, 1);
            assert_eq!(snapshot.year, 2026);
            assert_eq!(snapshot.user_count, 3);
            assert_eq!(snapshot.area_count, 2);
            assert_eq!(snapshot.users.len(), 3);
            assert_eq!(snapshot.areas.len(), 2);

            // Verify user data
            assert!(snapshot.users.iter().any(|u| u.initials == "ABC"));
            assert!(snapshot.users.iter().any(|u| u.initials == "DEF"));
            assert!(snapshot.users.iter().any(|u| u.initials == "GHI"));

            // Verify all users have can_bid = true
            assert!(snapshot.users.iter().all(|u| u.can_bid));

            // Verify all users have NULL bid_order and windows
            assert!(snapshot.users.iter().all(|u| u.bid_order.is_none()));
            assert!(snapshot.users.iter().all(|u| u.window_start_date.is_none()));
            assert!(snapshot.users.iter().all(|u| u.window_end_date.is_none()));

            // Verify area data
            let area1 = snapshot
                .areas
                .iter()
                .find(|a| a.area_code == "AREA1")
                .expect("AREA1 not found");
            assert_eq!(area1.user_count, 2);

            let area2 = snapshot
                .areas
                .iter()
                .find(|a| a.area_code == "AREA2")
                .expect("AREA2 not found");
            assert_eq!(area2.user_count, 1);

            // Verify timestamp is present and valid
            assert!(!snapshot.timestamp.is_empty());
            // Timestamp is in unix_SECONDS format
            assert!(snapshot.timestamp.starts_with("unix_"));
        }
        crate::BackendConnection::Mysql(_) => {
            panic!("This test is SQLite-specific");
        }
    }
}
