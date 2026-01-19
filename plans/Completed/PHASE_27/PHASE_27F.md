# Phase 27F — Test Isolation and Determinism Fixes

## Purpose

Eliminate flaky tests and implement fixes for ignored tests where appropriate, making the test suite reliable and deterministic.

## Scope

### Implementation Tasks Based on Prior Analysis

Based on findings from Phase 27D (ignored test inventory) and Phase 27E (flaky test analysis):

- Fix `test_invalid_session_token_rejected` per root cause identified in 27E
- Remove obsolete ignored tests identified in 27D
- Unignore hermetic-candidate tests identified in 27D
- Fix any other flaky tests discovered in 27E
- Ensure all tests use isolated database instances
- Replace random token generation with deterministic seeds where appropriate
- Replace time-based logic with fixed timestamps or injectable clocks
- Ensure proper test cleanup and teardown

### Verification

- Run each previously flaky test 100+ times to confirm reliability
- Ensure no test failures due to execution order
- Verify test suite passes reliably on CI
- Confirm ignored test count has decreased appropriately

## Explicit Non-Goals

- Do NOT create xtask runners for integration tests (defer to separate phase if needed)
- Do NOT add new test coverage beyond what's needed for fixes (covered in 27H)
- Do NOT refactor test helpers beyond what's needed for isolation
- Do NOT modify production code to accommodate test requirements

## Files Likely to Be Affected

### Test Files

Files identified in Phase 27D and 27E:

- `crates/api/tests/` (session/auth tests)
- `crates/*/tests/*.rs` (tests marked obsolete or hermetic-candidate)
- Any test modules with flaky behavior

### Test Infrastructure

- Test helper functions for database setup
- Test fixture factories
- Shared test utilities

### Potentially Affected

- Token generation logic (if deterministic seeding added)
- Time providers (if injectable clock added for tests)

## Common Fix Patterns

### Pattern 1: Database Isolation

Replace shared database with per-test instances:

```rust
// Before: Shared database
lazy_static! {
    static ref TEST_DB: Database = create_test_db();
}

// After: Isolated per test
#[test]
fn test_something() {
    let db = create_isolated_test_db(); // Unique instance
    // test code
}
```

### Pattern 2: Deterministic Randomness

Replace random generation with fixed seeds:

```rust
// Before: Non-deterministic
let token = rand::random::<u128>();

// After: Deterministic per test
let mut rng = StdRng::seed_from_u64(12345);
let token = rng.gen::<u128>();
```

### Pattern 3: Time Injection

Replace direct time calls with injectable providers:

```rust
// Before: Uses system time
let now = SystemTime::now();

// After: Injectable time
fn validate_token(token: &Token, clock: &impl Clock) {
    let now = clock.now();
    // validation logic
}

// In tests
let fixed_time = FixedClock::new(specific_timestamp);
validate_token(&token, &fixed_time);
```

### Pattern 4: Proper Cleanup

Ensure test cleanup happens even on failure:

```rust
#[test]
fn test_something() {
    let db = setup_test_db();

    // Use RAII or explicit cleanup
    let _guard = DbCleanupGuard::new(&db);

    // Test code that might panic
    assert_eq!(result, expected);

    // Cleanup happens automatically via Drop
}
```

## Removal Criteria for Obsolete Tests

A test should be removed if:

- Functionality no longer exists
- Test is superseded by better tests
- Test was marked "TODO" and feature was abandoned
- Test is duplicate of another test

Before removing, verify:

- No unique behavior is being tested
- Coverage is maintained by other tests
- Removal is documented in commit message

## Unignore Criteria for Hermetic Tests

A test should be unignored if:

- Root cause of flakiness is fixed
- External dependencies are now mocked or isolated
- Test now runs reliably in isolation
- Test passes 100+ consecutive runs

## Completion Conditions

- Zero flaky tests remain (all pass 100+ consecutive runs)
- Obsolete ignored tests removed
- Hermetic tests unignored and passing
- Test suite passes 10 consecutive full runs without failure
- All tests use isolated databases or properly cleaned shared resources
- No tests rely on execution order
- Git commit focused on test reliability only
- Commit message explains what was fixed and why

## Dependencies

- Phase 27D must be complete (requires ignored test inventory)
- Phase 27E must be complete (requires flaky test root cause analysis)

## Blocks

- Phase 27G (Coverage Measurement and Gap Identification)

Coverage analysis requires reliable tests to produce meaningful results.

## Execution Notes

### Incremental Approach

Fix tests incrementally:

1. Start with highest-impact flaky tests
2. Remove obvious obsolete tests
3. Unignore tests that are trivially fixable
4. Address complex flakiness issues last

Commit fixes in logical groups for easier review.

### Verification Process

For each fixed test:

1. Run test 100 times in isolation
2. Run test with full suite
3. Run test in different execution orders
4. Verify no shared state affects behavior

Use script:

```bash
for i in {1..100}; do
  cargo test <test_name> --quiet || exit 1
done
echo "Test is reliable"
```

### When to Stop and Ask

Stop and request guidance if:

- Fix requires modifying production code architecture
- Fix requires adding new dependencies
- Root cause cannot be eliminated without breaking existing behavior
- Test appears to be testing incorrect behavior

### Test Infrastructure Changes

If multiple tests need similar fixes, it's acceptable to:

- Add test helper functions for database isolation
- Add fixture factories for deterministic data
- Add clock abstraction for time-dependent tests

Keep infrastructure changes minimal and focused on reliability.

### Documentation

For each fix, document in commit or code comments:

- What was causing flakiness
- How the fix addresses root cause
- Any assumptions made

This helps future maintainers understand test requirements.

### Post-Fix Validation

After all fixes:

1. Run entire test suite 10 times
2. Run with `--test-threads=1` (serial execution)
3. Run with parallel execution
4. Verify CI passes reliably

If any run fails, investigation must continue.
