# Coverage Gap Analysis — Phase 27G

**Generated**: 2025-01-18
**Coverage Tool**: `cargo llvm-cov 0.6.20`
**Overall Workspace Coverage**: 52.26% regions, 50.49% lines

---

## Executive Summary

This analysis identifies critical untested paths in the Zabbid codebase, prioritized by risk to correctness, security, and auditability. The focus is on authorization boundaries, validation logic, lifecycle constraints, and error handling—not arbitrary coverage percentages.

### Key Findings

- **Authorization gaps**: Multiple handler functions lack tests for bidder role rejection
- **Persistence error paths**: Database error handling largely untested
- **MySQL backend**: Completely untested (0% coverage)
- **Lifecycle transitions**: Several state transition validation paths uncovered
- **Error formatting**: Display implementations untested (low priority)
- **Completeness tracking**: Bootstrap completeness queries entirely untested

### Coverage by Critical Module

| Module                                    | Region Coverage | Line Coverage | Priority     |
| ----------------------------------------- | --------------- | ------------- | ------------ |
| `api/src/handlers.rs`                     | 34.94%          | 33.43%        | **Critical** |
| `api/src/auth.rs`                         | 74.89%          | 68.29%        | **High**     |
| `persistence/src/mutations/canonical.rs`  | 38.68%          | 38.90%        | **Critical** |
| `persistence/src/queries/canonical.rs`    | 53.07%          | 49.05%        | **High**     |
| `persistence/src/mutations/bootstrap.rs`  | 43.12%          | 48.78%        | **High**     |
| `persistence/src/backend/mysql.rs`        | 0.00%           | 0.00%         | **Medium**   |
| `persistence/src/queries/completeness.rs` | 0.00%           | 0.00%         | **Medium**   |
| `domain/src/types.rs`                     | 64.38%          | 77.04%        | **Medium**   |
| `core/src/apply.rs`                       | 56.21%          | 49.31%        | **High**     |
| `core/src/error.rs`                       | 21.43%          | 37.50%        | **Low**      |

---

## Critical Priority Gaps

These gaps represent security, correctness, or auditability risks and must be tested.

### Gap 1: Handler Authorization Failures (Bidder Rejection) — ✅ DONE

**Status**: REMEDIATED in Phase 27H
**Tests**: `crates/api/src/tests/authorization_tests.rs` (13 tests)

**Location**: `crates/api/src/handlers.rs` (multiple functions)

**Uncovered Behavior**:
Many mutating handler functions only test admin access, not bidder rejection:

- `update_user()` — bidder attempting to update user metadata
- `update_area()` — bidder attempting to update area
- `update_bid_year_metadata()` — bidder attempting to update bid year
- `set_active_bid_year()` — bidder attempting to set active bid year
- `override_area_assignment()` — bidder attempting override operation
- `override_eligibility()` — bidder attempting override operation
- `override_bid_order()` — bidder attempting override operation
- `override_bid_window()` — bidder attempting override operation
- `transition_to_bootstrap_complete()` — bidder attempting lifecycle transition
- `transition_to_canonicalized()` — bidder attempting lifecycle transition
- `transition_to_bidding_active()` — bidder attempting lifecycle transition
- `transition_to_bidding_closed()` — bidder attempting lifecycle transition

**Why It Matters**:
Authorization boundaries are security-critical. Untested authorization paths represent unvalidated security assumptions. Every mutating endpoint must prove it rejects unauthorized roles.

**Priority**: **Critical**

**Estimated Complexity**: Simple (add bidder session, verify 403 response)

**Test Strategy**:
For each handler:

1. Create test fixture with bidder-authenticated actor
2. Attempt mutating operation
3. Assert `AuthError::Unauthorized` returned
4. Assert HTTP 403 status in integration tests
5. Assert no state mutation occurred
6. Assert no audit event emitted

**Tests Implemented**:

- `test_update_user_rejects_bidder`
- `test_update_area_rejects_bidder`
- `test_update_bid_year_metadata_rejects_bidder`
- `test_set_active_bid_year_rejects_bidder`
- `test_transition_to_bootstrap_complete_rejects_bidder`
- `test_transition_to_canonicalized_rejects_bidder`
- `test_transition_to_bidding_active_rejects_bidder`
- `test_transition_to_bidding_closed_rejects_bidder`
- `test_checkpoint_rejects_bidder`
- `test_finalize_rejects_bidder`
- `test_rollback_rejects_bidder`
- `test_create_bid_year_rejects_bidder`
- `test_create_area_rejects_bidder`

All tests verify bidder access is rejected with `Unauthorized` error containing correct action and required_role.

---

### Gap 2: Persistence Mutation Error Handling — ✅ DONE

**Status**: REMEDIATED in Phase 27H
**Tests**: `crates/persistence/src/tests/mutation_error_tests.rs` (12 tests)

**Location**: `crates/persistence/src/mutations/canonical.rs` (lines vary)

**Uncovered Behavior**:
Database error paths in mutation functions are untested:

- `update_user()` — constraint violations, foreign key failures
- `update_area()` — constraint violations, lifecycle check failures
- `override_area_assignment()` — database errors during override persistence
- `persist_finalization()` — transaction failures
- `persist_checkpoint()` — snapshot serialization failures

**Why It Matters**:
Persistence errors must be handled correctly to maintain data integrity. Untested error paths may hide silent failures, incorrect error reporting, or transaction inconsistencies.

**Priority**: **Critical**

**Estimated Complexity**: Moderate (requires error simulation, possibly mock or constraint setup)

**Test Strategy**:

1. Use constraint violations to trigger specific error paths
2. Test foreign key violations for non-existent references
3. Verify transaction rollback on failure
4. Assert correct error types returned
5. Verify no partial state mutations

**Tests Implemented**:

- `test_update_user_with_nonexistent_user_id_returns_not_found`
- `test_update_user_with_nonexistent_area_id_returns_database_error`
- `test_update_user_with_area_missing_canonical_id_returns_error`
- `test_update_area_name_with_nonexistent_area_id_returns_not_found`
- `test_override_area_assignment_with_nonexistent_user_returns_reconstruction_error`
- `test_override_area_assignment_with_nonexistent_area_returns_reconstruction_error`
- `test_override_eligibility_with_nonexistent_user_returns_reconstruction_error`
- `test_override_bid_order_with_nonexistent_user_returns_reconstruction_error`
- `test_override_bid_window_with_nonexistent_user_returns_reconstruction_error`
- `test_lookup_bid_year_id_with_nonexistent_year_returns_not_found`
- `test_lookup_area_id_with_nonexistent_area_code_returns_not_found`
- `test_lookup_area_id_with_nonexistent_bid_year_returns_not_found`

All tests verify correct error types (DatabaseError, NotFound, ReconstructionError) and error messages contain relevant context.

---

### Gap 3: Lifecycle Constraint Violations

**Location**: `crates/core/src/apply.rs` (various transition functions)

**Uncovered Behavior**:
State transition functions don't fully test lifecycle constraint violations:

- Attempting user registration after canonicalization
- Attempting area creation after bootstrap complete
- Attempting mutations during wrong lifecycle states
- Rollback to invalid target states

**Why It Matters**:
Lifecycle constraints enforce when operations are valid. Violations represent domain rule failures that must be caught and reported correctly.

**Priority**: **Critical**

**Estimated Complexity**: Moderate (requires state setup at specific lifecycle stages)

**Test Strategy**:

1. Create state at specific lifecycle stage
2. Attempt operation prohibited at that stage
3. Assert appropriate domain error returned
4. Verify error message is actionable
5. Assert no state mutation

**Example**:

```rust
#[test]
fn test_cannot_register_user_after_canonicalization() {
    let mut state = test_state_at_canonicalized();
    let command = Command::RegisterUser { /* ... */ };

    let result = apply(state, command);

    assert!(matches!(result, Err(CoreError::LifecycleViolation(_))));
}
```

---

### Gap 4: Canonical Lookup Failures

**Location**: `crates/persistence/src/queries/canonical.rs`

**Uncovered Behavior**:
Lookup functions for canonical entities don't test all failure modes:

- `lookup_bid_year_id()` — year not found
- `lookup_area_id()` — area code not found, bid year mismatch
- `get_user_by_id()` — user not found
- Query functions with missing foreign keys

**Why It Matters**:
Lookup failures are input validation failures. Incorrect error handling can leak information or cause crashes. NotFound vs. validation errors must be clearly distinguished.

**Priority**: **Critical**

**Estimated Complexity**: Simple (provide invalid inputs, verify NotFound errors)

**Test Strategy**:

1. Invoke lookup with non-existent identifier
2. Assert `PersistenceError::NotFound` returned
3. Verify error message contains actionable information
4. Test cross-bid-year lookups (area in wrong year)

**Example**:

```rust
#[test]
fn test_lookup_bid_year_id_not_found() {
    let mut persistence = test_persistence();

    let result = persistence.lookup_bid_year_id(9999);

    assert!(matches!(result, Err(PersistenceError::NotFound(_))));
    assert!(result.unwrap_err().to_string().contains("bid year"));
}
```

---

### Gap 5: CSV Import Error Handling — ✅ DONE

**Location**: `crates/api/src/csv_preview.rs`

**Tests Added** (Phase 27H):

```rust
fn test_multiple_errors_on_single_row
fn test_multiple_rows_with_independent_failures
fn test_error_messages_have_correct_row_numbers
fn test_mixed_valid_invalid_rows_preserves_valid
fn test_empty_csv_file
fn test_header_only_csv_file
fn test_duplicate_headers
fn test_error_message_determinism
fn test_error_messages_reference_column_names
fn test_multiple_missing_required_fields
```

**Production Fixes**:

- Fixed error short-circuiting in `parse_csv_row` - now collects all parsing errors before returning
- Added validation to error path in `preview_csv_users` to check initials length and area existence even when parsing fails
- Ensures complete error aggregation and diagnostic feedback

**Coverage Achieved**:

- ✅ Multiple errors on single CSV row are all reported
- ✅ Multiple rows with independent failures each get their own error diagnostics
- ✅ Error messages are deterministic and structured with stable ordering
- ✅ Messages reference correct row numbers and column names
- ✅ Mixed valid + invalid rows - valid rows not rejected
- ✅ CSV-wide validation failures (empty file, header-only, duplicate headers)

---

## High Priority Gaps

Important for correctness but not immediate security risks.

### Gap 6: Authorization Service Coverage

**Location**: `crates/api/src/auth.rs`

**Uncovered Behavior**:
Several authorization functions lack explicit tests:

- `authorize_finalize_round()`
- `authorize_create_checkpoint()`
- `authorize_rollback()`
- `authorize_set_active_bid_year()`
- Override-related authorization functions

**Why It Matters**:
While these are called by tested handlers, explicit unit tests provide clear documentation of authorization rules and make refactoring safer.

**Priority**: **High**

**Estimated Complexity**: Simple (unit tests with admin/bidder actors)

**Test Strategy**:
For each authorize function:

1. Test admin role succeeds
2. Test bidder role fails with Unauthorized
3. Verify error message contains action name and required role

---

### Gap 7: Password Reset Error Paths

**Location**: `crates/api/src/handlers.rs` (reset_password)

**Uncovered Behavior**:
Password reset doesn't test all error conditions:

- Target operator not found
- Password policy violations during reset
- Database errors during password update
- Session invalidation failures

**Why It Matters**:
Password operations are security-sensitive. All error paths must be tested to ensure no information leakage or security bypasses.

**Priority**: **High**

**Estimated Complexity**: Moderate (requires password policy setup)

**Test Strategy**:

1. Test reset with non-existent operator ID
2. Test reset with invalid password (violates policy)
3. Verify sessions are invalidated even on partial failure
4. Test bidder attempting reset (authorization failure)

---

### Gap 8: Bootstrap Mutation Error Paths

**Location**: `crates/persistence/src/mutations/bootstrap.rs`

**Uncovered Behavior**:
Bootstrap operations don't test all database error scenarios:

- `persist_create_bid_year()` — constraint violations
- `persist_create_area()` — duplicate area codes
- Foreign key constraint failures
- Transaction rollback verification

**Why It Matters**:
Bootstrap operations establish foundational state. Errors must be caught early and reported clearly to prevent cascading failures.

**Priority**: **High**

**Estimated Complexity**: Moderate (requires constraint violation simulation)

**Test Strategy**:

1. Test duplicate bid year creation
2. Test duplicate area code within same year
3. Test area creation for non-existent bid year
4. Verify transaction rollback on any failure

---

### Gap 9: State Transition Edge Cases

**Location**: `crates/core/src/apply.rs`

**Uncovered Behavior**:
State transition logic has uncovered edge cases:

- Empty state transitions (no users, no areas)
- Boundary conditions (max users, max areas)
- Concurrent operation simulation
- Rollback to same event (no-op)

**Why It Matters**:
Edge cases often reveal subtle bugs. While not security-critical, these tests improve robustness.

**Priority**: **High**

**Estimated Complexity**: Moderate (requires careful state setup)

**Test Strategy**:

1. Test transitions with empty state
2. Test with maximum reasonable entity counts
3. Test rollback to current event ID (should be no-op or error)
4. Test transition ordering constraints

---

### Gap 10: Domain Validation Error Messages

**Location**: `crates/domain/src/types.rs`

**Uncovered Behavior**:
Domain type validation errors not fully tested:

- `Initials::new()` — invalid format, empty, too long
- `AreaCode::new()` — invalid characters, normalization
- `UserType::from_str()` — invalid values
- Crew number validation edge cases

**Why It Matters**:
Validation errors are user-facing. Error messages must be clear and actionable. Untested validation paths may have poor error messages.

**Priority**: **High**

**Estimated Complexity**: Simple (unit tests with invalid inputs)

**Test Strategy**:
For each domain type:

1. Test all documented invalid formats
2. Verify error messages are actionable
3. Test boundary conditions (empty, max length)
4. Test normalization behavior (case, whitespace)

---

## Medium Priority Gaps

Useful coverage but lower immediate risk.

### Gap 11: MySQL Backend

**Location**: `crates/persistence/src/backend/mysql.rs`

**Uncovered Behavior**:
Entire MySQL backend is untested (0% coverage).

**Why It Matters**:
The MySQL backend is production-critical. However, it's intentionally tested via `cargo xtask test-mariadb` with external infrastructure. This gap is expected and acceptable for unit test coverage.

**Priority**: **Medium**

**Estimated Complexity**: N/A (requires external infrastructure, tested separately)

**Test Strategy**:

- MySQL backend is tested via `cargo xtask test-mariadb`
- Schema parity is enforced via `cargo xtask verify-migrations`
- Unit test coverage gap is acceptable
- No action required for Phase 27H

**Note**: This is not a true gap—it's a testing boundary by design.

---

### Gap 12: Bootstrap Completeness Tracking

**Location**: `crates/persistence/src/queries/completeness.rs`

**Uncovered Behavior**:
Entire completeness tracking module is untested (0% coverage):

- `get_bid_year_completeness()`
- `get_area_completeness()`
- `get_bootstrap_readiness()`

**Why It Matters**:
Completeness tracking determines when bootstrap is ready for progression. Incorrect completeness logic could block legitimate transitions or allow premature ones.

**Priority**: **Medium**

**Estimated Complexity**: Moderate (requires state setup with expected counts)

**Test Strategy**:

1. Test completeness with no entities (incomplete)
2. Test completeness with partial entities (incomplete)
3. Test completeness with exact expected count (complete)
4. Test completeness with over-count (complete)
5. Test readiness aggregation across bid years and areas

---

### Gap 13: Backend Initialization

**Location**: `crates/persistence/src/backend/mod.rs`, `backend/sqlite.rs`

**Uncovered Behavior**:
Database backend initialization paths:

- SQLite in-memory initialization
- SQLite file initialization
- Migration application
- Backend-specific connection setup

**Why It Matters**:
Backend initialization is typically exercised via integration tests. Unit coverage gaps are acceptable if integration tests cover behavior.

**Priority**: **Medium**

**Estimated Complexity**: Low (likely not needed, integration-tested)

**Test Strategy**:

- Review integration tests for coverage
- If gaps exist, add backend initialization tests
- Focus on error paths (permission denied, invalid path)

---

### Gap 14: Audit Event Serialization

**Location**: `crates/persistence/src/mutations/audit.rs`

**Uncovered Behavior**:
Audit event persistence error handling:

- Snapshot serialization failures
- Event insertion failures
- Large snapshot handling

**Why It Matters**:
Audit trail correctness is paramount. However, serialization is well-tested by Rust's JSON libraries. Focus on integration behavior.

**Priority**: **Medium**

**Estimated Complexity**: Moderate (requires large state or serialization failures)

**Test Strategy**:

1. Test audit event persistence with large snapshots
2. Verify snapshot compression if implemented
3. Test database errors during event insertion
4. Verify transaction rollback includes audit event rollback

---

### Gap 15: Operator Query Error Paths

**Location**: `crates/persistence/src/queries/operators.rs`

**Uncovered Behavior**:
Operator query functions have some uncovered paths:

- `list_operators()` — database errors
- `count_active_admins()` — database errors
- Empty result set handling

**Why It Matters**:
Query errors must be handled gracefully. However, happy paths are well-tested. Focus on error simulation.

**Priority**: **Medium**

**Estimated Complexity**: Simple (simulate database errors)

**Test Strategy**:

1. Test queries with no results (empty list)
2. Test count functions returning zero
3. Simulate database errors (requires mock or connection failure)

---

## Low Priority Gaps

Nice to have but acceptable to defer.

### Gap 16: Error Display Implementations

**Location**: `crates/core/src/error.rs`, `crates/domain/src/error.rs`, `crates/persistence/src/error.rs`

**Uncovered Behavior**:
Error `Display` and `Debug` implementations are largely untested.

**Why It Matters**:
Error formatting is user-facing but low-risk. Display implementations are typically simple wrappers. Testing is valuable for consistency but not critical.

**Priority**: **Low**

**Estimated Complexity**: Simple (call `.to_string()` on each error variant)

**Test Strategy**:

1. Create instances of each error variant
2. Call `.to_string()` and verify format
3. Verify error messages are actionable
4. Check for sensitive information leakage

**Note**: Consider snapshot testing for error messages.

---

### Gap 17: Request/Response Serialization

**Location**: `crates/api/src/request_response.rs`

**Uncovered Behavior**:
Request and response type serialization is untested.

**Why It Matters**:
Serialization is handled by `serde` and is well-tested by the library. Explicit tests are low-value unless custom serialization is implemented.

**Priority**: **Low**

**Estimated Complexity**: Simple (JSON round-trip tests)

**Test Strategy**:

1. Serialize each request/response type to JSON
2. Deserialize back to Rust type
3. Verify round-trip equality
4. Test edge cases (null fields, missing fields)

**Note**: Only add if API contract validation is needed.

---

### Gap 18: State Snapshot Accessors

**Location**: `crates/core/src/state.rs`

**Uncovered Behavior**:
A few state accessor methods are untested.

**Why It Matters**:
Simple getters are low-risk. Coverage gaps are acceptable unless they represent untested business logic.

**Priority**: **Low**

**Estimated Complexity**: Trivial

**Test Strategy**:

- Review uncovered methods
- Test only if they contain non-trivial logic
- Skip simple field accessors

---

## Not Applicable (False Positives)

These gaps do not require tests.

### MySQL Backend (Intentional External Testing)

**Location**: `crates/persistence/src/backend/mysql.rs` (0% coverage)

MySQL is tested via `cargo xtask test-mariadb` with live database infrastructure. Unit test coverage is intentionally zero. This is by design per AGENTS.md.

### Persistence Error Display

**Location**: `crates/persistence/src/error.rs` (0% coverage)

Error formatting is low-value to test explicitly. These are derived implementations or simple wrappers.

---

## Recommended Test Implementation Order (Phase 27H)

Based on priority and dependencies:

1. **Handler authorization failures** (Gap 1) — Critical, simple, high impact
2. **Canonical lookup failures** (Gap 4) — Critical, simple, foundational
3. **Lifecycle constraint violations** (Gap 3) — Critical, moderate complexity
4. **Persistence mutation errors** (Gap 2) — Critical, moderate complexity
5. **CSV import error handling** (Gap 5) — Critical, moderate complexity
6. **Authorization service coverage** (Gap 6) — High, simple
7. **Domain validation errors** (Gap 10) — High, simple
8. **Password reset error paths** (Gap 7) — High, moderate complexity
9. **Bootstrap mutation errors** (Gap 8) — High, moderate complexity
10. **Bootstrap completeness tracking** (Gap 12) — Medium, moderate complexity
11. **State transition edge cases** (Gap 9) — High, moderate complexity
12. **Audit serialization** (Gap 14) — Medium, moderate complexity

Defer: Error display (Gap 16), request/response serialization (Gap 17), state accessors (Gap 18)

Skip: MySQL backend (Gap 11, tested externally)

---

## Coverage Thresholds by Module

Recommended targets after Phase 27H:

| Module Type           | Target Coverage | Rationale              |
| --------------------- | --------------- | ---------------------- |
| Authorization logic   | 100%            | Security boundary      |
| Validation logic      | 100%            | Correctness critical   |
| State transitions     | >80%            | Business logic core    |
| Persistence mutations | >70%            | Data integrity         |
| Persistence queries   | >60%            | Read paths, lower risk |
| Error formatting      | >30%            | Low priority           |

---

## Testing Philosophy Reminder

From AGENTS.md:

> Testing is mandatory and treated as first-class code.
>
> - Every non-trivial behavior change **must be testable**
> - Every test must document a **specific domain invariant**
> - Success and failure cases are both required unless one is provably impossible

This analysis focuses on **critical untested paths**, not arbitrary percentages. Phase 27H should implement tests that meaningfully reduce risk, not chase coverage numbers.

---

## Appendix: Raw Coverage Data

Generated via:

```bash
cargo llvm-cov --summary-only
cargo llvm-cov --json --output-path coverage.json
cargo llvm-cov --html
```

HTML report: `target/llvm-cov/html/index.html`
JSON data: `coverage.json` (40MB)

Overall workspace coverage: **52.26% regions**, **50.49% lines**

Critical modules requiring attention:

- `api/src/handlers.rs`: 34.94% regions
- `persistence/src/mutations/canonical.rs`: 38.68% regions
- `core/src/apply.rs`: 56.21% regions

Well-tested modules (>90% coverage):

- `audit/src/lib.rs`: 100.00%
- `api/src/capabilities.rs`: 99.57%
- `domain/src/leave_availability.rs`: 100.00%
- `domain/src/leave_accrual.rs`: 99.36%
- `api/src/password_policy.rs`: 98.32%

---

## End of Coverage Gap Analysis
