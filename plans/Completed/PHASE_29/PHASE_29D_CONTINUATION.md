# Phase 29D Continuation Instructions

## Current Status (2026-01-20)

**Phase 29D is ~95% complete.** Seniority conflict detection has been fully implemented via real bid order computation, but work remains to complete the phase.

---

## What's Complete ✅

### 1. Real Seniority Conflict Detection

The stubbed conflict detection has been replaced with full bid order computation:

- **Domain Layer** (`crates/domain/src/bid_order.rs`):
  - `compute_bid_order()` function with strict 5-tier seniority ordering
  - `BidOrderPosition` and `SeniorityInputs` types
  - `SeniorityConflict` domain error variant
  - Comprehensive tests for all tie-breaking scenarios

- **Persistence Layer**:
  - `get_users_by_area_for_conflict_detection()` query
  - Groups users by non-system areas
  - Full SQLite/MySQL backend support

- **API Layer**:
  - `get_bid_year_readiness()` uses real conflict detection
  - Per-area bid order computation during readiness evaluation
  - Detailed conflict reporting with user initials and area

### 2. All Other Readiness Criteria

- ✅ Database schema (no_bid_reviewed flag)
- ✅ Domain types and logic
- ✅ Persistence queries (all readiness checks)
- ✅ API handlers and response types
- ✅ Build succeeds, all tests pass

---

## What Remains ⚠️

### 1. Fix Clippy Error (BLOCKING)

**Error:** `get_bid_year_readiness()` function exceeds 100 lines (clippy::too_many_lines)

**Resolution Required:**

Extract helper functions from `get_bid_year_readiness()`:

```rust
// Suggested refactoring:

fn detect_seniority_conflicts(
    persistence: &mut SqlitePersistence,
    bid_year_id: i64,
) -> Result<(usize, Vec<String>), ApiError> {
    // Move conflict detection logic here
}

fn build_blocking_reasons(
    areas_missing_rounds: &[String],
    no_bid_users_pending_review: usize,
    participation_flag_violations: usize,
    seniority_conflicts: usize,
    conflict_details: &[String],
    bid_schedule_set: bool,
) -> Vec<String> {
    // Move blocking reason assembly here
}

pub fn get_bid_year_readiness(...) -> Result<...> {
    // Call helpers, keep this function < 100 lines
}
```

**Location:** `crates/api/src/handlers.rs` around line 6088

### 2. Add Derived Bid Order Preview API (REQUIRED)

**Requirement:** Operators must be able to view computed bid order **before** irreversible confirmation.

**Endpoint to Add:**

```http
GET /api/bid-years/{bid_year_id}/areas/{area_id}/bid-order-preview
```

**Response:**

```json
{
  "bid_year_id": 1,
  "area_id": 2,
  "area_code": "North",
  "positions": [
    {
      "position": 1,
      "user_id": 42,
      "initials": "AB",
      "seniority_inputs": {
        "cumulative_natca_bu_date": "2018-01-15",
        "natca_bu_date": "2018-01-15",
        "eod_faa_date": "2018-01-15",
        "service_computation_date": "2018-01-15",
        "lottery_value": 3
      }
    },
    ...
  ]
}
```

**Implementation:**

1. Add `GetBidOrderPreviewResponse` type to `crates/api/src/request_response.rs`
2. Add `get_bid_order_preview()` handler to `crates/api/src/handlers.rs`
3. Call `compute_bid_order()` on users in the specified area
4. Return ordered list of `BidOrderPosition` mapped to response type
5. **No persistence, no audit events** (read-only preview)
6. **Pre-confirmation only** (lifecycle constraint)
7. Mark `#[allow(dead_code)]` until wired up in server

### 3. Update Planning Documents (MANDATORY)

Per authorization, the following documents must be updated to reflect the structural correction:

#### A. `plans/PHASE_29.md`

Add to section **F. Bid Order Determination and Freezing**:

```markdown
#### Pre-Confirmation Review

- Derived bid order is computed and **visible via preview API**
- Operators may review ordering before confirmation
- No irreversible action occurs without operator visibility
- Readiness evaluation **includes real bid order computation**
```

#### B. `plans/PHASE_29/PHASE_29D.md`

Update **Seniority Conflict Detection** section:

```markdown
### 5. Seniority Conflict Detection

**Implementation:** Real bid order computation via `compute_bid_order()`

This function:

- Applies strict 5-tier seniority ordering
- Returns `SeniorityConflict` error on unresolved ties
- Is used by readiness evaluation and bid order preview

**No manual resolution path exists.** Conflicts are blocking errors.
```

Add new section **6. Derived Bid Order Preview API**:

```markdown
### 6. Derived Bid Order Preview API (NEW)

**Purpose:** Allow operators to view computed bid order before confirmation.

**Endpoint:** `GET /api/bid-years/{bid_year_id}/areas/{area_id}/bid-order-preview`

**Constraints:**

- Pre-confirmation only
- Read-only (no persistence, no audit events)
- Uses same computation logic that will be frozen at confirmation
- Lifecycle constraint: Draft or BootstrapComplete only

**Response:** Ordered list of users with seniority inputs (for transparency)
```

#### C. `plans/PHASE_29/PHASE_29E.md`

Update **3. Bid Order Materialization** section:

Add:

```markdown
**Critical:** Phase 29E must use the **exact same** `compute_bid_order()` function
that was used for readiness evaluation and preview.

DO NOT:

- Duplicate bid order logic
- Recompute ordering independently
- Introduce new tie-breaking rules

The frozen bid order must match what operators reviewed in the preview.
```

### 4. Integration Tests (Recommended)

Add tests for:

- Readiness endpoint with seniority conflicts
- Readiness endpoint with no conflicts
- Review-no-bid endpoint (mark user as reviewed)
- Bid order preview endpoint (if added)

### 5. Final Validation

Before completion:

- [ ] Run `cargo xtask ci` (must pass)
- [ ] Run `pre-commit run --all-files` (must pass)
- [ ] All new files added via `git add`
- [ ] Update `PHASE_29_WORKING_STATE.md` to mark 29D complete
- [ ] Commit with clear message

---

## Key Files Modified

- `crates/domain/src/bid_order.rs` (NEW)
- `crates/domain/src/error.rs`
- `crates/domain/src/lib.rs`
- `crates/domain/src/readiness.rs`
- `crates/persistence/src/lib.rs`
- `crates/persistence/src/queries/mod.rs`
- `crates/persistence/src/queries/readiness.rs`
- `crates/api/src/error.rs`
- `crates/api/src/handlers.rs`

---

## Critical Invariants (DO NOT VIOLATE)

1. **Single source of truth:** `compute_bid_order()` is the ONLY bid order computation
2. **No persistence in 29D:** Preview API is read-only
3. **No audit events before confirmation:** Preview does not write to audit log
4. **Strict total ordering:** Unresolved ties are domain violations
5. **Phase 29E alignment:** Confirmation freezing must use the same computation logic

---

## Estimated Work Remaining

- Fix clippy error: **15 minutes**
- Add bid order preview API: **30-45 minutes**
- Update planning documents: **15-20 minutes**
- Integration tests: **30 minutes**
- Final validation: **10 minutes**

**Total:** ~2 hours

---

## Contact Points

If stuck:

1. Review `AGENTS.md` for constraints
2. Review `PHASE_EXECUTION.md` for protocol
3. Stop and ask if:
   - Bid order computation semantics are unclear
   - Phase boundaries become ambiguous
   - Freezing logic conflicts with preview logic

---

## Success Criteria

Phase 29D is complete when:

- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes
- [ ] Seniority conflicts detected via real computation
- [ ] Derived bid order preview API exists and works
- [ ] Planning documents updated
- [ ] No duplicate bid order logic exists
- [ ] Tests cover all readiness criteria

---

## Last Known State

- Branch: `phase29`
- Last commit: "Phase 29D: Implement real seniority conflict detection via bid order computation"
- Uncommitted changes: Clippy fixes in progress
- Next agent: Fix clippy, add preview API, update docs, complete phase
