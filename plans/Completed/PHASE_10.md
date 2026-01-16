# PHASE_10.md

## Phase 10: Leave Availability & Balance (Read-Only)

### Phase 10 Goal

Expose **deterministic, explainable leave availability** for a user within a single bid year by combining:

- Canonical leave accrual (Phase 9)
- Recorded leave usage

Phase 10 answers the question:

> _“How much leave does this user have available right now, and why?”_

This phase introduces **no new domain rules**, **no mutation**, and **no bidding behavior**.

---

### Phase 10 Scope

Phase 10 includes:

- Calculation of remaining leave balance for a user
- Read-only aggregation of:
  - Earned leave (from Phase 9)
  - Used leave (from persisted records)
- Deterministic subtraction of usage from rounded entitlement
- Rich, explainable output suitable for operators and audits
- API exposure of leave availability data

Phase 10 explicitly excludes:

- Leave bidding
- Leave reservation or locking
- Carryover between bid years
- Partial-day or fractional usage rules
- Persistence schema changes
- Mutation of leave usage records
- Authorization changes
- Performance optimization

---

### Phase 10 Core Principle

**Accrual is sealed before usage is applied.**

Formally:

earned_hours (Phase 9)
→ apply bonus
→ apply rounding to full 8-hour days
→ subtract used leave
→ available balance

yaml
Copy code

Usage **must never influence accrual or rounding behavior**.

---

### Phase 10 Inputs

For a given user and bid year:

- `CanonicalBidYear`
- `LeaveAccrualResult` (Phase 9 output)
- Set of leave usage records scoped to:
  - the same bid year
  - the same user

All inputs are read-only.

---

### Phase 10 Domain Model

#### Leave Usage

- Leave usage records represent **hours consumed**
- Usage records:
  - Are additive
  - Are immutable once written
  - Are assumed valid for Phase 10
- Phase 10 does **not** validate usage legality

---

#### Leave Availability Result

The core output must include:

- `earned_hours` (rounded, from Phase 9)
- `earned_days`
- `used_hours`
- `remaining_hours`
- `remaining_days`
- `is_exhausted` (remaining_hours == 0)
- `is_overdrawn` (remaining_hours < 0)
- Optional explanatory breakdown

---

### Phase 10 Calculation Rules

- Used hours are summed deterministically
- Remaining hours are calculated as:

remaining_hours = earned_hours - used_hours

csharp
Copy code

- Remaining days are derived as:

remaining_days = remaining_hours / 8

yaml
Copy code

- Negative balances are allowed and surfaced explicitly
- No rounding is applied after usage subtraction

---

### Phase 10 Error Handling

Phase 10 must fail explicitly if:

- Leave accrual data is missing
- Leave usage data cannot be read
- Bid year mismatch occurs between inputs

Errors must be:

- Structured
- Deterministic
- Side-effect free

---

### Phase 10 API Behavior

Read-only API endpoints must expose:

- Total earned leave (hours + days)
- Total used leave
- Remaining available leave
- Breakdown explaining:
  - accrual
  - rounding
  - usage subtraction

API responses must not:

- Mutate state
- Emit audit events
- Infer or recompute accrual logic

---

### Phase 10 Audit Semantics

- No audit events are emitted
- Reads are strictly side-effect free
- Availability queries are observational only

---

### Phase 10 Testing Requirements

Tests must demonstrate:

- Correct subtraction of used leave from rounded accrual
- Deterministic results across repeated calls
- Correct handling of:
  - zero usage
  - partial usage
  - full exhaustion
  - overdrawn balances
- No mutation of state during calculation
- Alignment with Phase 9 accrual outputs

---

### Phase 10 Exit Criteria

Phase 10 is complete when all of the following are true:

- Leave availability can be computed deterministically
- Accrual is never recomputed or altered
- Usage subtraction is explicit and auditable
- Negative balances are surfaced clearly
- API exposes availability data read-only
- No persistence or audit semantics changed
- All calculation paths are tested
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
