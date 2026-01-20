# Phase 29D — Readiness Evaluation

## Purpose

Implement domain-ready evaluation logic that determines whether a bid year is structurally complete and ready for confirmation.

Readiness is **computed**, not stored. It's a pure function of current state.

Passing readiness checks does **not** automatically enter bidding. Confirmation is a separate, explicit, irreversible action (Sub-Phase 29E).

---

## Scope

### 1. Readiness Criteria

A bid year is **domain-ready** when all of the following are true:

#### Structural Completeness

- All non-system areas exist
- All non-system areas have at least one round configured
- All rounds reference valid round groups
- Expected user counts are satisfied for all non-system areas

#### No Bid Area Review

- All users in "No Bid" (system area) have been reviewed
- Review means:
  - moved to a non-system area, OR
  - explicitly confirmed to remain in No Bid

#### Participation Flag Invariants

- No user violates the directional invariant:

  ```text
  excluded_from_leave_calculation == true
  ⇒ excluded_from_bidding == true
  ```

#### Bid Order Totality

- Bid order determination produces a strict total ordering
- No two users resolve to the same seniority position
- No seniority conflicts exist
- This is **non-negotiable** — there is no manual resolution path

#### Bid Schedule Configuration

- Bid schedule is set and valid:
  - timezone is valid IANA identifier
  - start_date is a Monday and is in the future
  - window_start_time < window_end_time
  - bidders_per_day > 0

### 2. Readiness Computation

Readiness is computed by evaluating all criteria above.

The result is a structured response containing:

- `is_ready: bool` — overall readiness flag
- `blocking_reasons: Vec<String>` — list of all unsatisfied criteria
- Detailed breakdowns per criterion

### 3. API Endpoint

#### Get Readiness Status

- `GET /api/bid-years/{bid_year_id}/readiness`
  - Computes and returns readiness status
  - Does not mutate state
  - Returns:

    ```json
    {
      "bid_year_id": 1,
      "year": 2026,
      "is_ready": false,
      "blocking_reasons": [
        "Area 'North' has no rounds configured",
        "3 users in No Bid area have not been reviewed",
        "Bid schedule is not set"
      ],
      "details": {
        "areas_missing_rounds": ["North"],
        "no_bid_users_pending_review": 3,
        "participation_flag_violations": 0,
        "seniority_conflicts": 0,
        "bid_schedule_set": false
      }
    }
    ```

### 4. System Area Exclusion

- System areas (e.g., No Bid) are excluded from:
  - expected area count checks
  - round configuration requirements
  - capacity calculations

- System area users **may still bid**, but:
  - do not compete against others
  - are not subject to round constraints

### 5. Seniority Conflict Detection

This sub-phase must implement **seniority conflict detection**.

#### Requirements

- Compute bid order for all non-excluded users
- Detect if any two users resolve to the same position
- A conflict is a **blocking error** — there is no UI-based resolution

#### Conflict Causes

Conflicts may arise from:

- Identical seniority dates
- Missing lottery values when needed
- Incorrect tie-breaking logic

#### Resolution

If a conflict exists:

- readiness must be blocked
- the error must be explicit
- the underlying data or logic must be fixed
- **no manual override or UI correction is permitted**

### 6. No Bid User Review Tracking

Users in No Bid area must be tracked as "reviewed" or "pending review".

#### Implementation Options

**Option A:** Add `no_bid_reviewed` flag to `users` table

- Default: false
- Set to true when user is moved out of No Bid OR explicitly confirmed

**Option B:** Use audit log to track review events

- Check for explicit "ReviewNoBidUser" audit event

**Recommended:** Option A (simpler, more explicit)

#### Review Action

Add API endpoint:

- `POST /api/users/{user_id}/review-no-bid`
  - Marks user as reviewed
  - Only applies to users in No Bid area
  - Returns error if user is not in No Bid

### 7. Integration with Bootstrap Completeness

Existing bootstrap completeness logic may overlap with readiness.

**Decision:**

- Readiness is a **superset** of bootstrap completeness
- Bootstrap completeness checks basic data presence
- Readiness checks structural correctness and rule compliance
- Both may coexist, or readiness may replace bootstrap completeness

If bootstrap completeness is retained, it should be a prerequisite for readiness.

---

## Explicit Non-Goals

- No automatic confirmation
- No bid execution logic
- No UI for readiness dashboard (out of scope for Phase 29)
- No seniority conflict resolution UI
- No manual override of readiness checks

---

## Completion Checklist

- [ ] Readiness criteria implemented
- [ ] Seniority conflict detection implemented
- [ ] No Bid user review tracking implemented (flag or audit)
- [ ] System area exclusion logic correct
- [ ] Participation flag invariant validation
- [ ] Bid schedule validation
- [ ] API endpoint implemented
- [ ] API response types defined
- [ ] Unit tests for each readiness criterion
- [ ] Integration tests for readiness endpoint
- [ ] Tests for seniority conflict detection
- [ ] Tests for No Bid review tracking
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes

---

## Stop-and-Ask Conditions

Stop if:

- Seniority conflict detection logic is unclear or ambiguous
- Bid order computation requires new domain rules not in scope
- No Bid review tracking mechanism is uncertain
- Integration with existing bootstrap completeness is unclear
- System area exclusion logic conflicts with existing rules
- Readiness criteria conflict with existing domain invariants

---

## Risk Notes

- Seniority conflict detection may reveal issues with existing bid order logic
- No Bid review tracking adds state to users table (or audit log complexity)
- Readiness computation may be expensive for large datasets
- Existing bid years may fail readiness until fully configured
- Seniority tie-breaking logic may need clarification or refinement
