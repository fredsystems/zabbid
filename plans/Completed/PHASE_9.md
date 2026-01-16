# PHASE_9.md

## Phase 9: Leave Accrual Calculation (Canonical, Deterministic)

### Phase 9 Goal

Implement a **pure, deterministic leave accrual calculation** for a single user within a single canonical bid year.

Phase 9 establishes the authoritative model for how leave is earned.
No persistence, bidding, or carryover logic is introduced.

---

### Phase 9 Scope

Phase 9 includes:

- Leave accrual calculation for **one user**
- Accrual across **one canonical bid year**
- Pay-period–based accrual logic
- Anniversary-based service thresholds
- 26-PP and 27-PP year handling
- Bonus-hour handling for the 6-hour tier
- Rounding behavior to full leave days
- Rich, auditable calculation output

Phase 9 explicitly excludes:

- Leave bidding
- Leave usage or depletion
- Carryover between years
- Cross-year accrual aggregation
- Persistence or database storage
- API endpoints
- Audit event emission
- Authorization or role logic
- Performance optimization

---

### Phase 9 Inputs

The calculation operates on:

- `User`
  - `service_computation_date (SCD)`
- `CanonicalBidYear`
  - `start_date` (Sunday in January)
  - `num_pay_periods` (26 or 27)
  - Derived pay periods (Sunday → Saturday)

The calculation must not depend on:

- current system time
- external state
- persistence
- API context

---

### Phase 9 Service Threshold Semantics

Years of service are determined using **anniversary-based logic**.

#### Phase 9 Rules

- Service thresholds are crossed **only on or after the calendar anniversary**
  of the user’s SCD.
- Threshold evaluation is based on the **start date of each pay period**.
- If a threshold anniversary occurs **during** a pay period:
  - That entire pay period earns the **prior accrual rate**
  - The new rate applies starting with the **next pay period**

No fractional years, rounding, or day-count division is permitted.

---

### Phase 9 Accrual Rates

Accrual rates are determined by years of service at the start of each pay period.

| Years of Service | Rate per Pay Period |
| ---------------- | ------------------- |
| < 3 years        | 4 hours             |
| ≥ 3 and < 15     | 6 hours             |
| ≥ 15             | 8 hours             |

---

### Phase 9 Bonus Hour Semantics

Users in the **6-hour tier** receive a **flat annual bonus of 4 hours**.

#### Phase 9 Bonus Hour Rules

- The bonus:
  - Is applied **once per bid year**
  - Has **no associated pay period**
  - Exists solely to reach the contractual annual total
- The bonus is **not** modeled as:
  - a virtual pay period
  - a dated accrual event

The bonus must be represented explicitly in the calculation output.

---

### Phase 9 27-Pay-Period Year Handling

For bid years with **27 pay periods**:

- The extra pay period earns leave at the **rate applicable at the start of PP #27**
- No special casing beyond normal PP logic is permitted
- Bonus hours (if applicable) are applied independently

---

### Phase 9 Rounding Rules

After all accrual calculations:

- If the total accrued hours are **not divisible by 8**:
  - The total is **rounded up** to the next full 8-hour day
- The rounding adjustment must be:
  - explicit
  - visible in the output
  - auditable

---

### Phase 9 Output (Rich Model)

The calculation must return a **rich, explainable structure**.

#### Phase 9 Required Output Fields

- Total accrued hours (after rounding)
- Total accrued days
- Whether rounding was applied
- A detailed breakdown explaining **why** the total was reached

#### Phase 9 Conceptual Output Shape

- `total_hours`
- `total_days`
- `rounded_up: bool`
- `breakdown: Vec<PayPeriodAccrual>`

Each breakdown entry must capture:

- Pay period index
- Pay period start date
- Pay period end date
- Accrual rate used
- Hours earned
- Reason (normal, transition, 27th PP, bonus)

The breakdown is part of the domain output and is **not optional**.

---

### Phase 9 Determinism Requirements

- Identical inputs must always produce identical outputs
- No randomness, clocks, or global state are permitted
- The calculation must be:
  - pure
  - side-effect free
  - repeatable

---

### Phase 9 Validation Requirements

The calculation must fail explicitly if:

- Canonical bid year validation fails
- Pay period derivation fails
- Date arithmetic overflows
- Required user fields are missing or invalid

Failures must return structured domain errors.

---

### Phase 9 Testing Requirements

Tests must demonstrate:

- Accrual for users under 3 years
- Accrual for users between 3 and 15 years
- Accrual for users 15+ years
- Transition across:
  - 3-year threshold
  - 15-year threshold
- Transitions occurring mid-pay-period
- Correct bonus hour application
- Correct handling of 27-PP years
- Correct rounding behavior
- Deterministic repeatability
- Rich breakdown correctness

Tests must not rely on persistence, APIs, or audit logs.

---

### Phase 9 Exit Criteria

Phase 9 is complete when all of the following are true:

- Leave accrual is computed correctly for one user and one bid year
- Anniversary-based service thresholds are enforced
- Bonus hours are applied correctly and explicitly
- 27-pay-period years are handled correctly
- Rounding rules are enforced and visible
- Output includes a rich, explainable breakdown
- All logic is pure and deterministic
- All validation and error paths are tested
- No persistence or API changes were introduced
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
