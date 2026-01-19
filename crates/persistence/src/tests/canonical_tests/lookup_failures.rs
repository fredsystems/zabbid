// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Tests for canonical lookup failures (Phase 27H, Gap 4).
//!
//! These tests verify that lookup functions properly fail when canonical
//! references do not exist, and that error semantics are correct.

use crate::{BackendConnection, Persistence, PersistenceError};
use diesel::prelude::*;

/// Helper to setup test bid year using raw SQL.
fn setup_bid_year(conn: &mut SqliteConnection, bid_year_id: i64, year: i32) {
    diesel::sql_query(format!(
        "INSERT INTO bid_years (bid_year_id, year, start_date, num_pay_periods, is_active, lifecycle_state)
         VALUES ({bid_year_id}, {year}, '2026-01-04', 26, 0, 'Draft')"
    ))
    .execute(conn)
    .expect("Failed to insert bid year");
}

/// Helper to setup test area using raw SQL.
fn setup_area(
    conn: &mut SqliteConnection,
    area_id: i64,
    bid_year_id: i64,
    area_code: &str,
    area_name: Option<&str>,
) {
    let name = area_name.unwrap_or("Test Area");
    diesel::sql_query(format!(
        "INSERT INTO areas (area_id, bid_year_id, area_code, area_name, is_system_area)
         VALUES ({area_id}, {bid_year_id}, '{area_code}', '{name}', 0)"
    ))
    .execute(conn)
    .expect("Failed to insert area");
}

// ============================================================================
// get_bid_year_id Tests
// ============================================================================

#[test]
fn test_get_bid_year_id_not_found() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    let result = persistence.get_bid_year_id(9999);

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, PersistenceError::NotFound(_)),
        "Expected NotFound, got {err:?}"
    );

    if let PersistenceError::NotFound(msg) = err {
        assert!(
            msg.contains("9999"),
            "Error message should mention the year: {msg}"
        );
        assert!(
            msg.contains("does not exist"),
            "Error message should indicate non-existence: {msg}"
        );
    }
}

#[test]
fn test_get_bid_year_id_succeeds_after_creation() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        BackendConnection::Sqlite(conn) => {
            setup_bid_year(conn, 1, 2026);
        }
        BackendConnection::Mysql(_) => panic!("This test requires SQLite"),
    }

    let result = persistence.get_bid_year_id(2026);

    assert!(result.is_ok());
    let bid_year_id = result.unwrap();
    assert_eq!(bid_year_id, 1, "Bid year ID should match inserted value");
}

#[test]
fn test_get_bid_year_id_distinguishes_years() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        BackendConnection::Sqlite(conn) => {
            setup_bid_year(conn, 1, 2026);
            setup_bid_year(conn, 2, 2027);
        }
        BackendConnection::Mysql(_) => panic!("This test requires SQLite"),
    }

    let id_2026 = persistence.get_bid_year_id(2026).expect("Should find 2026");
    let id_2027 = persistence.get_bid_year_id(2027).expect("Should find 2027");

    assert_eq!(id_2026, 1);
    assert_eq!(id_2027, 2);
    assert_ne!(id_2026, id_2027);

    let result_2028 = persistence.get_bid_year_id(2028);
    assert!(result_2028.is_err());
}

// ============================================================================
// get_area_id Tests
// ============================================================================

#[test]
fn test_get_area_id_not_found_no_bid_year() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    let result = persistence.get_area_id(99999, "NORTH");

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, PersistenceError::NotFound(_)),
        "Expected NotFound, got {err:?}"
    );

    if let PersistenceError::NotFound(msg) = err {
        assert!(
            msg.contains("NORTH") || msg.contains("does not exist"),
            "Error message should indicate lookup failure: {msg}"
        );
    }
}

#[test]
fn test_get_area_id_not_found_wrong_area_code() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        BackendConnection::Sqlite(conn) => {
            setup_bid_year(conn, 1, 2026);
            setup_area(conn, 1, 1, "NORTH", Some("North Area"));
        }
        BackendConnection::Mysql(_) => panic!("This test requires SQLite"),
    }

    let result = persistence.get_area_id(1, "SOUTH");

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, PersistenceError::NotFound(_)),
        "Expected NotFound, got {err:?}"
    );

    if let PersistenceError::NotFound(msg) = err {
        assert!(
            msg.contains("SOUTH"),
            "Error message should mention the area code: {msg}"
        );
        assert!(
            msg.contains("does not exist"),
            "Error message should indicate non-existence: {msg}"
        );
    }
}

#[test]
fn test_get_area_id_succeeds_after_creation() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        BackendConnection::Sqlite(conn) => {
            setup_bid_year(conn, 1, 2026);
            setup_area(conn, 1, 1, "NORTH", Some("North Area"));
        }
        BackendConnection::Mysql(_) => panic!("This test requires SQLite"),
    }

    let result = persistence.get_area_id(1, "NORTH");

    assert!(result.is_ok());
    let area_id = result.unwrap();
    assert_eq!(area_id, 1, "Area ID should match inserted value");
}

#[test]
fn test_get_area_id_case_insensitive() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        BackendConnection::Sqlite(conn) => {
            setup_bid_year(conn, 1, 2026);
            setup_area(conn, 1, 1, "NORTH", Some("North Area"));
        }
        BackendConnection::Mysql(_) => panic!("This test requires SQLite"),
    }

    let result_upper = persistence.get_area_id(1, "NORTH");
    assert!(result_upper.is_ok());

    let result_lower = persistence.get_area_id(1, "north");
    assert!(result_lower.is_ok());

    assert_eq!(result_upper.unwrap(), result_lower.unwrap());
}

#[test]
fn test_get_area_id_scoped_to_bid_year() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        BackendConnection::Sqlite(conn) => {
            setup_bid_year(conn, 1, 2026);
            setup_bid_year(conn, 2, 2027);
            setup_area(conn, 1, 1, "NORTH", Some("North Area 2026"));
            setup_area(conn, 2, 2, "NORTH", Some("North Area 2027"));
            setup_area(conn, 3, 1, "SOUTH", Some("South Area 2026"));
        }
        BackendConnection::Mysql(_) => panic!("This test requires SQLite"),
    }

    let area_id_2026_north = persistence
        .get_area_id(1, "NORTH")
        .expect("Should find NORTH in 2026");
    assert_eq!(area_id_2026_north, 1);

    let area_id_2027_north = persistence
        .get_area_id(2, "NORTH")
        .expect("Should find NORTH in 2027");
    assert_eq!(area_id_2027_north, 2);

    assert_ne!(area_id_2026_north, area_id_2027_north);

    let result_2027_south = persistence.get_area_id(2, "SOUTH");
    assert!(result_2027_south.is_err(), "SOUTH should not exist in 2027");
}

// ============================================================================
// Error Mapping Tests
// ============================================================================

#[test]
fn test_lookup_errors_are_not_found_errors() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    let bid_year_err = persistence.get_bid_year_id(9999).unwrap_err();
    assert!(
        matches!(bid_year_err, PersistenceError::NotFound(_)),
        "Bid year lookup failure should be NotFound"
    );

    let area_err = persistence.get_area_id(99999, "NONEXISTENT").unwrap_err();
    assert!(
        matches!(area_err, PersistenceError::NotFound(_)),
        "Area lookup failure should be NotFound"
    );
}

#[test]
fn test_lookup_error_messages_contain_context() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    let bid_year_err = persistence.get_bid_year_id(2050).unwrap_err();
    if let PersistenceError::NotFound(msg) = bid_year_err {
        assert!(msg.contains("2050"), "Error should mention the year: {msg}");
        assert!(
            msg.contains("does not exist"),
            "Error should indicate non-existence: {msg}"
        );
    } else {
        panic!("Expected NotFound");
    }

    let area_err = persistence.get_area_id(123, "TEST").unwrap_err();
    if let PersistenceError::NotFound(msg) = area_err {
        assert!(
            msg.contains("TEST"),
            "Error should mention area code: {msg}"
        );
        assert!(
            msg.contains("does not exist"),
            "Error should indicate non-existence: {msg}"
        );
    } else {
        panic!("Expected NotFound");
    }
}

// ============================================================================
// Lookup Workflow Integration Tests
// ============================================================================

#[test]
fn test_lookup_workflow_with_multiple_entities() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        BackendConnection::Sqlite(conn) => {
            setup_bid_year(conn, 1, 2026);
            setup_area(conn, 1, 1, "NORTH", Some("North Area"));
            setup_area(conn, 2, 1, "SOUTH", Some("South Area"));
            setup_area(conn, 3, 1, "EAST", Some("East Area"));
        }
        BackendConnection::Mysql(_) => panic!("This test requires SQLite"),
    }

    let bid_year_id = persistence
        .get_bid_year_id(2026)
        .expect("Should find bid year");
    assert_eq!(bid_year_id, 1);

    let north_id = persistence
        .get_area_id(bid_year_id, "NORTH")
        .expect("Should find NORTH");
    assert_eq!(north_id, 1);

    let south_id = persistence
        .get_area_id(bid_year_id, "SOUTH")
        .expect("Should find SOUTH");
    assert_eq!(south_id, 2);

    let east_id = persistence
        .get_area_id(bid_year_id, "EAST")
        .expect("Should find EAST");
    assert_eq!(east_id, 3);

    let west_result = persistence.get_area_id(bid_year_id, "WEST");
    assert!(west_result.is_err(), "WEST should not exist");
}

#[test]
fn test_lookup_after_deletion_fails() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        BackendConnection::Sqlite(conn) => {
            setup_bid_year(conn, 1, 2026);
            setup_area(conn, 1, 1, "NORTH", Some("North Area"));
        }
        BackendConnection::Mysql(_) => panic!("This test requires SQLite"),
    }

    let result = persistence.get_area_id(1, "NORTH");
    assert!(result.is_ok(), "Should find area before deletion");

    match &mut persistence.conn {
        BackendConnection::Sqlite(conn) => {
            diesel::sql_query("DELETE FROM areas WHERE area_id = 1")
                .execute(conn)
                .expect("Failed to delete area");
        }
        BackendConnection::Mysql(_) => panic!("This test requires SQLite"),
    }

    let result = persistence.get_area_id(1, "NORTH");
    assert!(result.is_err(), "Should not find area after deletion");
}

#[test]
fn test_get_active_bid_year_fails_when_no_active() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    let result = persistence.get_active_bid_year();

    assert!(
        result.is_err(),
        "Should return error when no active bid year"
    );
}

#[test]
fn test_get_active_bid_year_returns_year_after_activation() {
    let mut persistence = Persistence::new_in_memory().expect("Failed to create persistence");

    match &mut persistence.conn {
        BackendConnection::Sqlite(conn) => {
            setup_bid_year(conn, 1, 2026);

            diesel::sql_query("UPDATE bid_years SET is_active = 1 WHERE bid_year_id = 1")
                .execute(conn)
                .expect("Failed to set active");
        }
        BackendConnection::Mysql(_) => panic!("This test requires SQLite"),
    }

    let result = persistence.get_active_bid_year();

    assert!(result.is_ok());
    let year = result.unwrap();
    assert_eq!(year, 2026);
}
