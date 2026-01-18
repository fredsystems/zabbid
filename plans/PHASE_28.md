# Phase 28 — Canonical User Identity Enforcement

## Purpose

Eliminate all remaining uses of user initials as an identity mechanism and enforce
`user_id` as the sole canonical identifier for users at **all layers** of the system.

This phase corrects an architectural mismatch identified in Phase 27B, where
the core/domain command layer still uses initials as a selector despite
initials being mutable display metadata.

After this phase, **initials may never be used to identify, select, or mutate users**.

---

## Core Rule (Non-Negotiable)

> **Mutable domain attributes must never be used as entity identity.**

Specifically for users:

- `user_id` is the **only** identifier
- Initials are:
  - display metadata
  - validation input (e.g. uniqueness checks)
  - audit payload context
- Initials are **never**:
  - command selectors
  - lookup keys for mutation
  - persistence identifiers
  - audit correlation identifiers

This rule applies equally to:

- domain/core layer
- API layer
- persistence layer
- test helpers

---

## Scope

### 28A — Identity Misuse Audit (Re-run, Stricter)

Perform a fresh audit similar to Phase 27C, but with a stronger invariant.

#### Audit Question

“If initials were removed entirely, would this code still function correctly?”

#### Required Actions

- Enumerate every usage of:
  - `Initials`
  - lookup-by-initials helpers
  - commands that accept initials as selectors
- Classify each usage as:
  - **Allowed** (display or validation-only)
  - **Violation** (identity, selection, mutation)
- Produce a short findings report before fixes begin

Allowed usage:

- uniqueness validation
- CSV import validation
- error messages
- UI display

Forbidden usage:

- command selectors
- mutation targets
- audit identity
- persistence lookups returning `user_id`

---

### 28B — Core Command Refactor

Refactor all user-targeting commands to use `user_id` directly.

#### Required Changes

- Replace any command variants such as:
  - `UpdateUser { initials, ... }`
  - `Override* { initials, ... }`

With canonical forms:

- `UpdateUser { user_id, ... }`
- `Override* { user_id, ... }`

Initials may appear only as:

- fields being updated
- validation inputs
- payload metadata

No command may accept initials as a selector.

---

### 28C — Removal of Identity Translation Helpers

Eliminate or strictly constrain any helpers that translate initials → `user_id`.

#### Rules

- Functions that return `user_id` from initials are **forbidden**
- Validation helpers may:
  - check existence
  - check uniqueness
  - return booleans or structured validation errors
- No helper may return a canonical identifier based on initials

---

### 28D — API & Server Layer Alignment

Ensure all API handlers and server helpers:

- Accept `user_id` from the client
- Pass `user_id` through unchanged
- Never reconstruct identity from initials post-validation

Any internal helper that still accepts initials for lookup must be removed or refactored.

---

### 28E — Test Hardening & Regression Coverage

Add tests that permanently lock this invariant.

#### Required Tests

- Changing initials does not break:
  - subsequent updates
  - overrides
  - audit continuity
- Duplicate initials are allowed where policy permits
- No core command can be constructed without `user_id`
- Attempts to mutate users by initials fail at compile time or validation

Negative tests are encouraged.

---

## Explicit Non-Goals

- No UI redesign (unless strictly required for correctness)
- No bidding logic changes
- No performance optimizations
- No schema redesign beyond identity correctness
- No speculative domain rules

---

## Files Likely to Be Affected

### Backend

- `crates/core/src/command.rs`
- `crates/domain/src/user.rs`
- `crates/api/src/handlers/users.rs`
- `crates/server/src/*`
- `crates/persistence/src/*`

### Tests

- `crates/core/tests/`
- `crates/api/tests/`
- Any helpers constructing user commands

---

## Completion Conditions

- No command, helper, or mutation path accepts initials as identity
- All user mutations are keyed by `user_id`
- Initials used only for display or validation
- All tests pass
- New regression tests enforce the invariant
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes

---

## Rationale

This phase removes a subtle but dangerous inconsistency:

- Persistence and API layers already enforce canonical identity
- Core layer lagged behind with domain-vocabulary shortcuts
- That mismatch makes renames, replay, audit, and future bidding logic fragile

By enforcing identity consistency everywhere, the system becomes:

- easier to reason about
- safer to evolve
- harder to misuse
- future-proof for bidding and overrides

---

## Dependencies

- Phase 27B complete (verification done)
- Phase 27C may inform additional identity cleanups, but is not required

---

## When to Stop

If enforcing `user_id` requires:

- guessing domain intent
- altering business rules
- weakening audit guarantees

Stop and ask.
