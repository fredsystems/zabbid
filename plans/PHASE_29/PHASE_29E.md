# Phase 29E — Confirmation and Bid Order Freezing

## Purpose

Implement the explicit, irreversible confirmation action that transitions a bid year from domain-ready to confirmed-ready-to-bid.

At confirmation:

- Bid order is **materialized** and becomes authoritative
- Bid windows are **calculated and stored**
- Editing locks engage
- Bidding lifecycle begins
- The action **cannot be undone**

---

## Scope

### 1. Lifecycle State Transition

Add new lifecycle state or use existing:

**Option A:** Add `ConfirmedReadyToBid` state

**Option B:** Use existing `Canonicalized` state to represent confirmation

**Recommended:** Use `Canonicalized` (already exists, semantically correct)

The transition:

```text
BootstrapComplete → Canonicalized (irreversible)
```

### 2. Confirmation Preconditions

Confirmation is permitted **only** when:

- Bid year is in `BootstrapComplete` state
- Readiness evaluation passes (all criteria from Sub-Phase 29D)
- Actor is an Admin

### 3. Bid Order Materialization

At confirmation time, the system must:

- Compute bid order for all users (excluding those with `excluded_from_bidding = true`)
- Store the order in `canonical_bid_order` table
- Assign sequential position numbers
- Record audit event

#### Canonical Bid Order Table

Already exists from earlier phases. Verify schema supports:

```sql
CREATE TABLE canonical_bid_order (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bid_year_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    bid_order INTEGER,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(user_id) REFERENCES users(user_id)
);
```

**At confirmation:**

- Derive bid order from seniority data
- Insert one row per non-excluded user
- Set `is_overridden = 0`
- Set `override_reason = NULL`
- Set `bid_order` to derived position

### 4. Bid Window Calculation

At confirmation time, the system must:

- Calculate individual bid windows based on:
  - bid order position
  - `bidders_per_area_per_day` from bid schedule
  - `bid_start_date` from bid schedule
  - `bid_window_start_time` and `bid_window_end_time` from bid schedule
- Store windows in `bid_windows` table (new)

#### Bid Windows Table

```sql
CREATE TABLE bid_windows (
    bid_window_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    area_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    window_start_datetime TEXT NOT NULL,
    window_end_datetime TEXT NOT NULL,
    UNIQUE (bid_year_id, area_id, user_id),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id),
    FOREIGN KEY(area_id) REFERENCES areas(area_id),
    FOREIGN KEY(user_id) REFERENCES users(user_id)
);
```

**Window Calculation Algorithm:**

1. Sort users by bid order within each area
2. For each user in order:
   - Determine which day they bid (based on `bidders_per_area_per_day`)
   - Calculate start datetime (day + `window_start_time` in declared timezone)
   - Calculate end datetime (day + `window_end_time` in declared timezone)
   - Store in `bid_windows` table

**Week Structure:**

- Bidding occurs Monday–Friday only
- Skip weekends
- Start date must be a Monday (validated in Sub-Phase 29C)

**Example:**

- `bidders_per_area_per_day = 5`
- `bid_start_date = 2026-03-02` (Monday)
- `window_start_time = 08:00:00`
- `window_end_time = 18:00:00`
- `timezone = America/New_York`

Users 1–5: Monday Mar 2, 08:00–18:00 ET
Users 6–10: Tuesday Mar 3, 08:00–18:00 ET
Users 11–15: Wednesday Mar 4, 08:00–18:00 ET
Users 16–20: Thursday Mar 5, 08:00–18:00 ET
Users 21–25: Friday Mar 6, 08:00–18:00 ET
Users 26–30: Monday Mar 9, 08:00–18:00 ET (skip weekend)

### 5. Confirmation API Endpoint

- `POST /api/bid-years/{bid_year_id}/confirm-ready-to-bid`
  - Request: `{ confirmation: "I understand this action is irreversible" }`
  - Validates preconditions:
    - bid year is in `BootstrapComplete` state
    - readiness evaluation passes
    - actor is Admin
  - Materializes bid order
  - Calculates bid windows
  - Transitions to `Canonicalized` state
  - Records audit event
  - Returns:

    ```json
    {
      "audit_event_id": 123,
      "message": "Bid year 2026 confirmed ready to bid",
      "bid_order_count": 45,
      "bid_windows_calculated": 45
    }
    ```

### 6. Editing Locks Post-Confirmation

After confirmation, the following operations are prohibited:

- Creating/deleting areas
- Creating/deleting users
- Editing user participation flags
- Editing round configuration
- Editing bid schedule
- Editing area metadata (already enforced from earlier phases)

The following operations remain permitted (with audit):

- Adjusting bid order (Sub-Phase 29G)
- Adjusting bid windows (Sub-Phase 29G)

### 7. Audit Event

Record confirmation event:

- `action = "ConfirmReadyToBid"`
- `actor = <admin operator>`
- `cause = "Manual confirmation by administrator"`
- `state_before = { lifecycle_state: "BootstrapComplete" }`
- `state_after = { lifecycle_state: "Canonicalized", bid_order_materialized: true, bid_windows_calculated: true }`

### 8. Timezone-Aware Datetime Storage

Bid windows are stored as **UTC timestamps** in TEXT format (ISO 8601).

**Storage:** `2026-03-02T13:00:00Z` (UTC)

**Display:** Convert to declared timezone for UI

**Comparison:** Use UTC for all temporal logic

This ensures:

- DST transitions are handled correctly
- Timestamps are unambiguous
- Comparison and sorting work correctly

---

## Explicit Non-Goals

- No bid execution logic
- No status tracking (that's Sub-Phase 29F)
- No post-confirmation adjustments (that's Sub-Phase 29G)
- No UI for confirmation (out of scope for Phase 29)
- No rollback or undo mechanism

---

## Completion Checklist

- [ ] Lifecycle state transition implemented (`BootstrapComplete → Canonicalized`)
- [ ] Confirmation preconditions enforced
- [ ] Bid order materialization implemented
- [ ] Bid windows table created (SQLite and MySQL)
- [ ] Bid window calculation algorithm implemented
- [ ] Timezone-aware datetime conversion implemented
- [ ] Week structure (Mon-Fri) enforced
- [ ] API endpoint implemented
- [ ] Editing locks enforced post-confirmation
- [ ] Audit event recorded
- [ ] Unit tests for bid order materialization
- [ ] Unit tests for bid window calculation
- [ ] Tests for week structure (weekend skipping)
- [ ] Tests for timezone handling
- [ ] Integration tests for confirmation endpoint
- [ ] Tests for precondition enforcement
- [ ] Tests for editing locks
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes

---

## Stop-and-Ask Conditions

Stop if:

- Bid order derivation logic is unclear or conflicts with existing implementation
- Bid window calculation semantics are ambiguous
- Timezone conversion library is unavailable or unsuitable
- Lifecycle state choice (new vs. existing) is uncertain
- Editing lock enforcement patterns conflict with existing architecture
- Audit event structure is unclear
- DST handling semantics require clarification

---

## Risk Notes

- Bid order materialization is irreversible
- Bid window calculation errors cannot be corrected without manual intervention
- Timezone conversion may introduce bugs if not tested thoroughly
- Existing bid years in `BootstrapComplete` state will fail confirmation until all prerequisites are met
- Confirmation may be expensive for large datasets (many users/areas)
- Datetime storage format must be consistent with existing patterns
