## Phase 14c: Test Migration & Authentication Cleanup

### Phase 14c Goal

Restore **full test coverage and CI integrity** after the Phase 14 authentication refactor, without introducing any new behavior or features.

Phase 14c exists to complete work that was intentionally deferred due to token limits and scope constraints during Phase 14b.

This phase is about **correctness, stability, and confidence**, not functionality.

---

### Phase 14c Scope

Phase 14c includes:

- Migrating all legacy tests to the new **session-based authentication model**
- Removing all temporary test gating:
  - `#[cfg(feature = "legacy_tests")]`
  - feature-based test suppression
- Updating tests to:
  - Authenticate via sessions
  - Use operator fixtures instead of inline actors
  - Respect authorization boundaries (Admin vs Bidder)
- Restoring all previously disabled tests
- Adding or refining test helpers for:
  - Operator creation
  - Session creation and teardown
  - Authenticated request execution
- Ensuring **all existing tests** pass under:
  - `cargo test --all-targets --all-features`
  - `cargo xtask ci`
- Eliminating all compiler warnings related to:
  - unused cfg flags
  - deprecated test paths
  - feature mismatches
- Verifying authentication and authorization invariants in tests:
  - 401 for unauthenticated requests
  - 403 for unauthorized operators
  - No audit events emitted on auth failures

---

### Phase 14c Explicitly Excludes

Phase 14c must **not** include:

- New API endpoints
- Changes to API request or response schemas
- UI or frontend changes
- CLI changes
- Domain rule changes
- Persistence schema changes
- Audit semantic changes
- Authorization logic changes
- Behavior changes of any kind

This phase is strictly about **test correctness and cleanup**.

---

### Test Migration Rules

- Tests must authenticate using **real session flows**, not mocks
- Operator identity must be explicit and realistic
- Authorization failures must be asserted explicitly
- Tests must not bypass server authentication middleware
- No test may inject `Actor` data directly
- Test helpers are allowed and encouraged
- Temporary compatibility shims must be removed once migration is complete

---

### CI & Tooling Requirements

Phase 14c is complete only when:

- `cargo test --all-targets --all-features` passes
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- No tests are disabled or feature-gated
- No cfg-related warnings remain
- No TODOs related to authentication migration remain

---

### Audit & Authorization Guarantees

Phase 14c must preserve all existing guarantees:

- No audit events on authentication failures
- No audit events on authorization failures
- Actor attribution remains derived exclusively from sessions
- Disabled operators cannot authenticate
- Session expiration behaves deterministically

---

### Phase 14c Exit Criteria

Phase 14c is complete when all of the following are true:

- All previously disabled tests are restored and passing
- All tests use session-based authentication
- No legacy actor-based test paths remain
- CI passes without warnings or feature hacks
- No behavior changes were introduced
- Authentication and authorization invariants are fully covered by tests
- The codebase is ready for UI and CLI authentication integration

---

### Phase 14c Philosophy

This phase exists to:

- Pay down intentional technical debt
- Restore trust in the test suite
- Make the system boring, predictable, and safe again

Only after Phase 14c is complete should any further feature or UI work proceed.
