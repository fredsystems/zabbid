# Phase 8.3

## Phase 8.3: Pay Period Alignment & Bid Year Temporal Semantics

### Phase 8.3 Goal

Introduce **authoritative, FAA-aligned temporal semantics** for bid years and pay periods.
This phase formalizes _when_ a bid year starts and ends, ensuring correctness for downstream
leave accrual, eligibility, and historical reconstruction.

Phase 8.3 makes bid year **time boundaries a domain invariant**, not an operator convention.

---

### Phase 8.3 Scope

Phase 8.3 includes:

- Enforcing valid bid year start-date alignment
- Formalizing pay period (PP) boundaries
- Deterministic bid year end-date derivation
- Exposing derived bid year end dates via read APIs
- Updating API tooling to reflect API surface changes

Phase 8.3 explicitly excludes:

- Leave accrual calculations
- Seniority-based logic
- Bidding rounds or bid lifecycle logic
- Capacity, limits, or eligibility rules
- UI or frontend changes
- Performance optimizations

---

### Phase 8.3 Domain Definitions

#### Bid Year Start Date

A bid year start date **must** satisfy all of the following:

- Must be a **Sunday**
- Must occur in **January**
- Does **not** need to be the first Sunday of the year
- Is provided explicitly by the operator (no inference)

Invalid start dates must fail domain validation explicitly.

---

#### Pay Period (PP) Semantics

- A pay period is exactly **14 consecutive days**
- Each pay period:
  - Starts on **Sunday**
  - Ends on **Saturday**
- Pay periods are:
  - Contiguous
  - Non-overlapping
  - Gap-free

---

#### Bid Year Duration

- A bid year consists of **exactly 26 or 27 pay periods**
- The bid year:
  - Starts on the start date of PP #1
  - Ends on the **Saturday** of the final pay period
- The end date:
  - Is **derived**, never stored independently
  - May occur in the following calendar year
  - Is not required to fall in the same year as the start date

---

### Phase 8.3 Validation Rules

The domain must reject bid years where:

- `start_date` is not a Sunday
- `start_date` is not in January
- `num_pay_periods` is not exactly 26 or 27
- Any pay period would:
  - Overflow date arithmetic
  - Break contiguity
  - Violate Sunday–Saturday boundaries

All failures must return **structured, explicit domain errors**.

---

### Phase 8.3 Canonical Model Behavior

- `CanonicalBidYear` remains the authoritative representation
- `CanonicalBidYear::new()` must:
  - Validate start-date alignment
  - Validate pay-period count
  - Derive all pay periods deterministically
  - Derive the bid year end date deterministically
- No inferred or default values are permitted

---

### Phase 8.3 API Changes

#### Read API Enhancements

- Area listing responses must include:
  - `bid_year_end_date` (derived, ISO 8601)
- End date must be derived from canonical bid year data
- No persistence of end dates as standalone fields

#### API Contract Rules

- Any API change (add/remove/rename/modify fields) must:
  - Be reflected in API request/response DTOs
  - Be reflected in `api_cli.py`
  - Maintain consistency across server, API, and tooling layers

---

### Phase 8.3 Tooling Requirements

- `api_cli.py` must be updated whenever:
  - API endpoints change
  - Request schemas change
  - Response schemas change
- CLI updates are considered **required**, not optional
- CLI behavior must remain aligned with the current API surface

---

### Phase 8.3 Audit Semantics

- Audit behavior remains unchanged
- No new audit event types are introduced
- Bid year creation continues to emit exactly one audit event on success
- Validation failures emit no audit events

---

### Phase 8.3 Testing Requirements

Tests must demonstrate:

- Rejection of non-Sunday start dates
- Rejection of non-January start dates
- Acceptance of valid January Sundays
- Correct derivation of:
  - Pay period boundaries
  - Bid year end date
- Deterministic behavior across repeated executions
- Read APIs returning correct derived end dates
- No persistence of derived-only values

All validation paths must be covered by unit tests.

---

### Phase 8.3 Exit Criteria

Phase 8.3 is complete when all of the following are true:

- Bid year start-date alignment is enforced
- Pay period boundaries are validated explicitly
- Bid year end dates are derived correctly
- Derived end dates are exposed via read APIs
- No inferred temporal logic exists
- No persistence schema stores redundant derived values
- `api_cli.py` reflects the current API surface
- All validation and derivation paths are tested
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
