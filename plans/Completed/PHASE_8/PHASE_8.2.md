# Phase 8.2

## Phase 8.2: Canonical Bid Year Ownership & Persistence

### Phase 8.2 Goal

Phase 8.2 establishes authoritative ownership, persistence, and API surfacing of
canonical bid year metadata so the system no longer relies on placeholder or
inferred values.

This phase makes canonical bid year definition first-class, explicit input.

---

### Phase 8.2 Scope

Phase 8.2 includes:

- Updating the Create Bid Year API to accept canonical bid year metadata
- Persisting canonical bid year metadata as canonical state
- Returning canonical bid year metadata from read endpoints
- Removing placeholder canonical bid year construction from core
- Ensuring all changes remain auditable and deterministic
- Tests for success and failure paths across domain, core, API, and persistence

Phase 8.2 explicitly excludes:

- Leave accrual calculations
- Pay period expansion into persisted per-period rows
- Any bidding logic (bids, rounds, eligibility, capacity)
- Changes to rollback semantics
- Changes to snapshot semantics beyond reflecting canonical state
- UI or frontend work

---

### Phase 8.2 Canonical Bid Year Definition

A canonical bid year is defined by:

- `year: u16`
- `start_date: Date`
- `num_pay_periods: u8` (must be exactly 26 or 27)

These values are the sole authoritative definition of bid year boundaries
and pay period structure.

---

### Phase 8.2 API Requirements

The Create Bid Year endpoint must require explicit canonical metadata.

Create Bid Year requests must include:

- `year`
- `start_date`
- `num_pay_periods`

No defaults are allowed.
No inferred or derived values are allowed.

If any canonical field is missing, the request must fail explicitly.

List Bid Years responses must include canonical metadata:

- `year`
- `start_date`
- `num_pay_periods`

List Bid Years must remain read-only and side-effect free.

---

### Phase 8.2 Core Requirements

- `Command::CreateBidYear` must accept canonical bid year metadata
- Core must construct and validate `CanonicalBidYear` from provided input
- Placeholder canonical validation logic introduced in Phase 8.1.3 must be removed
- Duplicate bid year behavior remains unchanged
- Failure guarantees remain unchanged:
  - no state mutation
  - no audit event emission

---

### Phase 8.2 Persistence Requirements

- Canonical bid year metadata must be stored in canonical state
- Current-state reads must use canonical tables, not snapshots
- Canonical bid year persistence must be transactionally consistent with audit events
- Snapshots must reflect canonical bid year metadata when created

No schema changes beyond what is required to store canonical bid year metadata
are allowed.

---

### Phase 8.2 Audit Requirements

- Successful Create Bid Year emits exactly one audit event
- Audit events must include:
  - actor
  - cause
  - action performed
  - canonical bid year metadata
- Failed Create Bid Year emits no audit event

---

### Phase 8.2 Failure Semantics

Create Bid Year must fail explicitly if:

- `num_pay_periods` is not 26 or 27
- Canonical date arithmetic is invalid or overflows
- The bid year already exists
- Required canonical metadata is missing

Read endpoints must fail explicitly if canonical data is requested for
a non-existent bid year.

List Bid Years must never fail.

---

### Phase 8.2 Testing Requirements

Tests must demonstrate:

- Successful creation with valid canonical metadata (26 and 27 periods)
- Explicit failure for invalid canonical definitions
- Removal of all placeholder canonical logic
- Correct persistence of canonical bid year metadata
- Accurate read-back of canonical metadata
- No audit emission on failure
- Exactly one audit event on success
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes

---

### Phase 8.2 Exit Criteria

Phase 8.2 is complete when all of the following are true:

- Create Bid Year requires canonical metadata
- Placeholder canonical validation logic is fully removed
- Canonical bid year metadata is persisted as canonical state
- List Bid Years returns canonical metadata
- Canonical validation uses only operator-supplied data
- All success and failure paths are fully tested
- Audit semantics remain unchanged
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
