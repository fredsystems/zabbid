# Ignored Test Catalog

This document provides a complete inventory of all ignored tests in the workspace,
categorized by reason and with actionable recommendations for each.

## Summary

| Category           | Count |
| ------------------ | ----- |
| Obsolete           | 0     |
| Integration        | 9     |
| Hermetic-Candidate | 0     |
| Justified          | 0     |
| **Total**          | **9** |

## Obsolete Tests

**Phase 27F Update**: All obsolete tests have been removed. This section is now empty.

Previously removed tests:

- `test_duplicate_initials_in_different_bid_years_allowed` (removed in Phase 27F)
- `test_list_areas_isolated_by_bid_year` (removed in Phase 27F)

## Integration Tests

These tests require external infrastructure and are executed via explicit tooling.
They are correctly ignored and should remain so.

### Test: `test_mariadb_connection`

- **Location**: `crates/persistence/src/tests/backend_validation_tests.rs:100`
- **Category**: Integration
- **Reason**: "requires MariaDB via cargo xtask test-mariadb"
- **Recommendation**: Keep ignored (gated via xtask)
- **Notes**: Validates basic connection establishment to MariaDB instance.
  Correctly requires `DATABASE_URL` and `ZABBID_TEST_BACKEND=mariadb` environment variables.
  Executed via `cargo xtask test-mariadb` infrastructure.

### Test: `test_mariadb_migrations_apply_cleanly`

- **Location**: `crates/persistence/src/tests/backend_validation_tests.rs:114`
- **Category**: Integration
- **Reason**: "requires MariaDB via cargo xtask test-mariadb"
- **Recommendation**: Keep ignored (gated via xtask)
- **Notes**: Validates that database migrations apply successfully to MariaDB backend.
  This is a critical schema compatibility test that ensures migrations work across backends.
  Must remain ignored and executed only via xtask infrastructure.

### Test: `test_mariadb_foreign_key_enforcement`

- **Location**: `crates/persistence/src/tests/backend_validation_tests.rs:128`
- **Category**: Integration
- **Reason**: "requires MariaDB via cargo xtask test-mariadb"
- **Recommendation**: Keep ignored (gated via xtask)
- **Notes**: Validates that foreign key constraints are properly enforced by MariaDB backend.
  This test ensures referential integrity works correctly on production database backend.

### Test: `test_mariadb_operator_table_constraints`

- **Location**: `crates/persistence/src/tests/backend_validation_tests.rs:144`
- **Category**: Integration
- **Reason**: "requires MariaDB via cargo xtask test-mariadb"
- **Recommendation**: Keep ignored (gated via xtask)
- **Notes**: Validates UNIQUE constraint on `operators.login_name` in MariaDB.
  Tests schema-level constraint enforcement to ensure duplicate prevention works correctly.

### Test: `test_mariadb_canonical_table_foreign_keys`

- **Location**: `crates/persistence/src/tests/backend_validation_tests.rs:172`
- **Category**: Integration
- **Reason**: "requires MariaDB via cargo xtask test-mariadb"
- **Recommendation**: Keep ignored (gated via xtask)
- **Notes**: Validates foreign key constraints between canonical tables (e.g., areas → bid_years).
  Ensures referential integrity is enforced at the database level on MariaDB.

### Test: `test_mariadb_audit_event_foreign_keys`

- **Location**: `crates/persistence/src/tests/backend_validation_tests.rs:191`
- **Category**: Integration
- **Reason**: "requires MariaDB via cargo xtask test-mariadb"
- **Recommendation**: Keep ignored (gated via xtask)
- **Notes**: Validates foreign key constraint from audit_events to operators table.
  Ensures audit trail integrity is enforced by database constraints.

### Test: `test_mariadb_transaction_rollback`

- **Location**: `crates/persistence/src/tests/backend_validation_tests.rs:222`
- **Category**: Integration
- **Reason**: "requires MariaDB via cargo xtask test-mariadb"
- **Recommendation**: Keep ignored (gated via xtask)
- **Notes**: Validates transaction rollback semantics in MariaDB.
  Critical test ensuring that transaction boundaries work correctly across backends.

### Test: `test_mariadb_bid_year_unique_constraint`

- **Location**: `crates/persistence/src/tests/backend_validation_tests.rs:271`
- **Category**: Integration
- **Reason**: "requires MariaDB via cargo xtask test-mariadb"
- **Recommendation**: Keep ignored (gated via xtask)
- **Notes**: Validates UNIQUE constraint on `bid_years.year` in MariaDB.
  Ensures duplicate bid year prevention works at the schema level.

### Test: `test_mariadb_user_composite_unique_constraint`

- **Location**: `crates/persistence/src/tests/backend_validation_tests.rs:300`
- **Category**: Integration
- **Reason**: "requires MariaDB via cargo xtask test-mariadb"
- **Recommendation**: Keep ignored (gated via xtask)
- **Notes**: Validates composite UNIQUE constraint on `users(bid_year_id, area_id, initials)`.
  Ensures duplicate user prevention works correctly across the composite key in MariaDB.

## Completeness Check

- Total `#[ignore]` occurrences found: **9**
- Total catalog entries: **9**
- Unaccounted ignores: **0**

All ignored tests have been cataloged and categorized.

**Phase 27F Update**: Reduced from 11 to 9 ignored tests by removing 2 obsolete tests.

## Recommended Actions

### Completed in Phase 27F

1. ✓ Removed 2 obsolete tests that validated removed functionality:
   - `test_duplicate_initials_in_different_bid_years_allowed`
   - `test_list_areas_isolated_by_bid_year`

### No Action Required

The 9 MariaDB integration tests are correctly ignored and properly gated via
`cargo xtask test-mariadb`. They validate critical backend compatibility and
schema enforcement. These tests should remain ignored in the default test suite.

## Verification

This catalog was generated via exhaustive workspace search:

```bash
find . -name "*.rs" -type f -exec grep -Hn "^\s*#\s*\[ignore" {} \;
```

All occurrences have been documented and categorized.
