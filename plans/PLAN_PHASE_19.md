# Phase 19

## Phase 19 — Active Bid Year Invariant Enforcement & Domain Hardening

### Phase 19 Objective

Phase 19 exists to **restore and enforce the core domain invariant**:

> All mutating domain actions apply exclusively to the **single active bid year**, and this invariant must be enforced **in the backend**, not the UI.

This phase intentionally **removes flexibility** from the API surface in order to guarantee correctness, auditability, and long-term maintainability.

---

### Phase 19 Scope

Phase 19 includes:

- Enforcing the “exactly one active bid year” invariant at the **domain and API layers**
- Removing bid-year scoping from all mutating commands
- Ensuring all mutations implicitly target the active bid year
- Explicitly rejecting mutations when no active bid year exists
- Adding or tightening domain errors where invariants are violated
- Updating tests to reflect invariant-driven behavior

Phase 19 explicitly excludes:

- UI or frontend changes
- Navigation or routing changes
- CSV import logic changes
- User identifier (ID) refactors
- Styling, ergonomics, or accessibility work
- New features or workflow enhancements

---

### Phase 19 Core Domain Rules

#### Active Bid Year Invariant

- At most **one** bid year may be active at any time
- Exactly **one** bid year **must** be active for any mutation to occur
- If no active bid year exists:
  - All mutating operations must fail explicitly
  - No audit events are emitted
- The active bid year is:
  - Canonical state
  - Backend-owned
  - Not inferred from UI state

---

### Phase 19 Command Semantics

For all **mutating operations** (examples include but are not limited to):

- Creating areas
- Creating users
- Editing users
- Importing users
- Setting expected counts
- Any bootstrap-related mutation

The following rules apply:

- Commands **must not accept** a bid year parameter
- Commands **must internally resolve** the active bid year
- Commands **must fail explicitly** if no active bid year exists
- Commands **must never mutate** non-active bid years

Read-only operations may continue to accept bid year parameters where appropriate.

---

### Phase 19 API Contract Changes

- All mutating API requests:
  - Remove any bid year fields from request payloads
  - Implicitly operate on the active bid year
- API error responses must include:
  - Explicit error variants for missing or invalid active bid year
- No new API endpoints are introduced in this phase

---

### Phase 19 Error Handling

The domain must explicitly surface:

- `NoActiveBidYear` — when a mutation is attempted without an active bid year
- `MultipleBidYearsActive` — if invariant violation is detected internally

Errors must be:

- Typed
- Deterministic
- Testable
- Free of UI assumptions

---

### Phase 19 Audit Semantics

- All successful mutations continue to emit audit events
- Audit events implicitly reference the active bid year
- No audit events are emitted for:
  - Authorization failures
  - Validation failures
  - Missing active bid year errors

---

### Phase 19 Testing Requirements

Tests must demonstrate:

- Mutations succeed only when exactly one active bid year exists
- Mutations fail when no active bid year exists
- Mutations never affect inactive bid years
- No API allows bypassing the active bid year invariant
- All failure paths are explicitly tested
- No UI assumptions are embedded in domain or API logic

All existing tests must be updated to comply with the invariant.

---

### Phase 19 Exit Criteria

Phase 19 is complete when all of the following are true:

- All mutating domain actions require an active bid year
- No mutating API accepts a bid year identifier
- The active bid year invariant is enforced in the backend
- Domain violations fail explicitly and deterministically
- All affected tests are updated and passing
- No UI changes were required
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

---

### Phase 19 Rationale

This phase intentionally prioritizes **correctness over flexibility**.

By enforcing bid year scoping at the domain level:

- UI bugs cannot corrupt system state
- CSV imports cannot target incorrect bid years
- Future refactors (user IDs, workflows, UI polish) are simplified
- Audit trails remain trustworthy

All subsequent phases assume this invariant as foundational.
