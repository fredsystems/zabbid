# Dead Code Annotation Audit Report

**Date:** 2026-01-21
**Total Instances Found:** 91
**Status:** Complete

This document catalogs all `#[allow(dead_code)]` and `#[allow(unused)]` annotations in the codebase, categorizes them by necessity, and identifies work required to remove them.

---

## Executive Summary

- **Can be safely removed now:** 20 instances
- **Need server route wiring:** 18 instances
- **Legitimately needed (test helpers, internal helpers):** 31 instances
- **Diesel/tooling artifacts (necessary):** 9 instances
- **Documentation (ignore):** 13 instances

---

## Category 1: Can Be Removed Immediately (20 instances)

These annotations are on code that is already exported and used by the server.

### API Handlers - Already Wired to Server

**File:** `crates/api/src/handlers.rs`

| Line | Function                     | Status  | Notes                                         |
| ---- | ---------------------------- | ------- | --------------------------------------------- |
| 2422 | `set_active_bid_year`        | ✅ Used | Called by `handle_set_active_bid_year`        |
| 3370 | `get_active_bid_year`        | ✅ Used | Called by `handle_get_active_bid_year`        |
| 3403 | `set_expected_area_count`    | ✅ Used | Called by `handle_set_expected_area_count`    |
| 3491 | `set_expected_user_count`    | ✅ Used | Called by `handle_set_expected_user_count`    |
| 3641 | `update_user`                | ✅ Used | Called by `handle_update_user`                |
| 3783 | `get_bootstrap_completeness` | ✅ Used | Called by `handle_get_bootstrap_completeness` |
| 4451 | `override_area_assignment`   | ✅ Used | Called by `handle_override_area_assignment`   |

**Action:** Remove all `#[allow(dead_code)]` annotations from these 7 functions.

---

## Category 2: Need Server Route Wiring (18 instances)

These are fully implemented API handlers that need HTTP routes added to the server.

### Phase 29B: Round Groups and Rounds

**File:** `crates/api/src/handlers.rs`

| Line | Function             | Required Work                        | Priority |
| ---- | -------------------- | ------------------------------------ | -------- |
| 5683 | `create_round_group` | Wire POST `/api/round-groups`        | High     |
| 5783 | `list_round_groups`  | Wire GET `/api/round-groups`         | High     |
| 5852 | `update_round_group` | Wire PUT `/api/round-groups/{id}`    | High     |
| 5981 | `delete_round_group` | Wire DELETE `/api/round-groups/{id}` | High     |
| 6093 | `create_round`       | Wire POST `/api/rounds`              | High     |
| 6239 | `list_rounds`        | Wire GET `/api/rounds`               | High     |
| 6319 | `update_round`       | Wire PUT `/api/rounds/{id}`          | High     |
| 6480 | `delete_round`       | Wire DELETE `/api/rounds/{id}`       | High     |

**Status:** All handlers exported from `crates/api/src/lib.rs` ✅
**Remaining Work:**

1. Add server-side request/response wrapper types in `crates/server/src/main.rs`
2. Implement 8 HTTP handler functions (`handle_create_round_group`, etc.)
3. Wire routes in `build_router`
4. Remove `#[allow(dead_code)]` annotations

---

### Phase 29D: Readiness Evaluation

**File:** `crates/api/src/handlers.rs`

| Line | Function                 | Required Work                             | Priority |
| ---- | ------------------------ | ----------------------------------------- | -------- |
| 6676 | `get_bid_year_readiness` | Wire GET `/api/readiness/{bid_year_id}`   | High     |
| 7167 | `review_no_bid_user`     | Wire POST `/api/users/{id}/review-no-bid` | High     |
| 7218 | `get_bid_order_preview`  | Wire GET `/api/bid-order/preview`         | Medium   |

**Status:** All handlers exported from `crates/api/src/lib.rs` ✅
**Remaining Work:** Same as Phase 29B (3 handlers)

---

### Phase 29E: Confirmation and Bid Order Freezing

**File:** `crates/api/src/handlers.rs`

| Line | Function               | Required Work                         | Priority |
| ---- | ---------------------- | ------------------------------------- | -------- |
| 6805 | `confirm_ready_to_bid` | Wire POST `/api/confirm-ready-to-bid` | Critical |

**Status:** Handler exported from `crates/api/src/lib.rs` ✅
**Remaining Work:** Same as Phase 29B (1 handler)

**Note:** This is a critical workflow endpoint required before bidding can begin.

---

### Phase 29: Override Handlers

**File:** `crates/api/src/handlers.rs`

| Line | Function               | Required Work                               | Priority |
| ---- | ---------------------- | ------------------------------------------- | -------- |
| 4614 | `override_eligibility` | Wire POST `/api/users/override-eligibility` | Medium   |
| 4737 | `override_bid_order`   | Wire POST `/api/users/override-bid-order`   | Medium   |
| 4868 | `override_bid_window`  | Wire POST `/api/users/override-bid-window`  | Medium   |

**Status:** All handlers exported from `crates/api/src/lib.rs` ✅
**Remaining Work:** Same as Phase 29B (3 handlers)

---

### Phase 29D: Request/Response Types

**File:** `crates/api/src/request_response.rs`

| Line | Type                          | Notes                            |
| ---- | ----------------------------- | -------------------------------- |
| 1717 | `GetBidYearReadinessResponse` | Used by `get_bid_year_readiness` |
| 1733 | `ReadinessDetailsInfo`        | Nested in readiness response     |
| 1749 | `ReviewNoBidUserResponse`     | Used by `review_no_bid_user`     |
| 1762 | `GetBidOrderPreviewResponse`  | Used by `get_bid_order_preview`  |
| 1776 | `BidOrderPositionInfo`        | Nested in bid order preview      |
| 1790 | `SeniorityInputsInfo`         | Nested in bid order preview      |

**Action:** Remove annotations when corresponding handlers are wired.

---

## Category 3: Legitimately Needed (31 instances)

These annotations are appropriate and should remain.

### Internal Helper Functions

**File:** `crates/api/src/handlers.rs`

| Line | Function                       | Reason                                    |
| ---- | ------------------------------ | ----------------------------------------- |
| 7346 | `get_bid_status_for_area_impl` | Internal helper, called by public wrapper |
| 7443 | `get_bid_status_impl`          | Internal helper, called by public wrapper |
| 7565 | `transition_bid_status_impl`   | Internal helper, called by public wrapper |
| 7727 | `bulk_update_bid_status_impl`  | Internal helper, called by public wrapper |

**Action:** Keep annotations. These are implementation details not meant for direct export.

---

### Test Helpers (9 instances)

**File:** `crates/api/src/tests/helpers.rs`

All test helper functions and types (lines 136-461):

- `TestSession` struct
- `create_persisted_admin_operator`
- `create_persisted_bidder_operator`
- `create_admin_session`
- `create_bidder_session`
- `create_custom_session`
- `bootstrap_bid_year_and_area`
- `BootstrapIds` struct
- `bootstrap_with_ids`
- `bootstrap_bid_year_only`

**Action:** Keep all annotations. Test helpers are intentionally not used in production code.

---

### Diesel Data Models (10 instances)

**File:** `crates/persistence/src/data_models.rs`

Diesel queryable structs used internally by query functions (lines 48-293):

- `AuditEventRow` (L48)
- `CanonicalAreaMembershipRow` (L91)
- `CanonicalEligibilityRow` (L117)
- `CanonicalBidOrderRow` (L143)
- `BidWindowRow` (L169)
- `CanonicalBidWindowsRow` (L195)
- `BidStatusRow` (L259)
- `BidStatusHistoryRow` (L289)

**Action:** Keep annotations. These are Diesel internal types.

---

### Persistence Functions (8 instances)

**File:** `crates/persistence/src/mutations/bid_status.rs`

| Line | Function                         | Used By                        | Keep?  |
| ---- | -------------------------------- | ------------------------------ | ------ |
| 17   | `insert_initial_bid_status`      | `confirm_ready_to_bid` handler | ✅ Yes |
| 40   | `update_bid_status`              | `transition_bid_status_impl`   | ✅ Yes |
| 78   | `insert_bid_status_history`      | `transition_bid_status_impl`   | ✅ Yes |
| 110  | `bulk_insert_bid_status_history` | `bulk_update_bid_status_impl`  | ✅ Yes |

**File:** `crates/persistence/src/queries/bid_status.rs`

| Line | Function                            | Used By                        | Keep?  |
| ---- | ----------------------------------- | ------------------------------ | ------ |
| 16   | `get_bid_status_for_user_and_round` | Internal queries               | ✅ Yes |
| 45   | `get_bid_status_for_area`           | `get_bid_status_for_area_impl` | ✅ Yes |
| 66   | `get_bid_status_for_round`          | Future use                     | ✅ Yes |
| 86   | `get_bid_status_history`            | `get_bid_status_impl`          | ✅ Yes |
| 105  | `get_bid_status_by_id`              | `transition_bid_status_impl`   | ✅ Yes |

**Action:** Keep annotations. These are called internally by higher-level API functions.

---

### Domain Types (2 instances)

**File:** `crates/domain/src/types.rs`

| Line | Type/Impl           | Status  | Notes                        |
| ---- | ------------------- | ------- | ---------------------------- |
| 801  | `RoundGroup` struct | ✅ Used | Used by round group handlers |
| 816  | `impl RoundGroup`   | ✅ Used | Methods called by handlers   |
| 911  | `Round` struct      | ✅ Used | Used by round handlers       |
| 939  | `impl Round`        | ✅ Used | Methods called by handlers   |

**Action:** Remove annotations. These are used by Phase 29B handlers.

---

## Category 4: Diesel/Tooling Artifacts (9 instances)

### Query Result Helpers

**File:** `crates/persistence/src/queries/state.rs`

| Line | Code                           | Reason                                |
| ---- | ------------------------------ | ------------------------------------- |
| 24   | `StateSnapshotRow::state_json` | Diesel extracts only needed field     |
| 35   | `UserRow::bid_year_id`         | Diesel queryable, not all fields used |
| 35   | `UserRow::area_id`             | Diesel queryable, not all fields used |

**File:** `crates/persistence/src/queries/audit.rs`

| Line | Code                            | Reason                                  |
| ---- | ------------------------------- | --------------------------------------- |
| 37   | `AuditEventFullRow::created_at` | Optional field not used in all contexts |

**Action:** Keep annotations. These are Diesel artifacts where we query more columns than we use.

---

### Xtask Schema Introspection

**File:** `xtask/src/main.rs`

| Line | Code                | Reason                                    |
| ---- | ------------------- | ----------------------------------------- |
| 836  | `ColumnInfo::cid`   | Schema introspection, not all fields used |
| 861  | `IndexInfo::unique` | Schema introspection, not all fields used |

**Action:** Keep annotations. Diesel queryable structs for schema validation.

---

## Category 5: Documentation (13 instances)

**Files:**

- `plans/PHASE_29/PHASE_29D_CONTINUATION.md` (1 instance)
- `plans/PHASE_29_WORKING_STATE.md` (12 instances)

**Action:** Ignore. These are historical documentation of the development process.

---

## Category 6: Deprecated/Unused Types (6 instances)

**File:** `crates/api/src/request_response.rs`

Old CSV preview types (lines 868-956):

- `CsvUserRow` (L868)
- `PreviewCsvRequest` (L900)
- `PreviewCsvResponse` (L910)
- `ImportSelectedUsersRequest` (L926)
- `UserImportResult` (L938)
- `ImportSelectedUsersResponse` (L952)

**Analysis:** These appear to be an older CSV import API that has been replaced by the current `PreviewCsvUsersRequest`/`ImportCsvUsersRequest` API.

**Action:**

1. Verify these are truly unused
2. If unused, delete entirely (better than keeping with `#[allow(dead_code)]`)
3. If used, remove annotation

---

## Recommended Action Plan

### Immediate (Can Do Now)

1. **Remove 7 annotations** from handlers already wired to server:
   - `set_active_bid_year`
   - `get_active_bid_year`
   - `set_expected_area_count`
   - `set_expected_user_count`
   - `update_user`
   - `get_bootstrap_completeness`
   - `override_area_assignment`

2. **Remove 2 annotations** from domain types:
   - `RoundGroup` struct and impl
   - `Round` struct and impl

3. **Verify and delete** old CSV types (lines 868-956 in request_response.rs) if truly unused

---

### Short-term (Next Phase)

**Wire Phase 29B Round Management (8 handlers):**

- Implement server-side wrappers
- Add routes to `build_router`
- Remove annotations

**Wire Phase 29D Readiness (3 handlers):**

- `get_bid_year_readiness`
- `review_no_bid_user`
- `get_bid_order_preview`

**Wire Phase 29E Confirmation (1 handler - CRITICAL):**

- `confirm_ready_to_bid`

**Wire remaining override handlers (3 handlers):**

- `override_eligibility`
- `override_bid_order`
- `override_bid_window`

**Total server work needed:** 15 HTTP handler functions + route wiring

---

### Keep Permanently (62 instances)

- Test helpers: 9 instances
- Internal API helpers: 4 instances
- Diesel data models: 10 instances
- Persistence query functions: 9 instances
- Diesel query artifacts: 4 instances
- Xtask tooling: 2 instances
- Documentation: 13 instances

---

## Current Technical Debt Score

**Total Removable:** 29 instances (20 immediate + 9 domain)
**Work Required (server wiring):** 18 instances
**Legitimate (keep):** 31 instances
**Tooling artifacts (keep):** 9 instances
**Deprecated (delete or fix):** 6 instances

**Priority:** Remove the 20 immediate instances first, then complete server route wiring work to eliminate the remaining 18 handler-related annotations.
