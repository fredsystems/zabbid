# Phase 29 Gap Analysis

**Analysis Date:** 2025-01-27
**Analyzed By:** AI Agent (Phase 30A execution)
**Phase 29 Scope:** Pre-Bid Requirements & Readiness Gate
**Phase 30 Scope:** UI Enablement and End-to-End Validation

---

## Executive Summary

Phase 29 delivered the majority of backend APIs required for pre-bid configuration and readiness validation. However, **one critical capability is missing**: the ability to assign a round group to an area. This is a **blocking gap** that must be resolved before Phase 30C (Area → Round Group Assignment UI) can be implemented.

Additionally, **all Phase 29 APIs lack frontend bindings** in `ui/src/api.ts`, meaning the UI cannot currently interact with any Phase 29 features.

---

## 1. API Inventory

### 1.1 Phase 29A — User Participation Flags

| Endpoint               | Method | Handler                            | Frontend Binding | Status              |
| ---------------------- | ------ | ---------------------------------- | ---------------- | ------------------- |
| `/users/participation` | POST   | `handle_update_user_participation` | ❌ Missing       | ✅ Backend Complete |

**Purpose:** Update `excluded_from_bidding` and `excluded_from_leave_calculation` flags per user.

---

### 1.2 Phase 29B — Round Groups & Rounds

| Endpoint             | Method | Handler                     | Frontend Binding | Status              |
| -------------------- | ------ | --------------------------- | ---------------- | ------------------- |
| `/round-groups`      | POST   | `handle_create_round_group` | ❌ Missing       | ✅ Backend Complete |
| `/round-groups`      | GET    | `handle_list_round_groups`  | ❌ Missing       | ✅ Backend Complete |
| `/round-groups/{id}` | POST   | `handle_update_round_group` | ❌ Missing       | ✅ Backend Complete |
| `/round-groups/{id}` | DELETE | `handle_delete_round_group` | ❌ Missing       | ✅ Backend Complete |
| `/rounds`            | POST   | `handle_create_round`       | ❌ Missing       | ✅ Backend Complete |
| `/rounds`            | GET    | `handle_list_rounds`        | ❌ Missing       | ✅ Backend Complete |
| `/rounds/{id}`       | POST   | `handle_update_round`       | ❌ Missing       | ✅ Backend Complete |
| `/rounds/{id}`       | DELETE | `handle_delete_round`       | ❌ Missing       | ✅ Backend Complete |

**Purpose:** Full CRUD for round groups and rounds within a bid year.

---

### 1.3 Phase 29C — Bid Schedule

| Endpoint                      | Method | Handler                   | Frontend Binding | Status              |
| ----------------------------- | ------ | ------------------------- | ---------------- | ------------------- |
| `/bid-schedule`               | POST   | `handle_set_bid_schedule` | ❌ Missing       | ✅ Backend Complete |
| `/bid-schedule/{bid_year_id}` | GET    | `handle_get_bid_schedule` | ❌ Missing       | ✅ Backend Complete |

**Purpose:** Declare and retrieve bid schedule (timezone, start date, daily window, bidders/day).

---

### 1.4 Phase 29D — Readiness Evaluation

| Endpoint                         | Method | Handler                         | Frontend Binding | Status              |
| -------------------------------- | ------ | ------------------------------- | ---------------- | ------------------- |
| `/readiness/{bid_year_id}`       | GET    | `handle_get_bid_year_readiness` | ❌ Missing       | ✅ Backend Complete |
| `/users/{user_id}/review-no-bid` | POST   | `handle_review_no_bid_user`     | ❌ Missing       | ✅ Backend Complete |
| `/bid-order/preview`             | GET    | `handle_get_bid_order_preview`  | ❌ Missing       | ✅ Backend Complete |

**Purpose:** Check readiness status with blockers, review No Bid users, preview bid order per area.

---

### 1.5 Phase 29E — Confirmation (Irreversible)

| Endpoint                | Method | Handler                       | Frontend Binding | Status              |
| ----------------------- | ------ | ----------------------------- | ---------------- | ------------------- |
| `/confirm-ready-to-bid` | POST   | `handle_confirm_ready_to_bid` | ❌ Missing       | ✅ Backend Complete |

**Purpose:** Irreversibly transition to "Ready to Bid" state, freezing bid order and windows.

---

### 1.6 Phase 29G — Post-Confirmation Adjustments

| Endpoint                   | Method | Handler                          | Frontend Binding | Status              |
| -------------------------- | ------ | -------------------------------- | ---------------- | ------------------- |
| `/bid-order/adjust`        | POST   | `handle_adjust_bid_order`        | ❌ Missing       | ✅ Backend Complete |
| `/bid-windows/adjust`      | POST   | `handle_adjust_bid_window`       | ❌ Missing       | ✅ Backend Complete |
| `/bid-windows/recalculate` | POST   | `handle_recalculate_bid_windows` | ❌ Missing       | ✅ Backend Complete |

**Purpose:** Administrative adjustments to bid order and windows after confirmation (explicit, non-cascading).

---

### 1.7 Override Endpoints

| Endpoint                      | Method | Handler                       | Frontend Binding | Status              |
| ----------------------------- | ------ | ----------------------------- | ---------------- | ------------------- |
| `/users/override-eligibility` | POST   | `handle_override_eligibility` | ❌ Missing       | ✅ Backend Complete |
| `/users/override-bid-order`   | POST   | `handle_override_bid_order`   | ❌ Missing       | ✅ Backend Complete |
| `/users/override-bid-window`  | POST   | `handle_override_bid_window`  | ❌ Missing       | ✅ Backend Complete |

**Purpose:** Override computed eligibility, bid order, or bid windows with audit trail.

---

### 1.8 Existing APIs (Pre-Phase 29)

The following APIs already have frontend bindings and are not part of Phase 29 scope:

- Bid year management (create, list, set active, update metadata)
- Area management (create, list, update name, set expected counts)
- User management (register, list, update, CSV import/preview)
- Operator management (create, disable, enable, delete)
- Authentication (login, logout, whoami, bootstrap)
- Bootstrap status and completeness
- Lifecycle transitions (bootstrap complete, canonicalized, bidding active/closed)
- Override area assignment (post-canonicalization)

---

## 2. Capability Coverage Matrix

| Capability                               | Backend API         | Frontend Binding | Status          |
| ---------------------------------------- | ------------------- | ---------------- | --------------- |
| **Round Groups**                         |                     |                  |                 |
| → Create round group                     | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → List round groups                      | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Update round group                     | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Delete round group                     | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| **Rounds**                               |                     |                  |                 |
| → Create round                           | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → List rounds (for round group)          | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Update round                           | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Delete round                           | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| **Area → Round Group Assignment**        |                     |                  |                 |
| → Assign round group to area             | ❌ **MISSING**      | ❌ Missing       | ❌ **BLOCKING** |
| → Get area's round group                 | ⚠️ Via list areas   | ❌ Missing       | ⚠️ Partial      |
| → Validate one-per-non-system-area       | ❌ Unknown          | ❌ Missing       | ❌ **BLOCKING** |
| **Bid Schedule**                         |                     |                  |                 |
| → Set bid schedule                       | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Get bid schedule                       | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| **User Participation Flags**             |                     |                  |                 |
| → Update excluded_from_bidding           | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Update excluded_from_leave_calculation | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Enforce invariant (leave ⇒ bidding)    | ✅ Backend enforced | N/A              | ✅ Complete     |
| **Readiness & Confirmation**             |                     |                  |                 |
| → Get readiness status + blockers        | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Confirm ready to bid                   | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| **No Bid Review**                        |                     |                  |                 |
| → List users in No Bid                   | ⚠️ Via list users   | ✅ Exists        | ⚠️ Partial      |
| → Review/confirm user disposition        | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Reassign to competitive area           | ✅ Via update user  | ✅ Exists        | ✅ Complete     |
| **Bid Order Preview**                    |                     |                  |                 |
| → Get bid order (pre-confirm: derived)   | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Get bid order (post-confirm: frozen)   | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Display tie-breaker inputs             | ✅ In response      | ❌ Missing       | ⚠️ Partial      |
| **Bid Windows**                          |                     |                  |                 |
| → Get bid windows (derived or frozen)    | ⚠️ Unclear          | ❌ Missing       | ⚠️ Unclear      |
| **Post-Confirmation Adjustments**        |                     |                  |                 |
| → Adjust bid order                       | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Adjust bid window                      | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Recalculate bid windows                | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| **Overrides**                            |                     |                  |                 |
| → Override eligibility                   | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Override bid order                     | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |
| → Override bid window                    | ✅ Complete         | ❌ Missing       | ⚠️ Partial      |

---

## 3. Identified Gaps

### 3.1 Critical Blocking Gaps

#### **Gap 1: Area → Round Group Assignment API**

**Severity:** 🔴 **BLOCKING**

**Description:**

While the `areas` table in the database schema includes a `round_group_id` column, **there is no API endpoint to assign a round group to an area**.

- The `POST /areas/update` endpoint only allows updating the area's display name.
- There is no `POST /areas/{area_id}/round-group` or similar endpoint.
- There is no persistence mutation function to update `round_group_id`.

**Impact:**

- Phase 30C (Area → Round Group Assignment UI) **cannot be implemented** without this API.
- Pre-bid configuration workflow is incomplete.
- Readiness validation may not enforce the "exactly one round group per non-system area" constraint.

**Recommended Action:**

**STOP and ask user** before proceeding with Phase 30 execution.

Options:

1. **Implement missing API now** (Phase 29 gap-fill):
   - Add `POST /areas/{area_id}/assign-round-group` endpoint
   - Add persistence mutation: `update_area_round_group(area_id, round_group_id)`
   - Enforce lifecycle constraints (immutable after confirmation)
   - Enforce validation: non-system areas only, round group exists in same bid year
   - Add audit event for assignment

2. **Defer round group assignment** (Phase 30 scope reduction):
   - Remove Sub-Phase 30C from execution plan
   - Document limitation in Phase 30 deliverables
   - Mark as future work

3. **Workaround using raw SQL** (NOT RECOMMENDED):
   - Would bypass audit trail and domain validation
   - Violates AGENTS.md constraints

**User Decision Required:** ✋ **STOP HERE**

---

### 3.2 Non-Blocking Gaps

#### **Gap 2: Frontend API Bindings for All Phase 29 Features**

**Severity:** 🟡 **NON-BLOCKING** (expected work for Phase 30)

**Description:**

None of the Phase 29 backend APIs have corresponding frontend bindings in `ui/src/api.ts`.

**Impact:**

- All Phase 30 UI work will require adding frontend bindings first.
- This is expected and part of Phase 30 scope.

**Recommended Action:**

Add frontend bindings incrementally as each sub-phase requires them.

---

#### **Gap 3: Bid Window Retrieval API Unclear**

**Severity:** 🟡 **NON-BLOCKING** (needs clarification)

**Description:**

It's unclear whether there is a dedicated "get bid windows for users" endpoint, or if bid windows are returned as part of bid order preview or bid status queries.

The bid order preview endpoint exists (`GET /bid-order/preview`), but its response structure hasn't been verified to include bid windows.

**Impact:**

- May need additional API work if bid windows are not currently retrievable.
- May require composite UI logic to display bid windows.

**Recommended Action:**

- During Phase 30 execution, verify bid order preview response structure.
- If bid windows are missing, document as a gap and ask user.

---

## 4. Assumptions & Notes

### 4.1 Assumptions

1. **Round group assignment to areas was planned for Phase 29** but not implemented.
   - Evidence: `areas.round_group_id` column exists in schema.
   - Evidence: Phase 30 scope explicitly requires "Area → Round Group Assignment UI".

2. **Readiness validation may already check for round group assignment**, but enforcement at the API layer is missing.
   - This should be verified during Phase 30D (readiness review UI).

3. **Frontend bindings are expected work for Phase 30**, not a Phase 29 gap.

4. **Bid windows may be embedded in bid order preview response**, not a separate endpoint.

### 4.2 Composite vs. Atomic Operations

Several Phase 30 UI workflows will require **multiple backend API calls** rather than single composite endpoints:

- **No Bid Review Workflow:**
  - `GET /users?area_id={no_bid_area_id}` → list users
  - `POST /users/{user_id}/review-no-bid` → confirm disposition
  - `POST /users/update` → reassign user (if needed)

- **Bid Order View:**
  - `GET /bid-order/preview?bid_year_id={id}&area_id={id}` → get order

- **Readiness Review:**
  - `GET /readiness/{bid_year_id}` → get status + blockers
  - Various mutation APIs to resolve blockers

This is acceptable and aligns with Phase 30 scope ("orchestration in UI is permitted").

---

## 5. Schema Observations

### 5.1 Areas Table

```sql
areas (area_id) {
    area_id -> BigInt,
    bid_year_id -> BigInt,
    area_code -> Text,
    area_name -> Nullable<Text>,
    expected_user_count -> Nullable<Integer>,
    is_system_area -> Integer,
    round_group_id -> Nullable<BigInt>,  ← PRESENT but NO API TO SET
}
```

**Observation:** The schema supports round group assignment, but the API layer does not.

### 5.2 Bid Years Table

```sql
bid_years (bid_year_id) {
    ...
    bid_timezone -> Nullable<Text>,
    bid_start_date -> Nullable<Text>,
    bid_window_start_time -> Nullable<Text>,
    bid_window_end_time -> Nullable<Text>,
    bidders_per_area_per_day -> Nullable<Integer>,
}
```

**Observation:** Bid schedule fields are present and have corresponding APIs. ✅

---

## 6. Recommendations

### 6.1 Immediate Actions (Before Phase 30B execution)

1. **User decision required:** How to handle missing area → round group assignment API.
   - Option A: Implement now (Phase 29 gap-fill)
   - Option B: Defer and reduce Phase 30 scope
   - Option C: Stop Phase 30 until Phase 29 is complete

2. **If proceeding with Option A (gap-fill):**
   - Implement `POST /areas/{area_id}/assign-round-group` endpoint
   - Add request type: `{ round_group_id: i64 | null }`
   - Add response type: confirmation message + audit event ID
   - Enforce lifecycle: mutable pre-confirmation, immutable after
   - Enforce validation: non-system areas only, round group exists, same bid year
   - Add persistence mutation and audit event
   - Add tests
   - Add frontend binding

### 6.2 Phase 30 Execution Strategy (if gap is filled)

1. **Sub-Phase 30A:** ✅ Complete (this document)
2. **Sub-Phase 30B:** Round Groups & Rounds UI
   - Add frontend bindings for round group CRUD
   - Add frontend bindings for round CRUD
   - Implement UI components
3. **Sub-Phase 30C:** Area → Round Group Assignment UI
   - Add frontend binding for assign-round-group API (if implemented)
   - Implement UI to assign round groups to areas
4. **Sub-Phase 30D:** Bootstrap UI Restructure
   - Refactor monolithic BootstrapCompleteness component
   - Create structured, multi-section workflow
5. **Sub-Phase 30E–30I:** Continue as planned

### 6.3 Phase 30 Execution Strategy (if gap is deferred)

1. **Skip Sub-Phase 30C entirely**
2. **Document limitation:** Round group assignment not operable via UI
3. **Mark as future work**
4. **Proceed with remaining sub-phases**

---

## 7. Phase 29 Feature Coverage (Summary)

| Phase 29 Sub-Phase                 | Backend Complete | Frontend Bindings | UI Operable    |
| ---------------------------------- | ---------------- | ----------------- | -------------- |
| 29A: User Participation Flags      | ✅ Yes           | ❌ No             | ❌ No          |
| 29B: Round Groups & Rounds         | ✅ Yes           | ❌ No             | ❌ No          |
| 29C: Bid Schedule                  | ✅ Yes           | ❌ No             | ❌ No          |
| 29D: Readiness Evaluation          | ✅ Yes           | ❌ No             | ❌ No          |
| 29E: Confirmation                  | ✅ Yes           | ❌ No             | ❌ No          |
| 29G: Post-Confirmation Adjustments | ✅ Yes           | ❌ No             | ❌ No          |
| **Area → Round Group Assignment**  | ❌ **NO**        | ❌ No             | ❌ **BLOCKED** |

---

## 8. Conclusion

Phase 29 delivered a comprehensive backend API surface for pre-bid configuration and readiness validation. The **only critical gap** is the missing API to assign a round group to an area.

**User decision required before proceeding with Phase 30 execution.**

All other gaps (frontend bindings, UI components) are expected work within Phase 30 scope and do not block execution.

---

**Gap Analysis Complete.**
**Status:** ⏸️ **PAUSED — Awaiting User Guidance on Gap 1**
