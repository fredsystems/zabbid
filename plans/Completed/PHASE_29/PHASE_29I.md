# Phase 29I — Dead Code Cleanup and Remaining Route Wiring

## Purpose

Complete the HTTP API surface by wiring all remaining Phase 29 handlers to server routes and eliminate technical debt by removing unnecessary `#[allow(dead_code)]` annotations.

This sub-phase ensures that all implemented functionality is accessible via HTTP endpoints and that the codebase has minimal technical debt before proceeding to bidding workflows.

---

## Scope

### A. Remove Safe-to-Remove Annotations (20 instances)

Remove `#[allow(dead_code)]` annotations from code that is already in use:

**API Handlers (7 instances)** — `crates/api/src/handlers.rs`:

- `set_active_bid_year` (L2422)
- `get_active_bid_year` (L3370)
- `set_expected_area_count` (L3403)
- `set_expected_user_count` (L3491)
- `update_user` (L3641)
- `get_bootstrap_completeness` (L3783)
- `override_area_assignment` (L4451)

**Domain Types (4 instances)** — `crates/domain/src/types.rs`:

- `RoundGroup` struct (L801)
- `impl RoundGroup` (L816, L896)
- `Round` struct (L911)
- `impl Round` (L939, L1088)

**Request/Response Types** — Verify and handle:

- Old CSV types (L868-956) — Determine if deprecated, delete if unused

---

### B. Wire Phase 29B Round Management Handlers (8 handlers)

Implement server-side HTTP handlers and routes for round group and round management.

#### Round Group Endpoints

**POST `/api/round-groups`** — Create round group

- Handler: `handle_create_round_group`
- Auth: Admin only
- Request: `CreateRoundGroupApiRequest`
- Calls: `create_round_group`

**GET `/api/round-groups?bid_year_id={id}`** — List round groups

- Handler: `handle_list_round_groups`
- Auth: Authenticated
- Query: `ListRoundGroupsQuery { bid_year_id }`
- Calls: `list_round_groups`

**PUT `/api/round-groups/{id}`** — Update round group

- Handler: `handle_update_round_group`
- Auth: Admin only
- Path: `round_group_id`
- Request: `UpdateRoundGroupApiRequest`
- Calls: `update_round_group`

**DELETE `/api/round-groups/{id}`** — Delete round group

- Handler: `handle_delete_round_group`
- Auth: Admin only
- Path: `round_group_id`
- Request: `DeleteRoundGroupApiRequest`
- Calls: `delete_round_group`

#### Round Endpoints

**POST `/api/rounds`** — Create round

- Handler: `handle_create_round`
- Auth: Admin only
- Request: `CreateRoundApiRequest`
- Calls: `create_round`

**GET `/api/rounds?bid_year_id={id}&area_id={id}`** — List rounds

- Handler: `handle_list_rounds`
- Auth: Authenticated
- Query: `ListRoundsQuery { bid_year_id, area_id }`
- Calls: `list_rounds`

**PUT `/api/rounds/{id}`** — Update round

- Handler: `handle_update_round`
- Auth: Admin only
- Path: `round_id`
- Request: `UpdateRoundApiRequest`
- Calls: `update_round`

**DELETE `/api/rounds/{id}`** — Delete round

- Handler: `handle_delete_round`
- Auth: Admin only
- Path: `round_id`
- Request: `DeleteRoundApiRequest`
- Calls: `delete_round`

**Implementation Notes:**

- All handlers already exported from `crates/api/src/lib.rs`
- Request/response types already defined in `crates/api/src/request_response.rs`
- Follow existing server handler patterns from Phase 29A/C/G
- Remove `#[allow(dead_code)]` annotations after wiring

---

### C. Wire Phase 29D Readiness Handlers (3 handlers)

#### Readiness Endpoints

**GET `/api/readiness/{bid_year_id}`** — Get readiness evaluation

- Handler: `handle_get_bid_year_readiness`
- Auth: Admin only (readiness details may be sensitive)
- Path: `bid_year_id`
- Calls: `get_bid_year_readiness`
- Returns: Comprehensive readiness status with blocking reasons

**POST `/api/users/{user_id}/review-no-bid`** — Mark No Bid user as reviewed

- Handler: `handle_review_no_bid_user`
- Auth: Admin only
- Path: `user_id`
- Request: `ReviewNoBidUserApiRequest { cause_id, cause_description }`
- Calls: `review_no_bid_user`

**GET `/api/bid-order/preview?bid_year_id={id}&area_id={id}`** — Preview bid order

- Handler: `handle_get_bid_order_preview`
- Auth: Authenticated
- Query: `GetBidOrderPreviewQuery { bid_year_id, area_id }`
- Calls: `get_bid_order_preview`
- Returns: Derived bid order without persisting

**Implementation Notes:**

- Remove `#[allow(dead_code)]` from handlers and related types
- Ensure proper error handling for readiness validation failures
- Preview endpoint is read-only, no audit events

---

### D. Wire Phase 29E Confirmation Handler (1 handler - CRITICAL)

**POST `/api/confirm-ready-to-bid`** — Confirm readiness and enter bidding

- Handler: `handle_confirm_ready_to_bid`
- Auth: Admin only
- Request: `ConfirmReadyToBidApiRequest`
- Calls: `confirm_ready_to_bid`
- Returns: Confirmation with statistics (users, bid order count, windows created)

**Critical Workflow:**

- This is the irreversible transition to bidding
- Must validate readiness before executing
- Materializes bid order and calculates bid windows
- Updates lifecycle state to Canonicalized
- After this point, structural edits are locked

**Implementation Notes:**

- Already implemented in API layer (Phase 29E)
- Already exported from `crates/api/src/lib.rs`
- Handler marked `#[allow(dead_code)]` at L6805
- Remove annotation after wiring

---

### E. Wire Override Handlers (3 handlers)

**POST `/api/users/override-eligibility`** — Override user eligibility

- Handler: `handle_override_eligibility`
- Auth: Admin only
- Request: `OverrideEligibilityApiRequest`
- Calls: `override_eligibility`

**POST `/api/users/override-bid-order`** — Override single user bid order

- Handler: `handle_override_bid_order`
- Auth: Admin only
- Request: `OverrideBidOrderApiRequest`
- Calls: `override_bid_order`

**POST `/api/users/override-bid-window`** — Override single user bid window

- Handler: `handle_override_bid_window`
- Auth: Admin only
- Request: `OverrideBidWindowApiRequest`
- Calls: `override_bid_window`

**Implementation Notes:**

- All handlers already exported
- Follow existing override pattern from `override_area_assignment`
- Require minimum 10-character reason
- Generate audit events

---

### F. Verify and Clean Up Deprecated Code

**Old CSV Types Investigation:**

Investigate these types in `crates/api/src/request_response.rs` (L868-956):

- `CsvUserRow`
- `PreviewCsvRequest`
- `PreviewCsvResponse`
- `ImportSelectedUsersRequest`
- `UserImportResult`
- `ImportSelectedUsersResponse`

**Action Plan:**

1. Search codebase for usage
2. If unused, delete entirely (better than keeping with annotation)
3. If used, verify correctness and remove annotation
4. Document decision in working state

---

## Explicit Non-Goals

- Do NOT remove annotations from:
  - Test helpers (`crates/api/src/tests/helpers.rs`)
  - Internal implementation helpers (e.g., `*_impl` functions)
  - Diesel data models and query helpers
  - Xtask tooling artifacts
- Do NOT add new features or modify domain logic
- Do NOT change API contracts or request/response types
- Do NOT refactor working code "for consistency"

---

## Completion Checklist

### Part A: Safe Removals

- [ ] Remove `#[allow(dead_code)]` from 7 API handlers
- [ ] Remove `#[allow(dead_code)]` from `RoundGroup` and `Round` types
- [ ] Investigate and handle old CSV types (delete or remove annotation)
- [ ] Build passes: `cargo build`
- [ ] Tests pass: `cargo test --lib`

### Part B: Round Management (8 handlers)

- [ ] Add server-side request types for round groups (4 types)
- [ ] Add server-side request types for rounds (4 types)
- [ ] Implement `handle_create_round_group`
- [ ] Implement `handle_list_round_groups`
- [ ] Implement `handle_update_round_group`
- [ ] Implement `handle_delete_round_group`
- [ ] Implement `handle_create_round`
- [ ] Implement `handle_list_rounds`
- [ ] Implement `handle_update_round`
- [ ] Implement `handle_delete_round`
- [ ] Wire all 8 routes in `build_router`
- [ ] Remove `#[allow(dead_code)]` from all 8 handlers

### Part C: Readiness (3 handlers)

- [ ] Add server-side request types
- [ ] Implement `handle_get_bid_year_readiness`
- [ ] Implement `handle_review_no_bid_user`
- [ ] Implement `handle_get_bid_order_preview`
- [ ] Wire all 3 routes in `build_router`
- [ ] Remove `#[allow(dead_code)]` from handlers and types

### Part D: Confirmation (1 handler - CRITICAL)

- [ ] Add server-side request type
- [ ] Implement `handle_confirm_ready_to_bid`
- [ ] Wire route in `build_router`
- [ ] Remove `#[allow(dead_code)]` from handler
- [ ] Test confirmation workflow end-to-end

### Part E: Overrides (3 handlers)

- [ ] Add server-side request types
- [ ] Implement `handle_override_eligibility`
- [ ] Implement `handle_override_bid_order`
- [ ] Implement `handle_override_bid_window`
- [ ] Wire all 3 routes in `build_router`
- [ ] Remove `#[allow(dead_code)]` from all 3 handlers

### Final Verification

- [ ] All files added to git: `git add <files>`
- [ ] Build passes: `cargo build --bin zab-bid-server`
- [ ] All tests pass: `cargo test --lib`
- [ ] CI passes: `cargo xtask ci`
- [ ] Pre-commit passes: `pre-commit run --all-files`
- [ ] Update `PHASE_29_WORKING_STATE.md`
- [ ] Update `DEAD_CODE_AUDIT.md` with results

---

## Stop-and-Ask Conditions

Stop if:

- Any handler requires domain logic changes
- Request/response types are missing or incorrect
- Authorization model is unclear for any endpoint
- Lifecycle constraints are ambiguous
- Old CSV types appear to be in use but behave differently than expected
- Build or test failures cannot be resolved in 1-2 attempts

---

## Risk Notes

- **Round management endpoints** will enable UI workflow for round configuration
- **Confirmation endpoint** is irreversible — must have proper safeguards
- **Override endpoints** bypass normal validation — require admin-only auth
- Removing annotations may reveal actual dead code that should be deleted
- Large number of routes added in one phase — test thoroughly

---

## Success Criteria

Phase 29I is complete when:

1. All 20 safe-to-remove annotations are eliminated
2. All 15 remaining handlers are wired to HTTP routes
3. No `#[allow(dead_code)]` annotations remain on Phase 29 production code
4. All routes properly authenticated and authorized
5. Build, tests, CI, and pre-commit all pass
6. `DEAD_CODE_AUDIT.md` updated to reflect completion
7. System has complete HTTP API coverage for all implemented Phase 29 functionality

---

## Expected Outcome

After Phase 29I completion:

- **Zero Phase 29 technical debt** — All handlers accessible via HTTP
- **Complete pre-bid API** — All workflows from bootstrap through confirmation ready
- **Clean codebase** — Minimal necessary `#[allow(dead_code)]` annotations
- **Production ready** — System ready for Phase 29H (deployment) and Phase 30+ (bidding workflows)
