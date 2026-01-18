# Phase 27H — Coverage Gap Remediation

## Purpose

Add tests for critical coverage gaps identified in Phase 27G to ensure authorization, validation, and state transition correctness.

## Scope

### Test Implementation Based on 27G Priorities

Implement missing tests for gaps identified in Phase 27G, focusing on:

#### Authorization Failures

- Admin-only endpoint called by bidder operator
- Bidder-only endpoint called by admin operator
- Unauthenticated access to protected endpoints
- Disabled operator attempting actions
- "Last admin" protection logic (cannot disable/delete last admin)
- Capability-based action gating

#### Validation Failures

- Domain rule violations in user creation (invalid initials, duplicate detection)
- Domain rule violations in area creation
- Domain rule violations in bid year creation
- Invalid state transitions (e.g., activating already-active bid year)
- Lifecycle constraint violations (e.g., operating on inactive entities)
- Malformed input data (empty strings, out-of-range values)

#### Lifecycle Gating

- Operations on inactive bid years (should fail or be gated)
- Operations on wrong-state areas
- State transition ordering violations

#### Canonicalization Boundaries

- Foreign key violations for non-existent users
- Foreign key violations for non-existent areas
- Foreign key violations for non-existent bid years
- Cascade behavior on deletion (if applicable)

### Test Organization

Tests should be added to appropriate locations:

- Authorization tests: `crates/api/tests/authorization.rs` or per-handler test files
- Validation tests: `crates/domain/tests/` or `crates/core/tests/`
- Lifecycle tests: `crates/core/tests/lifecycle.rs` or similar
- Persistence tests: `crates/persistence/tests/`

## Explicit Non-Goals

- Do NOT aim for 100% coverage in this phase
- Do NOT test low-priority gaps (defer to future work)
- Do NOT refactor existing tests unless necessary for new test support
- Do NOT add performance or load tests
- Do NOT test third-party code or trivial derived implementations
- Do NOT modify production code to make tests easier (tests adapt to code, not vice versa)

## Files Likely to Be Affected

### New Test Files (if needed)

- `crates/api/tests/authorization.rs` (if not exists)
- `crates/core/tests/validation_failures.rs`
- `crates/persistence/tests/foreign_key_constraints.rs`

### Existing Test Files

- `crates/api/tests/*.rs` (handler tests)
- `crates/core/tests/*.rs` (state transition tests)
- `crates/domain/tests/*.rs` (domain validation tests)
- `crates/persistence/tests/*.rs` (query tests)

### Test Infrastructure

- Shared test helpers for creating operator sessions
- Fixture factories for test data
- Database test utilities

## Test Patterns

### Authorization Failure Test Pattern

```rust
#[test]
fn test_admin_only_endpoint_rejects_bidder() {
    let db = create_test_db();
    let bidder_session = create_bidder_session(&db);

    let response = call_admin_endpoint(&bidder_session);

    assert_eq!(response.status(), 403);
    assert_eq!(response.error_type(), "insufficient_permissions");
}
```

### Validation Failure Test Pattern

```rust
#[test]
fn test_user_creation_rejects_invalid_initials() {
    let db = create_test_db();

    let result = create_user_with_initials(&db, "A"); // Too short

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::InvalidInitials);
}
```

### Lifecycle Gating Test Pattern

```rust
#[test]
fn test_operation_on_inactive_bid_year_fails() {
    let db = create_test_db();
    let inactive_year = create_inactive_bid_year(&db);

    let result = perform_operation(&db, inactive_year.id());

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::BidYearNotActive);
}
```

### Foreign Key Violation Test Pattern

```rust
#[test]
fn test_create_user_with_nonexistent_area_fails() {
    let db = create_test_db();
    let nonexistent_area_id = 99999;

    let result = create_user(&db, nonexistent_area_id);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ErrorKind::AreaNotFound);
}
```

## Priority Focus

Implement tests in priority order from Phase 27G:

1. **Critical Priority First** (authorization, data integrity)
2. **High Priority Second** (validation, lifecycle)
3. **Medium Priority** (only if time permits)
4. **Low Priority** (defer to future work)

Stop after critical and high priority gaps are covered. Medium and low priority gaps can be addressed in future maintenance phases.

## Completion Conditions

- All critical priority gaps from 27G have tests
- All high priority gaps from 27G have tests
- New tests pass reliably (verified via 100+ runs for critical tests)
- No existing tests broken by new additions
- Coverage for authorization paths >90%
- Coverage for validation error paths >80%
- Git commit focused on test additions only
- Commit message references Phase 27G gap numbers or descriptions

## Dependencies

- Phase 27G must be complete (requires gap identification and prioritization)

Cannot write tests until gaps are known.

## Blocks

None (this is final test work in Phase 27)

## Execution Notes

### Incremental Implementation

Add tests incrementally by category:

1. Authorization failures (highest security impact)
2. Validation failures (data integrity impact)
3. Lifecycle gating (correctness impact)
4. Foreign key constraints (referential integrity impact)

Commit each category separately for easier review.

### Test Data Management

Reuse existing test fixtures where possible:

- Use shared helper functions for common setup
- Use fixture factories for consistent test data
- Ensure each test remains hermetic (isolated database)

Do NOT create test-specific production code just to make testing easier.

### Verification Process

For each new test:

1. Verify test fails when expected (remove fix to confirm test works)
2. Verify test passes with correct implementation
3. Run test 10+ times to ensure reliability
4. Verify test passes in full suite context

### Coverage Re-measurement

After implementing tests, re-run coverage:

```bash
cargo llvm-cov --html --open
```

Compare before/after to confirm gaps are filled. Update COVERAGE_GAPS.md to mark completed gaps.

### When to Stop

Stop implementing tests when:

- All critical priority gaps are covered
- All high priority gaps are covered
- Coverage thresholds met (>90% auth, >80% validation)

Do NOT pursue 100% coverage. Remaining gaps are acceptable if low priority.

### Documentation

For complex test scenarios, add comments explaining:

- What invariant is being tested
- Why this scenario matters
- What failure would indicate

This helps future maintainers understand test intent.

### Test Naming

Use descriptive test names that explain scenario:

- `test_admin_endpoint_rejects_bidder_access`
- `test_user_creation_fails_with_duplicate_initials`
- `test_operation_fails_on_inactive_bid_year`

Avoid generic names like `test_error` or `test_validation`.

### Handling Ambiguous Gaps

If a gap from 27G is unclear:

- Review the gap description carefully
- Check production code to understand intent
- If still unclear, ask user for clarification
- Do NOT guess at expected behavior

### Integration with Existing Tests

Ensure new tests integrate cleanly:

- Follow existing test organization patterns
- Use existing test utilities and helpers
- Match existing assertion style
- Respect existing module boundaries

Consistency improves maintainability.

### Post-Implementation Validation

After all tests added:

1. Run full test suite 10 times to verify reliability
2. Run coverage report to confirm gaps filled
3. Review test organization for clarity
4. Ensure all tests have descriptive names
5. Verify no test infrastructure leaks into production code

If any validation step fails, address before completion.
