# Phase 25D — Override Semantics

## Objective

Implement explicit override actions that allow administrators to edit canonical data after canonicalization while maintaining a complete audit trail. Overrides record the original value, new value, reason, and actor, ensuring all changes are transparent and reversible.

---

## In-Scope

- Implement override actions for canonical data
- Track override state in canonical tables (already have `is_overridden` flag)
- Emit detailed audit events for each override
- Enforce that overrides require canonicalization (lifecycle state ≥ Canonicalized)
- Require non-empty reason for all overrides
- Update canonical records to reflect override state
- Validate override business rules (e.g., cannot override to invalid state)
- Comprehensive tests for override behavior and audit trail

---

## Out-of-Scope

- Bidding execution logic
- Round management
- Rollback functionality (future phase)
- Bulk override operations (may be added later)
- Override approval workflow (all overrides are immediate)
- Override history tracking beyond audit log (single override flag per record)

---

## Override Philosophy

### Core Principles

1. **Explicit, Not Implicit**: Overrides are intentional administrative actions, not side effects
2. **Auditable**: Every override records who, what, when, why
3. **Non-Recomputing**: Overrides do not trigger recomputation of dependent data
4. **Authoritative**: Overridden values become the source of truth
5. **Reason Required**: All overrides must justify why the canonical value was insufficient

### Overrides vs Edits

**Before Canonicalization (Edits):**

- Direct mutations to user/area data
- No special tracking
- Normal audit events
- Part of bootstrap process

**After Canonicalization (Overrides):**

- Explicit override actions
- Original value captured in audit event
- Override flag set on canonical record
- Override reason required and stored

---

## Invariants Enforced

1. **Overrides Require Canonicalization**
   - Cannot override data if lifecycle state < Canonicalized
   - Attempting override before canonicalization fails with clear error

2. **Override Reason Required**
   - All override actions must include a non-empty reason string
   - Minimum length: 10 characters (prevents placeholder text)

3. **Overrides Are Authoritative**
   - Overridden values are the source of truth
   - System must not recompute or "fix" overridden values
   - Future phases must respect override flag

4. **One Override State Per Record**
   - Each canonical record has a single `is_overridden` flag
   - Subsequent overrides replace previous override
   - Audit log preserves full history

5. **Override Validity**
   - Cannot override to invalid state (e.g., assign to non-existent area)
   - Cannot override eligibility to NULL
   - Business rules still apply to override values

---

## Domain Actions Introduced

### 1. `OverrideAreaAssignment`

Reassign a user to a different area after canonicalization.

**Preconditions:**

- Lifecycle state ≥ Canonicalized
- User exists and has canonical area membership record
- Target area exists and is not a system area (No Bid)
- Reason provided (≥ 10 characters)

**Effects:**

1. Query current canonical area assignment
2. Update `canonical_area_membership`:
   - Set `area_id = new_area_id`
   - Set `is_overridden = true`
   - Set `override_reason = reason`
3. Emit `UserAreaAssignmentOverridden` audit event

**Errors:**

- `CannotOverrideBeforeCanonicalization` if state < Canonicalized
- `InvalidOverrideReason` if reason empty or < 10 characters
- `AreaNotFound` if target area does not exist
- `CannotAssignToSystemArea` if target area is No Bid
- `UserNotFound` if user does not exist
- `CanonicalRecordNotFound` if user has no canonical area membership

---

### 2. `OverrideEligibility`

Change whether a user is eligible to bid.

**Preconditions:**

- Lifecycle state ≥ Canonicalized
- User exists and has canonical eligibility record
- Reason provided (≥ 10 characters)

**Effects:**

1. Query current canonical eligibility
2. Update `canonical_eligibility`:
   - Set `can_bid = new_eligibility_status`
   - Set `is_overridden = true`
   - Set `override_reason = reason`
3. Emit `UserEligibilityOverridden` audit event

**Errors:**

- `CannotOverrideBeforeCanonicalization` if state < Canonicalized
- `InvalidOverrideReason` if reason empty or < 10 characters
- `UserNotFound` if user does not exist
- `CanonicalRecordNotFound` if user has no canonical eligibility record

---

### 3. `OverrideBidOrder`

Manually set or change a user's bid order position.

**Preconditions:**

- Lifecycle state ≥ Canonicalized
- User exists and has canonical bid order record
- New bid order is positive integer or NULL
- Reason provided (≥ 10 characters)

**Effects:**

1. Query current canonical bid order
2. Update `canonical_bid_order`:
   - Set `bid_order = new_order`
   - Set `is_overridden = true`
   - Set `override_reason = reason`
3. Emit `UserBidOrderOverridden` audit event

**Errors:**

- `CannotOverrideBeforeCanonicalization` if state < Canonicalized
- `InvalidOverrideReason` if reason empty or < 10 characters
- `InvalidBidOrder` if order ≤ 0
- `UserNotFound` if user does not exist
- `CanonicalRecordNotFound` if user has no canonical bid order record

**Note:**

- Setting bid order to NULL is allowed (clears override)
- No uniqueness constraint on bid order (ties are permitted)
- Future seniority logic must respect override flag

---

### 4. `OverrideBidWindow`

Manually set or change a user's bidding window dates.

**Preconditions:**

- Lifecycle state ≥ Canonicalized
- User exists and has canonical bid windows record
- Window dates are valid (start ≤ end) or both NULL
- Reason provided (≥ 10 characters)

**Effects:**

1. Query current canonical bid windows
2. Update `canonical_bid_windows`:
   - Set `window_start_date = new_start`
   - Set `window_end_date = new_end`
   - Set `is_overridden = true`
   - Set `override_reason = reason`
3. Emit `UserBidWindowOverridden` audit event

**Errors:**

- `CannotOverrideBeforeCanonicalization` if state < Canonicalized
- `InvalidOverrideReason` if reason empty or < 10 characters
- `InvalidBidWindow` if start > end (when both non-NULL)
- `UserNotFound` if user does not exist
- `CanonicalRecordNotFound` if user has no canonical bid window record

**Note:**

- Both dates may be NULL (clears override)
- Partial windows (only start or only end) are invalid
- Future bid window logic must respect override flag

---

## Audit Events

### `UserAreaAssignmentOverridden`

**Payload:**

```rust
{
    bid_year_id: i64,
    user_id: i64,
    user_initials: String,
    previous_area_id: i64,
    previous_area_name: String,
    new_area_id: i64,
    new_area_name: String,
    reason: String,
    was_already_overridden: bool,
    actor: String,
    timestamp: DateTime,
}
```

---

### `UserEligibilityOverridden`

**Payload:**

```rust
{
    bid_year_id: i64,
    user_id: i64,
    user_initials: String,
    previous_eligibility: bool,
    new_eligibility: bool,
    reason: String,
    was_already_overridden: bool,
    actor: String,
    timestamp: DateTime,
}
```

---

### `UserBidOrderOverridden`

**Payload:**

```rust
{
    bid_year_id: i64,
    user_id: i64,
    user_initials: String,
    previous_bid_order: Option<i32>,
    new_bid_order: Option<i32>,
    reason: String,
    was_already_overridden: bool,
    actor: String,
    timestamp: DateTime,
}
```

---

### `UserBidWindowOverridden`

**Payload:**

```rust
{
    bid_year_id: i64,
    user_id: i64,
    user_initials: String,
    previous_window_start: Option<String>,
    previous_window_end: Option<String>,
    new_window_start: Option<String>,
    new_window_end: Option<String>,
    reason: String,
    was_already_overridden: bool,
    actor: String,
    timestamp: DateTime,
}
```

---

## Override Reason Validation

### Minimum Length

- Reason must be ≥ 10 characters
- Prevents lazy justifications like "fix" or "correct"

### Suggested Reason Patterns

**Area Assignment Override:**

- "User requested transfer to [area] due to personal circumstances"
- "Administrative correction: user was incorrectly assigned during bootstrap"
- "Accommodation for medical/family need"

**Eligibility Override:**

- "User on extended leave, ineligible for this bid year"
- "New hire arrived after canonicalization, now eligible"
- "User retiring mid-year, marked ineligible"

**Bid Order Override:**

- "Seniority calculation error corrected per union agreement"
- "Lottery tie-breaker manually applied"
- "Accommodation per Article [X] of CBA"

**Bid Window Override:**

- "User on leave during standard window, extended to [dates]"
- "Accommodation for personal emergency"
- "Make-up window for system downtime during original window"

---

## Schema Impact

No new tables required. Canonical tables already have:

- `is_overridden` flag (boolean)
- `override_reason` field (text, nullable)

These fields are populated by override actions.

---

## API Implications

### Override Endpoints (Admin Only)

New endpoints required:

- `POST /api/bid_years/{year}/users/{initials}/override_area`
- `POST /api/bid_years/{year}/users/{initials}/override_eligibility`
- `POST /api/bid_years/{year}/users/{initials}/override_bid_order`
- `POST /api/bid_years/{year}/users/{initials}/override_bid_window`

### Request Schemas

**Override Area Assignment:**

```json
{
  "new_area_id": 123,
  "reason": "User requested transfer due to personal circumstances"
}
```

**Override Eligibility:**

```json
{
  "can_bid": false,
  "reason": "User on extended leave, ineligible for this bid year"
}
```

**Override Bid Order:**

```json
{
  "bid_order": 42,
  "reason": "Seniority calculation corrected per union agreement"
}
```

**Override Bid Window:**

```json
{
  "window_start": "2025-01-15",
  "window_end": "2025-01-20",
  "reason": "Extended window due to leave during standard window"
}
```

### Response

All override endpoints return:

```json
{
  "success": true,
  "audit_event_id": 1234
}
```

---

## Authorization

All override actions require **Admin** role.

**Bidder** role cannot perform overrides (bidders enter bids, admins manage structure).

---

## UI Implications

### Admin User Detail Page

**Display Override Status:**

- Show badge/indicator if any canonical data is overridden
- Display override reason for each overridden field
- Link to audit event that created override

**Override Actions:**

- "Override Area Assignment" button/form
- "Override Eligibility" toggle with reason field
- "Override Bid Order" input with reason field
- "Override Bid Window" date pickers with reason field

**Override Form Validation:**

- Reason field required, min 10 characters
- Target values validated (e.g., area exists)
- Confirmation prompt before submitting

### Audit Log Display

**Override Events Highlighted:**

- Override events shown distinctly in audit log
- Display previous and new values side-by-side
- Show reason prominently
- Link to affected user

---

## Testing Requirements

### Override Action Tests

- Override actions succeed when lifecycle state ≥ Canonicalized
- Override actions fail when lifecycle state < Canonicalized
- Override actions require valid reason (≥ 10 chars)
- Override actions validate new values (e.g., area exists)

### Audit Event Tests

- Each override emits correct audit event
- Audit events capture previous and new values
- Audit events include reason
- Audit events record `was_already_overridden` flag

### Idempotency Tests

- Overriding same field twice updates canonical record
- Second override replaces first (not additive)
- Audit log contains both events

### Validation Tests

- Cannot override to invalid state (non-existent area, NULL eligibility)
- Cannot override with empty reason
- Cannot override with reason < 10 characters
- Bid window validation: start ≤ end

### Authorization Tests

- Admin role can perform overrides
- Bidder role cannot perform overrides
- Unauthenticated requests fail

### Read Query Tests

- Queries return overridden values, not original canonical values
- Override flag is visible in query results (for admin views)
- Override reason is accessible (for audit purposes)

---

## Exit Criteria

Phase 25D is complete when:

1. ✅ All four override actions implemented and tested
2. ✅ Override actions enforce lifecycle state requirement
3. ✅ Override reason validation enforced (≥ 10 chars)
4. ✅ Audit events emitted for all overrides
5. ✅ Canonical records updated with override flag and reason
6. ✅ Override authorization enforced (Admin only)
7. ✅ Queries return overridden values correctly
8. ✅ All tests pass (`cargo xtask ci` and `pre-commit run --all-files`)
9. ✅ API endpoints for overrides implemented (if applicable)
10. ✅ No breaking changes to existing APIs
11. ✅ Override business rules validated (e.g., cannot assign to No Bid area)

---

## Notes

- Overrides are a controlled escape hatch for administrative corrections
- Override reason ensures accountability and auditability
- Override flag prevents silent recomputation from overwriting manual corrections
- Future phases (seniority, bid windows) must check override flag before computing
- Rollback (future phase) may restore pre-override canonical values
- Overrides are immediate; no approval workflow (trust model: admins are authorized)
- Bulk override operations may be added in future if needed for efficiency
