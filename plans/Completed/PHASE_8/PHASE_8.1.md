# Phase 8.1

## Phase 8.1: Bid Year Canonical Definition

### Phase 8.1 Purpose

Phase 8.1 formalizes the **canonical domain definition of a bid year**.

This phase establishes _what a bid year is_ in operational terms, independent of persistence, APIs, or UI concerns.
It creates the foundation required for correct pay-period modeling and leave accrual logic in later phases.

---

### Phase 8.1 Goal

Define a **deterministic, auditable, and testable bid year model** that:

- Represents real FAA leave years
- Supports both 26- and 27-pay-period years
- Enables precise pay-period derivation
- Does not depend on runtime clocks, databases, or external systems

---

### Phase 8.1 Scope

Phase 8.1 includes:

- Canonical bid year domain modeling
- Validation of bid year structural correctness
- Deterministic pay-period derivation
- Pure, side-effect-free domain logic
- Exhaustive unit tests for bid year behavior

Phase 8.1 explicitly excludes:

- Leave accrual calculations
- Seniority or eligibility logic
- Persistence schema changes
- API exposure or request handling
- Bidding, scheduling, or round logic
- Time-zone handling or clock access

---

### Phase 8.1 Canonical Bid Year Definition

A bid year MUST be defined by the following canonical inputs:

- **Bid year identifier** (human-readable, e.g. `2026`)
- **Start date** (ISO-8601 date, inclusive)
- **Number of pay periods** (26 or 27 only)

A bid year MUST NOT be defined by calendar year boundaries.

---

### Phase 8.1 Derived Bid Year Properties

From the canonical definition, the following properties MUST be derived deterministically:

- Bid year end date
- Ordered list of pay periods
- Each pay period’s:
  - index (1-based)
  - start date (inclusive)
  - end date (inclusive)

Derived properties MUST NOT be persisted as canonical data.

---

### Phase 8.1 Pay Period Semantics

- Pay periods are **bi-weekly (14 days)**
- The first pay period starts on the bid year start date
- Pay periods are contiguous and non-overlapping
- The final pay period ends exactly at the derived bid year end date
- Pay periods are immutable once derived

---

### Phase 8.1 Validation Rules

Bid year creation MUST fail explicitly if:

- The start date is invalid
- The number of pay periods is not exactly 26 or 27
- The derived end date is inconsistent with the number of pay periods
- Any derived pay period would overlap or be non-contiguous

Validation failures MUST:

- Produce structured domain errors
- Be deterministic
- Prevent any downstream use of the invalid bid year

---

### Phase 8.1 Domain Placement

Phase 8.1 logic MUST reside in the `domain` crate.

- No persistence concerns
- No API concerns
- No global state
- No time-based logic
- No side effects

Core and persistence layers MUST treat the bid year as opaque domain data.

---

### Phase 8.1 Testing Requirements

Tests MUST demonstrate:

- Creation of valid 26-pay-period bid years
- Creation of valid 27-pay-period bid years
- Correct derivation of pay period boundaries
- Deterministic behavior across repeated executions
- Explicit failure on invalid definitions
- Absence of side effects or hidden dependencies

Tests MUST be exhaustive for boundary conditions.

---

### Phase 8.1 Failure Semantics

On failure:

- No state mutation may occur
- No audit events may be emitted
- Errors MUST be structured and testable
- Failures MUST be deterministic

---

### Phase 8.1 Exit Criteria

Phase 8.1 is complete when all of the following are true:

- A canonical bid year domain model exists
- Pay periods are derived deterministically
- Invalid bid year definitions fail explicitly
- All logic is pure and side-effect free
- All validation and derivation paths are fully tested
- No persistence or API changes were required
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
