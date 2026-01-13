# Phase 22.2 — Required Active Admin Invariant

## Goal

Ensure the system can **never enter a state with zero active admin operators**.

This is a structural safety invariant enforced at the **domain/core layer**.

---

## Authoritative Domain Rule

- At least **one active Admin operator must always exist**
- An Admin operator:
  - may not be disabled if they are the last active Admin
  - may not be deleted if they are the last active Admin
- Disabled Admins do **not** count toward this requirement
- Bidder operators do **not** count toward this requirement

This rule is **non-negotiable** and must not be enforced solely by the UI.

---

## Scope

- Backend only
- Domain / Core enforcement
- API error propagation only
- No UI behavior changes except surfacing structured errors
- No persistence schema changes unless strictly required

---

## Required Work

### Domain / Core

- Enforce invariant during:
  - operator disable transitions
  - operator delete transitions
- Introduce a dedicated domain/core error, e.g.:
  - `CannotRemoveLastActiveAdmin`
- Enforcement must occur **before** persistence mutation
- No audit events emitted for rejected operations

---

### Persistence

- Query active operators by role
- Only active Admin operators count
- Disabled operators must be excluded

---

### API Layer

- Map the domain error to a structured API error
- Error message must be generic but explicit, e.g.:
  - “Operation would leave the system without an active admin”
- Must not leak operator counts or identities

---

### Tests (Required)

Add coverage for:

- Disabling the only active Admin → fails
- Deleting the only active Admin → fails
- Disabling an Admin when another active Admin exists → succeeds
- Deleting an Admin when another active Admin exists → succeeds
- Disabled Admins do not satisfy the invariant
- Bidder operators do not satisfy the invariant

---

## Exit Criteria

- It is impossible to reach a state with zero active Admins
- All new tests pass
- All existing tests continue to pass
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
