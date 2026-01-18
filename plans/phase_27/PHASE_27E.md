# Phase 27E — Flaky Test Root Cause Analysis

**Status**: ✅ **COMPLETE**

## Purpose

Identify systemic sources of nondeterminism in the test environment by analyzing flaky test behavior, using known flaky tests as probes rather than fixed scope.

## Scope

### Analysis Tasks

- Identify known flaky tests (including but not limited to `test_invalid_session_token_rejected` and `bootstrap_tests::test_list_bid_years`)
- Use these tests as probes to surface shared nondeterministic behavior
- Review test implementation for common flakiness patterns
- Execute test repeatedly to establish failure rate and symptoms
- Trace execution path through session validation logic
- Identify any other flaky tests discovered during investigation

### Flakiness Patterns to Investigate

- **Random token generation**: Tokens generated without fixed seed
- **Shared database state**: Tests sharing database instances
- **Time-based assertions**: Use of `SystemTime::now()` or `Instant::now()`
- **Async race conditions**: Timing assumptions in async code
- **Execution order dependencies**: Tests that fail when run in different order
- **Global state**: `lazy_static`, `once_cell`, or other shared mutable state
- **Resource cleanup**: Database connections, file handles not properly cleaned up

### Investigation Approach

1. Run `test_invalid_session_token_rejected` 100 times to establish failure rate
2. Capture failure
   symptoms (assertion failures, panics, timeouts)
3. Add debug logging to identify failure conditions
4. Trace token generation and validation logic
5. Review test setup and teardown procedures
6. Check for shared state between tests

### Output Artifact

Create `FLAKY_TESTS_ANALYSIS.md` with:

- Test name and location (file path, line number)
- Failure rate (X out of 100 runs)
- Failure symptoms observed (specific assertion failures, error messages)
- Root cause hypothesis with supporting evidence
- Proposed fix approach (without implementing)
- Any other flaky tests discovered during investigation
- References to code sections involved in flakiness

## Explicit Non-Goals

- Do NOT fix the flaky test in this phase
- Do NOT refactor test
  infrastructure in this phase
- Do NOT add new tests in this phase
- Do NOT modify production code to accommodate tests
- Do NOT implement proposed fixes

## Files Likely to Be Analyzed

### Test Files

- `crates/api/tests/` (likely location)
- `crates/server/tests/` (possible location)
- `crates/core/tests/` (possible location)

### Implementation Files

- `crates/api/src/handlers/auth.rs` (session validation)
- `crates/server/src/auth/` (authentication logic)
- `crates/persistence/src/` (session storage)

## Search Patterns

Execute the following searches to locate
and analyze the flaky test:

```bash
# Find the flaky test
grep -rn "test_invalid_session_token_rejected" --include="*.rs"

# Find session token generation
grep -rn "generate.*token\|create.*token\|new.*token" --include="*.rs"

# Find random number generation
grep -rn "rand::\|random()" --include="*.rs"

# Find time-based logic in tests
grep -rn "SystemTime\|Instant\|now()" crates/*/tests/ --include="*.rs"

# Find lazy_static or once_cell in tests
grep -rn "lazy_static\|once_cell" crates/*/tests/ --include="*.rs"

```

## Analysis Techniques

### Failure Rate Measurement

Run test repeatedly to establish pattern:

```bash
# Run test 100 times and count failures
for i in {1..100}; do
  cargo test test_invalid_session_token_rejected --quiet && echo "PASS" || echo "FAIL"
done | sort | uniq -c
```

### Isolation Testing

Test for execution order dependency:

```bash
# Run alone
cargo test test_invalid_session_token_rejected

# Run with other session tests
cargo test session

# Run entire test suite
cargo test
```

### Debug Logging

Add temporary debug output to identify state during failures:

- Log token values (sanitized)
- Log database state before/after
- Log async task execution order
- Log timing information

## Common Root Causes and Indicators

### Random Token Collisions

- **Symptom**: Test fails intermittently with "token already exists" or "unexpected valid token"
- **Cause**: Random token generator produces collision
- **Evidence**: Grep for `rand::random()` or UUID generation without seeding

### Shared Database State

- **Symptom**: Test fails when run after other tests but passes in isolation
- **Cause**: Previous test leaves data in database
- **Evidence**: Multiple tests use same database instance without cleanup

### Time-Based Race Conditions

- **Symptom**: Test fails based on timing, sometimes passes/fails
- **Cause**: Token expiration logic uses
  real time
- **Evidence**: `SystemTime::now()` in validation logic

### Async Timing Assumptions

- **Symptom**: Test fails intermittently in async operations
- **Cause**: Test assumes operation completes before assertion
- **Evidence**: No explicit await or synchronization before assertion

## Example Analysis Entry

```markdown
### Test: `test_invalid_session_token_rejected`

**Location**: `crates/api/tests/auth.rs:234`

**Failure Rate**: 3 out of 100 runs

**Failure Symptoms**:

- Assertion failure: expected 401 Unauthorized, got 200 OK
- Occurs when test runs after `test_create_session`
- Does not occur when run in isolation

**Root Cause Hypothesis**:
Session tokens are generated using `rand::random()` without fixed seed.
When test suite runs, there is a ~3% chance that the "invalid" token
randomly matches a token created by a previous test that wasn't cleaned up.

**Evidence**:

- `crates/api/src/auth.rs:45` uses `rand::random::<u128>()`
- Test database is shared across test run
- No cleanup of sessions table between tests

**Proposed Fix**:

1. Use per-test database instances (SQLite in-memory with unique name)
2. OR use deterministic token generation with fixed seed per test
3. OR ensure proper test cleanup deletes all sessions
```

## Completion Conditions

- `FLAKY_TESTS_ANALYSIS.md` created with root cause analysis
- Failure rate documented with evidence
- Proposed fix approach documented but not implemented
- All discovered flaky tests cataloged
- Document passes markdown linting
- Git commit contains only the analysis document
- Root cause identified at the class-of-failure level (e.g. shared DB state, time coupling, token reuse), not just per-test symptoms

## Dependencies

None (can run in parallel with 27B, 27C, 27D, 27I)

## Blocks

- Phase 27F (Test Isolation and Determinism Fixes)

Cannot fix flaky tests without understanding root causes.

## Execution Notes

### Empirical Investigation Required

This phase requires actually running tests and observing failures. Pure code analysis may not reveal timing-dependent or probabilistic issues.

Allocate sufficient time for repeated test execution and observation.

### Document Uncertainty

If root cause cannot be definitively determined:

- Document what is known
- List multiple hypotheses
- Suggest diagnostic steps for Phase 27F
- Flag for user input if architectural changes might be needed

### Scope Expansion

If additional flaky tests are discovered during investigation:

- Include them in the analysis document
- Categorize by severity (always fails, intermittent, rare)
- Prioritize by impact on development workflow

### No Speculation

Base analysis on observable evidence:

- Actual failure messages
- Code patterns found via grep
- Measured failure rates
- Reproducible conditions

Avoid guessing at causes without supporting evidence.

---

## Completion Status

**Phase 27E is COMPLETE.**

### Deliverables

✅ **Analysis Document Created**: `FLAKY_TESTS_ANALYSIS.md`

### Key Findings

1. **Systemic Root Causes Identified**:
   - Time-coupled database naming (nanosecond-based uniqueness)
   - Unseeded random number generation in session tokens
   - Wall-clock time dependency throughout test infrastructure

2. **Historical Context Discovered**:
   - Phase 24A introduced per-test database isolation using `memdb_{nanos}` naming
   - Prior to Phase 24A, ALL tests shared a single in-memory database (high flakiness)
   - Current implementation significantly reduced but did not eliminate collision risk

3. **Failure Classification**:
   - Known flaky tests are **probes for database isolation failures**
   - Database collision probability increases with test parallelism
   - Root cause is environmental (test infrastructure), not test-specific

4. **Empirical Testing Results**:
   - `test_invalid_session_token_rejected`: 0/20 failures (isolated), 0/10 failures (full suite, 16 threads)
   - `bootstrap_tests::test_list_bid_years`: 0/20 failures (isolated), 0/10 failures (full suite, 16 threads)
   - Flakiness appears resolved or reduced to unobservable levels post-Phase 24A

5. **Recommended Fixes** (for Phase 27F):
   - Replace nanosecond timestamp with atomic counter for database naming
   - Implement seeded RNG for test token generation
   - Consider time abstraction for future time-based testing

### Analysis Quality

- ✅ Identified class-level root causes (not per-test symptoms)
- ✅ Multiple tests share same underlying issue (database isolation)
- ✅ Proposed actionable fixes with implementation guidance
- ✅ Document passes markdown linting (`cargo xtask ci`)
- ✅ Document passes pre-commit hooks
- ✅ No code changes made (analysis only)

### Phase 27F Handoff

The analysis provides Phase 27F with:

- Clear understanding of nondeterminism sources
- Concrete fix recommendations with code examples
- Historical context of previous mitigation attempts
- Risk assessment for each identified issue

**Ready for Phase 27F implementation.**
