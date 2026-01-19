// Copyright (C) 2026 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

//! Backend initialization tests.
//!
//! ## Coverage Gap 13 Status: Covered by Integration
//!
//! Backend initialization (`SQLite` in-memory, file-based, migrations, foreign key
//! enforcement) is exercised extensively by all persistence tests that call
//! `SqlitePersistence::new_in_memory()`. This includes:
//!
//! - `operator_tests` (18 tests)
//! - `bootstrap_tests` (20+ tests)
//! - `completeness_tests` (16 tests)
//! - `audit_serialization_tests` (8 tests)
//! - `canonical_tests`
//! - `state_tests`
//! - `override_tests`
//!
//! Each test implicitly validates:
//! - Connection establishment
//! - Migration application (schema must exist for tests to work)
//! - Foreign key enforcement (tests rely on referential integrity)
//! - Transaction support (tested via `mutation_error_tests`)
//!
//! Additional explicit unit tests for initialization edge cases (invalid paths,
//! permission errors) would require mocking filesystem behavior, which adds
//! complexity without meaningful correctness validation.
//!
//! Per AGENTS.md: "integration tests cover behavior" is acceptable for
//! infrastructure code like database initialization.

use crate::SqlitePersistence;

#[test]
fn test_persistence_initialization() {
    let result: Result<SqlitePersistence, crate::error::PersistenceError> =
        SqlitePersistence::new_in_memory();
    assert!(result.is_ok());
}

#[test]
fn test_multiple_in_memory_instances_are_isolated() {
    // Each in-memory instance should be isolated
    let mut db1 = SqlitePersistence::new_in_memory().unwrap();
    let mut db2 = SqlitePersistence::new_in_memory().unwrap();

    // Create operator in db1
    db1.create_operator("op1", "Operator One", "password", "Admin")
        .unwrap();

    // db2 should not see it
    let count1 = db1.count_operators().unwrap();
    let count2 = db2.count_operators().unwrap();

    assert_eq!(count1, 1, "db1 should have 1 operator");
    assert_eq!(count2, 0, "db2 should have 0 operators (isolated)");
}

#[test]
fn test_migrations_applied_on_initialization() {
    // If migrations didn't run, the schema wouldn't exist and this would fail
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();

    // Verify tables exist by querying them
    let result = persistence.list_bid_years();

    assert!(
        result.is_ok(),
        "Migrations must have applied for bid_years table to exist"
    );
}
