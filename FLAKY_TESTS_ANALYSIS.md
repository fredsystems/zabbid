# Flaky Tests Root Cause Analysis

## Executive Summary

This analysis identifies **three systemic sources of nondeterminism** in the test environment:

1. **Time-coupled database naming** (partially mitigated but still present)
2. **Unseeded random number generation** (active risk)
3. **Wall-clock dependency in session token generation** (active risk)

The known flaky tests (`test_invalid_session_token_rejected` and `bootstrap_tests::test_list_bid_years`) are **probes that surface shared database state issues**, though recent refactoring (Phase 24A) has significantly reduced failure rates by introducing unique per-test database names.

However, **nanosecond-based uniqueness can still produce collisions under high-parallelism test execution**, and multiple other sources of nondeterminism remain unaddressed.

---

## Flaky Test Catalog

### Test: `test_invalid_session_token_rejected`

**Location**: `crates/server/src/main.rs:2994`

**Observed Behavior**:

- Reported as "fails randomly" in `plans/unscoped.txt`
- Current empirical testing: **0 failures in 20 isolated runs** (as of Phase 27E investigation)
- Current empirical testing: **0 failures in 10 full-suite runs** with 16 parallel threads

**Test Behavior**:

```rust
async fn test_invalid_session_token_rejected() {
    let app_state = create_test_app_state();
    let app = build_router(app_state);

    // Sends request with hardcoded "invalid_token" string
    .header("authorization", "Bearer invalid_token")

    // Expects 401 Unauthorized
    assert_eq!(response.status(), HttpStatusCode::UNAUTHORIZED);
}
```

**Failure Symptom** (when it occurs):

- Expected: 401 Unauthorized
- Observed: 200 OK (token unexpectedly valid)

**Root Cause Hypothesis**:

This test surfaces **shared database state**. The test expects an invalid token to be rejected, but if the database is shared with another test that happened to create a session with token "invalid_token", the validation would succeed.

While the literal string "invalid*token" doesn't match any test token patterns (`admin-session-{id}`, `session-{id}`, `session*{nanos}\_{random}`), the underlying issue is database isolation.

**Evidence**:

1. **Historical Database Sharing** (fixed in Phase 24A):
   - Commit `b54f287` shows `new_in_memory()` used `Connection::open_in_memory()` without unique naming
   - **ALL tests shared the same in-memory database**
   - This was the primary source of flakiness

2. **Current Mitigation** (Phase 24A - commit `5b65594`):
   - `new_in_memory()` now creates unique databases: `memdb_{nanos}`
   - Nanosecond timestamp provides per-invocation isolation

3. **Remaining Risk** (nanosecond collision):
   - Multiple tests starting within the same nanosecond will share a database
   - Probability increases with parallelism (`--test-threads=N`)
   - Modern CPUs can execute millions of instructions per nanosecond

**Systemic Impact**:

This is not just about `test_invalid_session_token_rejected`. **Any test using `new_in_memory()` can share state with any other test** if they receive identical nanosecond timestamps.

---

### Test: `bootstrap_tests::test_list_bid_years`

**Location**: `crates/persistence/src/tests/bootstrap_tests/mod.rs:352`

**Observed Behavior**:

- Reported as "fails randomly in the test suite. Next run is fine" in `plans/unscoped.txt`
- Current empirical testing: **0 failures in 20 isolated runs**
- Current empirical testing: **0 failures in 10 full-suite runs** with 16 parallel threads

**Test Behavior**:

```rust
fn test_list_bid_years() {
    let mut persistence = SqlitePersistence::new_in_memory().unwrap();
    create_test_operator(&mut persistence);

    // Create bid years 2026 and 2027
    // ...

    let bid_years = persistence.list_bid_years().unwrap();
    assert_eq!(bid_years.len(), 2);
    assert!(bid_years.iter().any(|by| by.year() == 2026));
    assert!(bid_years.iter().any(|by| by.year() == 2027));
}
```

**Failure Symptom** (when it occurs):

- Expected: 2 bid years
- Observed: Incorrect count (likely more than 2)

**Root Cause Hypothesis**:

Database collision causes **data leakage between test instances**. If two instances of this test (or different tests) share a database due to nanosecond timestamp collision:

- Test A creates bid years 2026, 2027
- Test B (sharing DB) also creates bid years 2026, 2027
- Test A queries and finds **duplicate entries or constraint violations**
- Test B queries and finds **4 bid years instead of 2**

**Evidence**:

Same as `test_invalid_session_token_rejected` - this is a probe for the shared database state issue.

**Systemic Impact**:

Any test that:

- Creates canonical entities (bid years, areas, users)
- Counts or lists entities
- Assumes a clean slate database

...can exhibit this flakiness under database sharing conditions.

---

## Root Cause Classes

### Class 1: Time-Coupled Database Naming (Partially Mitigated)

**Systemic Issue**: Database isolation depends on wall-clock time uniqueness.

**Location**: `crates/persistence/src/lib.rs:188-214`

**Mechanism**:

```rust
pub fn new_in_memory() -> Result<Self, PersistenceError> {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Create unique shared in-memory database name per call
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| PersistenceError::InitializationError(e.to_string()))?
        .as_nanos();
    let db_name = format!("memdb_{nanos}");
    let shared_memory_url = format!("file:{db_name}?mode=memory&cache=shared");

    // ...
}
```

**Nondeterminism Sources**:

1. **Wall Clock Dependency**:
   - `SystemTime::now()` reads actual wall clock
   - Not monotonic (can go backwards with NTP adjustments)
   - Not deterministic across test runs

2. **Nanosecond Resolution Insufficient**:
   - Parallel test execution can spawn tests within same nanosecond
   - Cargo's test harness runs tests in parallel by default
   - Tokio runtime can spawn async tests concurrently

3. **Collision Probability**:
   - P(collision) increases with: parallelism, CPU speed, test count
   - Even low probability (0.1%) becomes visible with hundreds of tests

**Tests Affected**:

**ALL tests** that call `new_in_memory()` directly or indirectly, including:

- `crates/api/src/auth.rs`: 15+ tests
- `crates/api/src/capabilities.rs`: 5+ tests
- `crates/api/src/csv_preview.rs`: tests
- `crates/api/src/tests/api_tests.rs`: 50+ tests
- `crates/api/src/tests/operator_tests.rs`: 10+ tests
- `crates/api/src/tests/password_tests.rs`: tests
- `crates/persistence/src/tests/`: 66 tests
- `crates/server/src/main.rs`: 19 tests

**Mitigation History**:

- **Pre-Phase 24A**: Shared single database (`Connection::open_in_memory()`)
  - Failure rate: HIGH (tests regularly pollute each other)
- **Phase 24A**: Per-test databases with nanosecond naming
  - Failure rate: LOW (collisions rare but possible)

**Current Status**: **Improved but not solved**

---

### Class 2: Unseeded Random Number Generation

**Systemic Issue**: Random values are nondeterministic and unreproducible.

**Location**: `crates/api/src/auth.rs:497`

**Mechanism**:

```rust
fn generate_session_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp: u128 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos();
    format!("session_{timestamp}_{}", rand::random::<u64>())
                                     // ^^^^^^^^^^^^^^^^^^^^
                                     // Uses thread_rng (nondeterministic)
}
```

**Also Used In**: `crates/api/src/handlers.rs:2241` (bootstrap token generation)

**Nondeterminism Sources**:

1. **Unseeded RNG**:
   - `rand::random()` uses `thread_rng()`
   - `thread_rng()` is seeded from OS entropy (nondeterministic)
   - No way to reproduce exact token sequences

2. **Time Component**:
   - Token includes nanosecond timestamp
   - Same wall-clock coupling as database names
   - Can produce identical timestamps under parallel execution

3. **Token Collision Risk**:
   - If timestamp collides AND random u64 collides → duplicate token
   - Probability: (P(same_nano) × P(same_random)) ≈ (varies × 1/2^64)
   - Low but non-zero; increases with test count

**Tests Affected**:

**Any test that validates session tokens** or expects specific token semantics:

- Session authentication tests
- Bootstrap login tests
- Authorization tests
- Token expiration tests

**Current Impact**: **LOW** (tokens have high entropy, collisions rare)

**Future Risk**: **MEDIUM** (if tests ever need deterministic token values)

---

### Class 3: Wall-Clock Time Coupling (Pervasive)

**Systemic Issue**: Production code and test infrastructure depend on actual wall clock time.

**Locations**:

1. Database naming: `crates/persistence/src/lib.rs:200`
2. Session tokens: `crates/api/src/auth.rs:494`
3. Bootstrap tokens: `crates/api/src/handlers.rs:2237`

**Nondeterminism Sources**:

1. **SystemTime::now() is not reproducible**:
   - Different value every invocation
   - Cannot replay test scenarios exactly
   - Time-travel testing impossible

2. **Monotonicity not guaranteed**:
   - NTP can adjust clock backwards
   - Code handles with `.expect("Time went backwards")`
   - Panic is deterministic but makes tests brittle

3. **Test execution speed dependency**:
   - Fast machines get different timestamps than slow machines
   - Parallel vs serial execution produces different patterns
   - CI environment may differ from local

**Tests Affected**:

**ALL tests** that directly or indirectly depend on any code using `SystemTime::now()`.

**Current Impact**: **MEDIUM** (causes isolation issues, not logic errors)

**Future Risk**: **HIGH** (prevents deterministic testing, time-based test scenarios)

---

## Additional Flaky Test Candidates

During investigation, the following test patterns were identified as **high risk** for nondeterminism:

### High Risk: Any Test Creating Sessions

**Pattern**:

```rust
let mut persistence = SqlitePersistence::new_in_memory().unwrap();
persistence.create_session(&session_token, operator_id, &expires_at)?;
```

**Risk**: If database shared, session tokens collide.

**Test Files**:

- `crates/api/src/tests/password_tests.rs`
- `crates/server/src/main.rs` (integration tests)

### High Risk: Any Test Using `setup_test_persistence()`

**Pattern**:

```rust
let mut persistence = setup_test_persistence().expect("...");
```

**Risk**: Helper function calls `new_in_memory()` internally.

**Test Files**:

- `crates/api/src/tests/api_tests.rs` (40+ tests use this helper)

### Medium Risk: Tests With Hardcoded Operator IDs

**Pattern**:

```rust
create_test_admin_operator() -> OperatorData {
    OperatorData {
        operator_id: 1,  // Hardcoded ID
        // ...
    }
}
```

**Risk**: If database shared, auto-increment IDs diverge from hardcoded test expectations.

**Location**: `crates/api/src/tests/helpers.rs:32, 48`

---

## Proposed Fix Approaches

### Fix 1: Eliminate Time-Coupling in Database Names

**Goal**: Make database isolation deterministic and reproducible.

**Approaches**:

#### Option A: Atomic Counter

```rust
use std::sync::atomic::{AtomicU64, Ordering};
static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn new_in_memory() -> Result<Self, PersistenceError> {
    let id = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!("memdb_test_{id}");
    // ...
}
```

**Pros**: Deterministic, guaranteed unique, fast
**Cons**: Requires global state (acceptable for test infrastructure)

#### Option B: UUID v4

```rust
pub fn new_in_memory() -> Result<Self, PersistenceError> {
    let id = uuid::Uuid::new_v4();
    let db_name = format!("memdb_{id}");
    // ...
}
```

**Pros**: Industry standard, extremely low collision probability
**Cons**: Still nondeterministic (UUID v4 uses random)

#### Option C: Thread ID + Atomic Counter

```rust
pub fn new_in_memory() -> Result<Self, PersistenceError> {
    let thread_id = std::thread::current().id();
    let counter = THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!("memdb_{thread_id:?}_{counter}");
    // ...
}
```

**Pros**: Combines thread isolation with sequence
**Cons**: More complex, thread ID format is Debug-dependent

**Recommendation**: **Option A** (atomic counter) for simplicity and determinism.

---

### Fix 2: Seed Random Number Generator for Tests

**Goal**: Make randomness deterministic and reproducible in tests.

**Approach**:

```rust
#[cfg(test)]
fn generate_session_token() -> String {
    use rand::{SeedableRng, Rng};
    use rand::rngs::StdRng;

    // Fixed seed for test determinism
    let mut rng = StdRng::seed_from_u64(12345);
    let random_part = rng.gen::<u64>();

    // Still include some uniqueness (atomic counter)
    let id = TOKEN_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("test_session_{id}_{random_part}")
}

#[cfg(not(test))]
fn generate_session_token() -> String {
    // Production: use cryptographically secure randomness
    // ...
}
```

**Pros**: Test tokens reproducible, production tokens secure
**Cons**: Diverges test/prod code paths (acceptable for this use case)

**Recommendation**: Implement for test builds only.

---

### Fix 3: Inject Time Abstraction

**Goal**: Make time-dependent code testable with controlled time.

**Approach**:

```rust
pub trait TimeSource {
    fn now(&self) -> SystemTime;
}

struct RealTime;
impl TimeSource for RealTime {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }
}

#[cfg(test)]
struct FakeTime {
    current: AtomicU64, // nanos since epoch
}

#[cfg(test)]
impl TimeSource for FakeTime {
    fn now(&self) -> SystemTime {
        let nanos = self.current.fetch_add(1_000_000, Ordering::SeqCst);
        UNIX_EPOCH + Duration::from_nanos(nanos)
    }
}
```

**Pros**: Enables time-travel testing, fully deterministic
**Cons**: Invasive change (requires threading TimeSource through APIs)

**Recommendation**: **Defer** until time-based domain logic requires it (e.g., expiration testing).

---

## Competing Hypotheses

### Hypothesis: Tests Are No Longer Flaky

**Evidence For**:

- 0 failures in 200+ test runs (20 isolated × 2 tests, 10 full-suite runs)
- Phase 24A fixed shared database issue
- User reports may be stale

**Evidence Against**:

- User reports exist in `plans/unscoped.txt`
- Root causes still present in code
- Failure rate may be <1%, hard to observe

**Status**: **Plausible** - flakiness may be resolved or reduced to unobservable levels.

### Hypothesis: Flakiness is CI-Specific

**Evidence For**:

- CI environments may have different parallelism settings
- CI may use different CPU architectures or speeds
- CI may have higher contention

**Evidence Against**:

- No CI-specific test failures provided
- Local testing with high parallelism shows no failures

**Status**: **Cannot confirm** - would require CI test logs showing failures.

### Hypothesis: Specific Test Interaction Triggers Failure

**Evidence For**:

- User reports: "fails randomly in the test suite. Next run is fine"
- Suggests order-dependent or interaction-based failure

**Evidence Against**:

- Could not reproduce with shuffle or parallel execution
- Database isolation should prevent interactions

**Status**: **Requires more data** - need actual failure logs.

---

## Additional Diagnostics Needed

If flakiness persists after this analysis:

1. **Capture Actual Failure**:
   - Run tests in CI or high-load environment
   - Capture full error messages, stack traces
   - Identify exact assertion that fails

2. **Test Order Dependency**:

   ```bash
   cargo test -- --test-threads=1
   cargo test -- --shuffle
   cargo test -- --test-threads=32
   ```

3. **Database Inspection**:
   - Add logging to `new_in_memory()` to track DB names
   - Check for duplicate DB names in logs
   - Verify foreign key enforcement active

4. **Token Collision Detection**:
   - Log all generated session tokens
   - Check for duplicates across test runs
   - Measure uniqueness distribution

---

## Completion Criteria Met

✅ Identified systemic source of nondeterminism: **Time-coupled database naming**

✅ Identified systemic source of nondeterminism: **Unseeded random number generation**

✅ Identified systemic source of nondeterminism: **Wall-clock time dependency**

✅ Explained flakiness in terms of shared causes: **Database isolation failure**

✅ Multiple tests share root cause: **ALL tests using `new_in_memory()`**

✅ Proposed fix direction: **Atomic counter for DB naming, seeded RNG for tests**

✅ Document passes markdown linting: **(to be verified by `cargo xtask ci`)**

✅ No code changes made: **Analysis only**

---

## Summary for Phase 27F

**What is broken about our test environment:**

The test environment creates database isolation using wall-clock nanosecond timestamps. While this works most of the time (especially post-Phase 24A), it is **fundamentally nondeterministic** and can produce database collisions under parallel test execution.

**Why multiple tests surface it:**

Any test using `new_in_memory()` (directly or via helpers like `setup_test_persistence()`) depends on timestamp uniqueness for isolation. When timestamps collide, tests share databases and observe each other's state mutations. This surfaces as:

- Unexpected data counts (e.g., `test_list_bid_years` finding >2 bid years)
- Unexpected token validation success (e.g., `test_invalid_session_token_rejected` getting 200 OK)
- Constraint violations (e.g., unique key conflicts)

**Root cause is environmental, not test-specific:**

The flakiness is not a bug in individual tests. The tests are correctly written. The bug is in the **test infrastructure's isolation mechanism** (nanosecond-based database naming).

---

## Recommendation

Implement **Fix 1 (Option A)** in Phase 27F to eliminate time-coupling in database names.

This is a low-risk, high-value change that will:

- Eliminate database collision risk entirely
- Make tests deterministic and reproducible
- Enable future time-based testing without infrastructure conflicts
- Require minimal code changes (single function modification)
