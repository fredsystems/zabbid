// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Backend validation tests for multi-database support.
//!
//! These tests validate that the persistence layer works correctly
//! across different database backends (`SQLite`, MariaDB/MySQL).
//!
//! ## Purpose
//!
//! The purpose of these tests is to ensure:
//! 1. Migrations apply cleanly on all supported backends
//! 2. Foreign key constraints are enforced correctly
//! 3. Unique constraints work as expected
//! 4. Transactions and rollback behavior is consistent
//! 5. Backend-specific behavior is documented and tested
//!
//! ## Test Execution
//!
//! - `SQLite` tests run normally via `cargo test`
//! - MariaDB/MySQL tests are marked `#[ignore]` and run only via `cargo xtask test-mariadb`
//!
//! ## Infrastructure Requirements
//!
//! `MariaDB` tests require:
//! - `DATABASE_URL` environment variable (set by xtask)
//! - `ZABBID_TEST_BACKEND=mariadb` environment variable
//! - Running `MariaDB` instance (provisioned by xtask)
//!
//! Tests fail fast if required infrastructure is missing.
//!
//! ## What These Tests Validate
//!
//! These tests focus on **infrastructure and schema compatibility**, not business logic:
//! - Schema creation and migration application
//! - Database constraint enforcement (FK, UNIQUE, CHECK)
//! - Transaction semantics
//! - Backend-specific SQL compatibility
//!
//! Business logic and domain rules are validated by the standard test suite
//! running against `SQLite`. These backend validation tests ensure the
//! persistence layer works correctly on additional databases.
//!
//! ## Adding New Backend Validation Tests
//!
//! When adding a new test:
//! 1. Mark it with `#[ignore]`
//! 2. Call `verify_mariadb_test_environment()` first
//! 3. Use raw SQL to test schema-level behavior
//! 4. Clean up test data if needed (or use transactions)
//! 5. Document what backend-specific behavior is being validated

use diesel::MysqlConnection;
use diesel::QueryableByName;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use std::env;

use crate::backend::mysql;

/// Result type for COUNT queries.
#[derive(QueryableByName)]
struct CountResult {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

/// Result type for `LAST_INSERT_ID` queries.
#[derive(QueryableByName)]
struct LastInsertIdResult {
    #[diesel(sql_type = BigInt)]
    id: i64,
}

/// Helper to get the `MariaDB` connection URL from environment.
///
/// # Panics
///
/// Panics if `DATABASE_URL` is not set, indicating missing infrastructure.
fn get_mariadb_url() -> String {
    env::var("DATABASE_URL")
        .expect("DATABASE_URL not set - MariaDB tests must be run via `cargo xtask test-mariadb`")
}

/// Helper to verify we're running in the `MariaDB` test environment.
///
/// # Panics
///
/// Panics if `ZABBID_TEST_BACKEND` is not set to `mariadb`.
fn verify_mariadb_test_environment() {
    let backend = env::var("ZABBID_TEST_BACKEND").expect(
        "ZABBID_TEST_BACKEND not set - MariaDB tests must be run via `cargo xtask test-mariadb`",
    );
    assert_eq!(backend, "mariadb", "ZABBID_TEST_BACKEND must be 'mariadb'");
}

#[test]
#[ignore = "requires MariaDB via cargo xtask test-mariadb"]
fn test_mariadb_connection() {
    verify_mariadb_test_environment();
    let url = get_mariadb_url();

    let result = MysqlConnection::establish(&url);
    assert!(
        result.is_ok(),
        "Failed to connect to MariaDB: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "requires MariaDB via cargo xtask test-mariadb"]
fn test_mariadb_migrations_apply_cleanly() {
    verify_mariadb_test_environment();
    let url = get_mariadb_url();

    let result = mysql::initialize_database(&url);
    assert!(
        result.is_ok(),
        "Failed to initialize MariaDB and run migrations: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "requires MariaDB via cargo xtask test-mariadb"]
fn test_mariadb_foreign_key_enforcement() {
    verify_mariadb_test_environment();
    let url = get_mariadb_url();

    let mut conn = mysql::initialize_database(&url).expect("Failed to initialize MariaDB database");

    let result = mysql::verify_foreign_key_enforcement(&mut conn);
    assert!(
        result.is_ok(),
        "Foreign key enforcement verification failed: {:?}",
        result.err()
    );
}

#[test]
#[ignore = "requires MariaDB via cargo xtask test-mariadb"]
fn test_mariadb_operator_table_constraints() {
    verify_mariadb_test_environment();
    let url = get_mariadb_url();

    let mut conn = mysql::initialize_database(&url).expect("Failed to initialize MariaDB database");

    // Verify unique constraint on login_name
    diesel::sql_query(
        "INSERT INTO operators (login_name, display_name, password_hash, role)
         VALUES ('test_user', 'Test User', 'hash', 'Admin')",
    )
    .execute(&mut conn)
    .expect("Failed to insert test operator");

    let duplicate_result = diesel::sql_query(
        "INSERT INTO operators (login_name, display_name, password_hash, role)
         VALUES ('test_user', 'Another User', 'hash2', 'Bidder')",
    )
    .execute(&mut conn);

    assert!(
        duplicate_result.is_err(),
        "Duplicate login_name should fail due to UNIQUE constraint"
    );
}

#[test]
#[ignore = "requires MariaDB via cargo xtask test-mariadb"]
fn test_mariadb_canonical_table_foreign_keys() {
    verify_mariadb_test_environment();
    let url = get_mariadb_url();

    let mut conn = mysql::initialize_database(&url).expect("Failed to initialize MariaDB database");

    // Try to insert area without bid_year - should fail due to FK
    let result =
        diesel::sql_query("INSERT INTO areas (bid_year_id, area_code) VALUES (99999, 'TEST')")
            .execute(&mut conn);

    assert!(
        result.is_err(),
        "Inserting area with non-existent bid_year_id should fail due to foreign key constraint"
    );
}

#[test]
#[ignore = "requires MariaDB via cargo xtask test-mariadb"]
fn test_mariadb_audit_event_foreign_keys() {
    verify_mariadb_test_environment();
    let url = get_mariadb_url();

    let mut conn = mysql::initialize_database(&url).expect("Failed to initialize MariaDB database");

    // Create an operator first
    diesel::sql_query(
        "INSERT INTO operators (login_name, display_name, password_hash, role)
         VALUES ('audit_test', 'Audit Test', 'hash', 'Admin')",
    )
    .execute(&mut conn)
    .expect("Failed to create test operator");

    // Try to insert audit event with non-existent operator - should fail
    let result = diesel::sql_query(
        "INSERT INTO audit_events
         (year, area_code, actor_operator_id, actor_login_name, actor_display_name,
          actor_json, cause_json, action_json, before_snapshot_json, after_snapshot_json)
         VALUES (2026, 'TEST', 99999, 'fake', 'Fake', '{}', '{}', '{}', '{}', '{}')",
    )
    .execute(&mut conn);

    assert!(
        result.is_err(),
        "Audit event with non-existent operator should fail due to foreign key constraint"
    );
}

#[test]
#[ignore = "requires MariaDB via cargo xtask test-mariadb"]
fn test_mariadb_transaction_rollback() {
    verify_mariadb_test_environment();
    let url = get_mariadb_url();

    let mut conn = mysql::initialize_database(&url).expect("Failed to initialize MariaDB database");

    // Begin transaction
    conn.begin_test_transaction()
        .expect("Failed to begin transaction");

    // Insert operator
    diesel::sql_query(
        "INSERT INTO operators (login_name, display_name, password_hash, role)
         VALUES ('rollback_test', 'Rollback Test', 'hash', 'Admin')",
    )
    .execute(&mut conn)
    .expect("Failed to insert operator");

    // Verify operator exists within transaction
    let count: i64 = diesel::sql_query(
        "SELECT COUNT(*) as count FROM operators WHERE login_name = 'rollback_test'",
    )
    .get_result::<CountResult>(&mut conn)
    .map(|r| r.count)
    .expect("Failed to count operators");

    assert_eq!(count, 1, "Operator should exist within transaction");

    // Transaction will rollback when conn is dropped (test transaction mode)
    drop(conn);

    // Reconnect and verify rollback
    let mut new_conn = mysql::initialize_database(&url).expect("Failed to reconnect to MariaDB");

    let count_after: i64 = diesel::sql_query(
        "SELECT COUNT(*) as count FROM operators WHERE login_name = 'rollback_test'",
    )
    .get_result::<CountResult>(&mut new_conn)
    .map(|r| r.count)
    .expect("Failed to count operators after rollback");

    assert_eq!(
        count_after, 0,
        "Operator should not exist after transaction rollback"
    );
}

#[test]
#[ignore = "requires MariaDB via cargo xtask test-mariadb"]
fn test_mariadb_bid_year_unique_constraint() {
    verify_mariadb_test_environment();
    let url = get_mariadb_url();

    let mut conn = mysql::initialize_database(&url).expect("Failed to initialize MariaDB database");

    // Insert a bid year
    diesel::sql_query(
        "INSERT INTO bid_years (year, start_date, num_pay_periods)
         VALUES (2026, '2026-01-01', 26)",
    )
    .execute(&mut conn)
    .expect("Failed to insert bid year");

    // Try to insert duplicate year - should fail
    let result = diesel::sql_query(
        "INSERT INTO bid_years (year, start_date, num_pay_periods)
         VALUES (2026, '2026-06-01', 27)",
    )
    .execute(&mut conn);

    assert!(
        result.is_err(),
        "Duplicate bid year should fail due to UNIQUE constraint"
    );
}

#[test]
#[ignore = "requires MariaDB via cargo xtask test-mariadb"]
fn test_mariadb_user_composite_unique_constraint() {
    verify_mariadb_test_environment();
    let url = get_mariadb_url();

    let mut conn = mysql::initialize_database(&url).expect("Failed to initialize MariaDB database");

    // Create bid year and area with unique year to avoid conflicts with other tests
    diesel::sql_query(
        "INSERT INTO bid_years (year, start_date, num_pay_periods)
         VALUES (2099, '2099-01-01', 26)",
    )
    .execute(&mut conn)
    .expect("Failed to insert bid year");

    let bid_year_id: i64 = diesel::sql_query("SELECT LAST_INSERT_ID() as id")
        .get_result::<LastInsertIdResult>(&mut conn)
        .map(|r| r.id)
        .expect("Failed to get bid_year_id");

    diesel::sql_query(format!(
        "INSERT INTO areas (bid_year_id, area_code) VALUES ({bid_year_id}, 'ZAB')"
    ))
    .execute(&mut conn)
    .expect("Failed to insert area");

    let area_id: i64 = diesel::sql_query("SELECT LAST_INSERT_ID() as id")
        .get_result::<LastInsertIdResult>(&mut conn)
        .map(|r| r.id)
        .expect("Failed to get area_id");

    // Insert user
    diesel::sql_query(format!(
        "INSERT INTO users
         (bid_year_id, area_id, initials, name, user_type,
          cumulative_natca_bu_date, natca_bu_date, eod_faa_date, service_computation_date,
          excluded_from_bidding, excluded_from_leave_calculation)
         VALUES ({bid_year_id}, {area_id}, 'ABC', 'Test User', 'CPC',
                 '2020-01-01', '2020-01-01', '2020-01-01', '2020-01-01', 0, 0)"
    ))
    .execute(&mut conn)
    .expect("Failed to insert user");

    // Try to insert duplicate (bid_year_id, area_id, initials) - should fail
    let result = diesel::sql_query(format!(
        "INSERT INTO users
         (bid_year_id, area_id, initials, name, user_type,
          cumulative_natca_bu_date, natca_bu_date, eod_faa_date, service_computation_date,
          excluded_from_bidding, excluded_from_leave_calculation)
         VALUES ({bid_year_id}, {area_id}, 'ABC', 'Another User', 'CPC',
                 '2021-01-01', '2021-01-01', '2021-01-01', '2021-01-01', 0, 0)"
    ))
    .execute(&mut conn);

    assert!(
        result.is_err(),
        "Duplicate user (same bid_year, area, initials) should fail due to UNIQUE constraint"
    );
}
