# Phase 29A — User Participation Flags

## Purpose

Implement user participation flags (`excluded_from_bidding`, `excluded_from_leave_calculation`) as first-class domain data with explicit directional invariant enforcement.

These flags control:

- bid order derivation
- readiness evaluation
- round capacity calculations

They do **not** trigger execution behavior or time-based transitions.

---

## Scope

### 1. Database Schema

Add to `users` table:

- `excluded_from_bidding` (INTEGER, NOT NULL, DEFAULT 0, CHECK IN (0, 1))
- `excluded_from_leave_calculation` (INTEGER, NOT NULL, DEFAULT 0, CHECK IN (0, 1))

### 2. Domain Types

Add fields to `User` struct in `domain/src/types.rs`:

- `excluded_from_bidding: bool`
- `excluded_from_leave_calculation: bool`

### 3. Directional Invariant (Non-Negotiable)

**A user may never be included in bidding while excluded from leave calculation.**

Formally:

```text
excluded_from_leave_calculation == true
⇒ excluded_from_bidding == true
```

This invariant must be enforced:

- at domain construction
- at mutation
- at persistence write
- at API request validation

Any violation must be a blocking error.

### 4. API Endpoints

Add endpoints for updating participation flags:

- `POST /api/users/{user_id}/participation`
  - Request: `{ excluded_from_bidding: bool, excluded_from_leave_calculation: bool }`
  - Validates directional invariant
  - Returns error if invariant violated
  - Pre-confirmation: editable
  - Post-confirmation: immutable (or explicitly allowed with justification)

### 5. API Response Updates

Update `UserInfo` to include:

- `excluded_from_bidding: bool`
- `excluded_from_leave_calculation: bool`

### 6. Persistence Layer

- Add insert support for new fields
- Add update support for new fields
- Add read support for new fields
- Enforce invariant at persistence boundary

### 7. Lifecycle Constraints

- Flags are editable in `Draft` and `BootstrapComplete` states
- After confirmation/canonicalization, flags become immutable (or require explicit override)

---

## Explicit Non-Goals

- No automatic flag derivation
- No time-based flag transitions
- No bid execution logic
- No round capacity calculation (that's a later sub-phase)

---

## Completion Checklist

- [ ] Migration created for both SQLite and MySQL
- [ ] Schema verification passes (`cargo xtask verify-migrations`)
- [ ] Domain types updated
- [ ] Directional invariant enforced in domain
- [ ] Persistence layer supports new fields
- [ ] API endpoint implemented
- [ ] API response types updated
- [ ] Lifecycle constraints enforced
- [ ] Unit tests for invariant enforcement
- [ ] Integration tests for API endpoint
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes

---

## Stop-and-Ask Conditions

Stop if:

- Invariant enforcement conflicts with existing domain rules
- Lifecycle constraints are unclear or ambiguous
- Post-confirmation mutability requirements are uncertain
- Flag semantics require clarification

---

## Risk Notes

- Existing users will have default values (both false)
- Existing code may assume all users participate in bidding
- Bid order derivation logic will need updates in later sub-phases
