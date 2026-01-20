# Phase 29G — Post-Confirmation Bid Order Adjustments

## Purpose

Implement administrative bid order and bid window adjustment capabilities that operate **after confirmation**.

These adjustments support operational reality without corrupting domain truth:

- Seniority data is never changed
- No recomputation occurs
- No waterfall effects
- Changes apply to current/future rounds only
- All adjustments are auditable

---

## Scope

### 1. Adjustment Capabilities

Post-confirmation, administrators may:

- **Reorder users** — change bid order position
- **Adjust bid windows** — change window start/end datetimes

### 2. Constraints

#### Seniority Data Is Immutable

- Adjustments **never** modify seniority dates, lottery values, or other input data
- Adjustments override derived order only
- Original seniority-based order remains visible for audit

#### No Waterfall Effects

- Reordering one user does **not** automatically shift others
- Gaps in bid order are permitted
- Duplicate positions are permitted (though discouraged)

**Example:**

Original order: 1, 2, 3, 4, 5

Admin swaps users 2 and 4:

New order: 1, 4, 3, 2, 5

No other users are affected.

#### Scope of Changes

- Adjustments apply to:
  - **current round** (if user has not yet bid)
  - **all future rounds**
- Adjustments do **not** affect:
  - completed rounds
  - rounds where user has already bid

### 3. Bid Order Adjustment

#### Bid Order Database Impact

Adjustments update the `canonical_bid_order` table:

- `bid_order` field is updated
- `is_overridden = 1`
- `override_reason` is set

**Note:** This may overlap with existing `OverrideBidOrder` functionality from earlier phases. Verify consistency.

#### Bid Order Adjustment API Endpoint

- `POST /api/bid-years/{bid_year_id}/areas/{area_id}/adjust-bid-order`
  - Request:

    ```json
    {
      "adjustments": [
        { "user_id": 1, "new_bid_order": 5 },
        { "user_id": 2, "new_bid_order": 3 }
      ],
      "reason": "Operational adjustment per management directive"
    }
    ```

  - Validates:
    - bid year is in Canonicalized state
    - actor is Admin
    - reason is at least 10 characters
  - Updates `canonical_bid_order` records
  - Records audit event
  - Does **not** recalculate bid windows automatically (that's a separate action)

### 4. Bid Window Adjustment

#### Bid Window Database Impact

Adjustments update the `bid_windows` table:

- `window_start_datetime` field is updated
- `window_end_datetime` field is updated

**Note:** Consider adding `is_adjusted` flag and `adjustment_reason` fields to `bid_windows` table for audit clarity.

#### Bid Window Adjustment API Endpoint

- `POST /api/bid-years/{bid_year_id}/areas/{area_id}/adjust-bid-window`
  - Request:

    ```json
    {
      "user_id": 1,
      "round_id": 1,
      "new_window_start": "2026-03-05T13:00:00Z",
      "new_window_end": "2026-03-05T23:00:00Z",
      "reason": "Accommodating special request"
    }
    ```

  - Validates:
    - bid year is in Canonicalized state
    - actor is Admin
    - reason is at least 10 characters
    - new window start < new window end
    - user has not yet bid in this round
  - Updates `bid_windows` record
  - Records audit event

### 5. Bulk Bid Window Recalculation

After bid order adjustments, administrators may need to recalculate bid windows for affected users.

#### Bulk Recalculation API Endpoint

- `POST /api/bid-years/{bid_year_id}/areas/{area_id}/recalculate-bid-windows`
  - Request:

    ```json
    {
      "user_ids": [1, 2, 3],
      "rounds": [1, 2, 3],
      "reason": "Recalculate windows after bid order adjustment"
    }
    ```

  - Validates:
    - bid year is in Canonicalized state
    - actor is Admin
    - users have not yet bid in specified rounds
  - Recalculates windows using bid schedule parameters
  - Updates `bid_windows` records
  - Records audit event

### 6. Audit Events

All adjustments generate audit events:

#### Bid Order Adjustment

- `action = "AdjustBidOrder"`
- `actor = <admin operator>`
- `cause = <reason>`
- `state_before = { user_id, area_id, bid_order: 2 }`
- `state_after = { user_id, area_id, bid_order: 5 }`

#### Bid Window Adjustment

- `action = "AdjustBidWindow"`
- `actor = <admin operator>`
- `cause = <reason>`
- `state_before = { user_id, round_id, window_start, window_end }`
- `state_after = { user_id, round_id, new_window_start, new_window_end }`

### 7. Round Completion Constraint

Adjustments are prohibited for rounds where:

- The user has already completed bidding
- The round is marked as closed/finalized

Validation must check bid status (from Sub-Phase 29F) before allowing adjustments.

### 8. UI Considerations (Out of Scope)

This sub-phase implements API endpoints only. UI is not in scope for Phase 29.

However, the API must support:

- Viewing current bid order with adjustment indicators
- Viewing current bid windows with adjustment indicators
- Comparing original vs. adjusted values
- Audit trail of all adjustments

---

## Explicit Non-Goals

- No automatic recalculation of bid windows on order adjustment
- No waterfall/cascade effects
- No seniority data modification
- No bid execution logic
- No UI for adjustment interface (out of scope for Phase 29)
- No rollback of adjustments (use audit log for history)

---

## Completion Checklist

- [ ] Bid order adjustment endpoint implemented
- [ ] Bid window adjustment endpoint implemented
- [ ] Bulk bid window recalculation endpoint implemented
- [ ] Integration with existing `OverrideBidOrder` verified (no conflicts)
- [ ] Round completion constraint enforced
- [ ] Audit event recording implemented
- [ ] Validation for Canonicalized state
- [ ] Validation for Admin role
- [ ] Validation for reason length (min 10 chars)
- [ ] Unit tests for adjustment logic
- [ ] Tests for constraint enforcement (completed rounds)
- [ ] Integration tests for API endpoints
- [ ] Tests for audit event generation
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes

---

## Stop-and-Ask Conditions

Stop if:

- Integration with existing `OverrideBidOrder` functionality is unclear or conflicts
- Round completion detection logic is ambiguous
- Bid window recalculation semantics conflict with original calculation (Sub-Phase 29E)
- Audit event structure for adjustments is unclear
- Waterfall prevention semantics require clarification
- Adjustment scope (current/future rounds) is ambiguous

---

## Risk Notes

- Existing `OverrideBidOrder` functionality may overlap or conflict
- Bid order adjustments without window recalculation may cause confusion
- Manual window adjustments may violate fairness expectations
- Gaps and duplicates in bid order may cause downstream issues
- Adjustments may be difficult to visualize without UI
- Audit trail complexity increases with multiple adjustment types
