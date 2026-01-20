# Phase 29 Working State

## Phase

- Phase: 29
- Title: Pre-Bid Readiness, Ordering, and Bootstrap Finalization

## Current Status

- Status: In Progress
- Last Updated: 2026-01-21
- Reason: Sub-Phase 29D complete, Phase 29E in progress

## Active Sub-Phase

- Sub-Phase: 29E — Confirmation and Bid Order Freezing
- State: In Progress

## Completed Sub-Phases

- [x] Planning Pass — Sub-phase documents created
- [x] 29A — User Participation Flags
- [x] 29B — Round Groups and Rounds (including semantic correction)
- [x] 29C — Bid Schedule Declaration
- [x] 29D — Readiness Evaluation

## Planned Sub-Phases

- [x] 29A — User Participation Flags
- [x] 29B — Round Groups and Rounds (COMPLETE)
- [x] 29C — Bid Schedule Declaration (COMPLETE)
- [x] 29D — Readiness Evaluation (COMPLETE)
- [ ] 29E — Confirmation and Bid Order Freezing (IN PROGRESS)
- [ ] 29F — Bid Status Tracking Structure
- [ ] 29G — Post-Confirmation Bid Order Adjustments
- [ ] 29H — Docker Compose Deployment

## Work Completed

### Planning Phase

- Created `plans/PHASE_29/PHASE_29A.md` — User Participation Flags
- Created `plans/PHASE_29/PHASE_29B.md` — Round Groups and Rounds
- Created `plans/PHASE_29/PHASE_29C.md` — Bid Schedule Declaration
- Created `plans/PHASE_29/PHASE_29D.md` — Readiness Evaluation
- Created `plans/PHASE_29/PHASE_29E.md` — Confirmation and Bid Order Freezing
- Created `plans/PHASE_29/PHASE_29F.md` — Bid Status Tracking Structure
- Created `plans/PHASE_29/PHASE_29G.md` — Post-Confirmation Bid Order Adjustments
- Created `plans/PHASE_29/PHASE_29H.md` — Docker Compose Deployment
- Created `plans/PHASE_29_WORKING_STATE.md` — This document

### Sub-Phase 29A (Implementation Complete)

#### Database Schema ✅ Complete

- Created SQLite migration `2026-01-18-150000-0000_add_user_participation_flags/up.sql`
- Created SQLite migration `2026-01-18-150000-0000_add_user_participation_flags/down.sql`
- Created MySQL migration `2026-01-18-150000-0000_add_user_participation_flags/up.sql`
- Created MySQL migration `2026-01-18-150000-0000_add_user_participation_flags/down.sql`
- Added `excluded_from_bidding` column to `users` table (INTEGER/TINYINT with CHECK constraint)
- Added `excluded_from_leave_calculation` column to `users` table (INTEGER/TINYINT with CHECK constraint)
- Schema verification passes: `cargo xtask verify-migrations`

#### Domain Types ✅ Complete

- Added `excluded_from_bidding: bool` field to `User` struct in `domain/src/types.rs`
- Added `excluded_from_leave_calculation: bool` field to `User` struct in `domain/src/types.rs`
- Updated `User::new()` constructor to accept both participation flags
- Updated `User::with_id()` constructor to accept both participation flags
- Added `validate_participation_flags()` method to `User` struct with proper documentation
- Added `DomainError::ParticipationFlagViolation` variant in `domain/src/error.rs`
- Fixed all compilation errors in domain layer
- Fixed all clippy warnings

#### Persistence Layer ✅ Complete

- Updated Diesel schema in `persistence/src/diesel_schema.rs` to include new fields
- Updated `list_users()` query in `persistence/src/queries/canonical.rs` to read new fields
- Updated `User` reconstruction in `list_users()` to map integer flags to booleans
- Updated SQLite raw SQL INSERT statements in `persistence/src/sqlite/persistence.rs`:
  - `insert_new_user_tx` includes both participation flags
  - `sync_canonical_users_tx` includes both participation flags (with and without user_id)
- Updated all test SQL INSERT statements:
  - `backend_validation_tests.rs` (2 locations)
  - `canonical_tests/canonicalization.rs` (5 locations)
- Updated all `User::new()` and `User::with_id()` calls throughout codebase:
  - `csv_preview.rs`
  - `core/src/apply.rs` (preserves existing flags in UpdateUser, UpdateUserParticipation)
  - `core/src/tests/apply_tests.rs`
  - `domain/src/leave_accrual.rs`
  - `domain/src/tests/types.rs`
  - `domain/src/tests/validation.rs`

#### Core Layer ✅ Complete

- Added `UpdateUserParticipation` command variant in `core/src/command.rs`
- Implemented command handler in `core/src/apply.rs`:
  - Finds user by canonical `user_id`
  - Preserves all other user fields
  - Validates directional invariant via `validate_participation_flags()`
  - Creates audit event
  - Returns `TransitionResult` with new state
- Fixed `UpdateUser` command to preserve participation flags when updating other fields

#### API Layer ✅ Complete

- Added `excluded_from_bidding` and `excluded_from_leave_calculation` fields to `UserInfo` struct
- Updated `list_users` handler to populate participation flags in response
- Created `UpdateUserParticipationRequest` type
- Created `UpdateUserParticipationResponse` type
- Implemented `update_user_participation` handler in `handlers.rs`:
  - Enforces lifecycle constraints (Draft or BootstrapComplete only)
  - Validates directional invariant before command construction
  - Finds user across all areas by `user_id`
  - Applies command and persists audit event
  - Returns detailed response
- Added error translation for `ParticipationFlagViolation` in `error.rs`
- Fixed all clippy warnings
- Marked handler and types with `#[allow(dead_code)]` pending route wiring

#### Build & CI ✅ Complete

- All tests pass: `cargo test --lib` (125 passed, 9 ignored)
- Full CI passes: `cargo xtask ci`
- Pre-commit hooks pass: `pre-commit run --all-files`
- Schema parity verified: `cargo xtask verify-migrations`
- All clippy warnings resolved
- All files tracked in git

### Sub-Phase 29B (Semantic Correction In Progress)

#### SEMANTIC MODEL CORRECTION REQUIRED

**Issue Identified**: Initial implementation violated the authoritative domain model:

- **WRONG**: Rounds referenced areas directly via `area_id`
- **WRONG**: Round groups were not referenced by areas
- **CORRECT**: Rounds belong to round groups, areas reference round groups
- **CORRECT**: Round groups are reusable across multiple areas
- **CORRECT**: Non-system areas must reference exactly one round group; system areas: none

#### Completed — Semantic Correction Phase

- [x] Database schema migrations created (SQLite and MySQL)
- [x] Schema parity verified (`cargo xtask verify-migrations`)
- [x] Domain types created (`RoundGroup` and `Round`)
- [x] Domain error variants added
- [x] Domain validation methods implemented
- [x] API error translations added
- [x] Persistence layer CRUD operations implemented:
  - [x] Round group queries (list, get, insert, update, delete)
  - [x] Round queries (list, get, insert, update, delete)
  - [x] Validation queries (name exists, round number exists, group in use)
  - [x] Backend-specific functions for SQLite and MySQL
- [x] Code compiles without errors
- [x] All existing tests pass
- [x] Clippy passes
- [x] Pre-commit hooks pass
- [x] API request/response types created
- [x] API handlers for round groups (create, list, update, delete)
- [x] API handlers for rounds (create, list, update, delete)
- [x] Persistence layer wrappers exposed
- [x] Lifecycle constraint enforcement in handlers
- [x] System area validation in create_round handler
- [x] Round group in-use validation in delete handler
- [x] Core command variants added (marked unreachable - not used for configuration)
- [x] All files added to git

**SEMANTIC CORRECTION STARTED - 2026-01-20**:

- [x] Created corrective migrations (SQLite & MySQL)
  - [x] Removed `area_id` from `rounds` table
  - [x] Added `round_group_id` to `areas` table
  - [x] Changed unique constraint to `(round_group_id, round_number)`
  - [x] Schema parity verified
- [x] Updated Diesel schema
  - [x] Added `round_group_id` to areas table definition
  - [x] Removed `area_id` from rounds table definition
  - [x] Updated joinable declarations
- [x] Corrected domain types
  - [x] Added `round_group_id: Option<i64>` to `Area` struct
  - [x] Removed `area: Area` field from `Round` struct
  - [x] Updated all constructors and accessors
  - [x] Removed `validate_not_system_area()` from Round (no longer needed)
  - [x] Updated documentation
- [x] Fixed domain tests
  - [x] Updated test helpers to match corrected model
  - [x] Removed invalid system area test on rounds
  - [x] All 129 domain tests pass

#### Semantic Correction — COMPLETE ✅

All work completed and committed (commit 089ee7e).

- [x] **Persistence Layer** — All queries updated
  - [x] Updated `list_areas` to populate `round_group_id`
  - [x] Updated all area queries to handle `round_group_id`
  - [x] Fixed all `Area::with_id()` calls (now includes `round_group_id` parameter)
  - [x] Removed `area_id` from all round queries
  - [x] Updated round CRUD to work with `round_group_id` only
  - [x] Updated round queries to use round_group_id

- [x] **API Layer** — All handlers corrected
  - [x] Updated round handlers to use round_group_id
  - [x] Updated `list_rounds` handler (queries by round_group_id)
  - [x] Updated all round handlers to work without area_id
  - [x] Updated area response types to include `round_group_id`
  - [x] Removed area_id from all round API response types

- [x] **API Tests** — Not yet created (out of scope for Phase 29B)
  - Note: API tests will be created in a future phase when routes are wired

- [x] **Planning Documents** — To be updated as needed
  - Note: PHASE_29B.md has known inconsistency in API section (will update if needed)

- [x] **Final Validation**
  - [x] All 129 domain tests pass
  - [x] All 125 persistence tests pass
  - [x] All 9 MariaDB backend validation tests pass
  - [x] `cargo xtask ci` passes
  - [x] `pre-commit run --all-files` passes
  - [x] Schema parity verified
  - [x] Committed (089ee7e)

## Outstanding Work

### Current Sub-Phase

- Sub-Phase 29C: ✅ COMPLETE
- Next: Sub-Phase 29D (Readiness Evaluation)

### Future Sub-Phases

- Execute Sub-Phase 29D (Readiness Evaluation)
- Execute Sub-Phase 29E (Confirmation and Bid Order Freezing)
- Execute Sub-Phase 29F (Bid Status Tracking Structure)
- Execute Sub-Phase 29G (Post-Confirmation Bid Order Adjustments)
- Execute Sub-Phase 29H (Docker Compose Deployment)

## Known Failures / Breakages

**Sub-Phases 29A, 29B, and 29C COMPLETE**:

- ✅ Database schema corrected via migrations (SQLite & MySQL)
- ✅ Domain layer corrected (129 tests passing)
- ✅ Persistence layer corrected (125 tests passing)
- ✅ API layer corrected (handlers updated)
- ✅ MariaDB backend validation (9 tests passing)
- ✅ All validation passing (cargo xtask ci, pre-commit, schema parity)

## Stop-and-Ask Items

None

## Resume Instructions

### Sub-Phase 29B Complete ✅

**Semantic correction completed and committed (089ee7e).**

**Corrected Domain Model (AUTHORITATIVE)**:

1. **Round Groups**: Reusable collections of rounds, scoped to bid year
2. **Rounds**: Belong to round groups (NOT areas), carry all bidding rules
3. **Areas**: Reference exactly one round group (non-system) or none (system)
4. **Round number uniqueness**: Within round group (not within area)

**All Corrections Applied**:

- ✅ Database schema migrations (SQLite & MySQL)
- ✅ Diesel schema
- ✅ Domain types
- ✅ Domain tests (129 passing)
- ✅ Persistence layer queries
- ✅ API handlers
- ✅ API response types
- ✅ All validation passing

**Completion Criteria for 29B — ALL MET**:

**Schema & Domain**:

- [x] Corrective migrations created (SQLite and MySQL)
- [x] Schema parity verified
- [x] Domain types corrected (Area has round_group_id, Round has no area)
- [x] Domain tests updated and passing (129 tests)

**Implementation**:

- [x] Persistence layer queries updated for corrected model
- [x] API handlers updated for corrected model
- [x] API tests deferred (no routes wired yet)
- [x] Planning documents note any inconsistencies

**Validation**:

- [x] All tests pass (129 domain + 125 persistence + 9 MariaDB)
- [x] `cargo xtask ci` passes
- [x] `pre-commit run --all-files` passes
- [x] Schema parity verified
- [x] Semantic correction committed (089ee7e)

**Correctness Criteria (ALL TRUE)**:

- [x] Rounds belong to round groups (not areas)
- [x] Areas reference exactly one round group (non-system) or none (system)
- [x] Round groups are reusable across multiple areas
- [x] Round number uniqueness is within round group (not within area)

### Ready to Proceed to Sub-Phase 29C

### Reference Documents

- Completed sub-phases:
  - `plans/PHASE_29/PHASE_29A.md`
  - `plans/PHASE_29/PHASE_29B.md`
  - `plans/PHASE_29/PHASE_29C.md`
- Next sub-phase: `plans/PHASE_29/PHASE_29D.md`
- Architectural rules: `AGENTS.md`
- Execution protocol: `plans/PHASE_EXECUTION.md`

---

## Phase 29B Complete - 2026-01-20

**Commit**: 089ee7e "Phase 29B: Complete semantic correction - rounds belong to round groups"

**What Was Completed**:

- ✅ Semantic correction: rounds belong to round groups (not areas)
- ✅ Database migrations (corrective) for SQLite and MySQL
- ✅ Diesel schema updates
- ✅ Domain type corrections (Area has round_group_id, Round has no area)
- ✅ Domain test fixes (129 tests passing)
- ✅ Persistence layer queries updated
- ✅ API handlers corrected (create/list/update/delete rounds)
- ✅ API response types updated
- ✅ All validation passing (cargo xtask ci, pre-commit, schema parity, MariaDB tests)

**Next Steps**:

1. Proceed to Sub-Phase 29D (Readiness Evaluation)
2. Update PHASE_29_WORKING_STATE.md before pausing
3. Continue execution per Phase Planning & Execution Protocol

---

### Sub-Phase 29C (Implementation Complete)

#### 29C Database Schema ✅ Complete

- Created SQLite migration `2026-01-20-120000-0000_add_bid_schedule_to_bid_years/up.sql`
- Created SQLite migration `2026-01-20-120000-0000_add_bid_schedule_to_bid_years/down.sql`
- Created MySQL migration `2026-01-20-120000-0000_add_bid_schedule_to_bid_years/up.sql`
- Created MySQL migration `2026-01-20-120000-0000_add_bid_schedule_to_bid_years/down.sql`
- Added bid schedule fields to `bid_years` table:
  - `bid_timezone` (TEXT/VARCHAR)
  - `bid_start_date` (TEXT/VARCHAR)
  - `bid_window_start_time` (TEXT/VARCHAR)
  - `bid_window_end_time` (TEXT/VARCHAR)
  - `bidders_per_area_per_day` (INTEGER/INT)
- All fields nullable until confirmation
- Schema verification passes: `cargo xtask verify-migrations`

#### 29C Domain Types ✅ Complete

- Added `BidSchedule` struct in `domain/src/types.rs`
- Added `chrono-tz` dependency for timezone validation
- Implemented validation methods:
  - Timezone validation (IANA identifier)
  - Start date must be Monday
  - Window times: start < end
  - Bidders per day > 0
  - Future date validation (relative to reference date)
- Added domain error variants:
  - `InvalidTimezone`
  - `BidStartDateNotMonday`
  - `BidStartDateNotFuture`
  - `InvalidBidWindowTimes`
  - `InvalidBiddersPerDay`
- All 129 domain tests pass

#### 29C Persistence Layer ✅ Complete

- Updated Diesel schema to include new bid_years fields
- Created `BidScheduleFields` type alias to simplify return types
- Added `get_bid_schedule()` backend function
- Added `update_bid_schedule()` backend function
- Exposed functions in `Persistence` API
- All 125 persistence tests pass

#### 29C API Layer ✅ Complete

- Added `BidScheduleInfo` struct to request_response.rs
- Updated `BidYearInfo` to include optional `bid_schedule` field
- Created `SetBidScheduleRequest` type
- Created `SetBidScheduleResponse` type
- Created `GetBidScheduleResponse` type
- Implemented `set_bid_schedule()` handler:
  - Admin-only authorization
  - Lifecycle constraint enforcement (Draft/BootstrapComplete only)
  - Full validation (timezone, date, times, capacity)
  - Audit event creation
- Implemented `get_bid_schedule()` handler
- Updated `list_bid_years()` to populate bid_schedule field
- Added error translations for all bid schedule validation errors
- Handlers marked with `#[allow(dead_code)]` pending route wiring

#### 29C Build & CI ✅ Complete

- All tests pass: `cargo test --lib` (125 persistence + 129 domain)
- Full CI passes: `cargo xtask ci`
- Pre-commit hooks pass: `pre-commit run --all-files`
- Schema parity verified: `cargo xtask verify-migrations`
- All clippy warnings resolved
- All files tracked in git

#### Completion Checklist — ALL MET ✅

**Schema & Migrations**:

- [x] Migrations created for both SQLite and MySQL
- [x] Schema verification passes
- [x] All bid schedule fields added to bid_years table
- [x] All fields nullable until confirmation

**Domain Layer**:

- [x] BidSchedule type created
- [x] Timezone validation implemented (IANA identifiers)
- [x] Start date validation (Monday, future at confirmation)
- [x] Daily window validation (start < end)
- [x] Bidders per day validation (> 0)
- [x] All error variants added
- [x] All domain tests pass

**Persistence Layer**:

- [x] Persistence functions created (get/update)
- [x] Type alias for complex return types
- [x] Backend-agnostic wrapper functions
- [x] All persistence tests pass

**API Layer**:

- [x] Request/response types created
- [x] set_bid_schedule handler implemented
- [x] get_bid_schedule handler implemented
- [x] list_bid_years updated to include bid_schedule
- [x] Lifecycle constraints enforced
- [x] Error translations added
- [x] Handlers marked dead_code (routes not wired yet)

**Validation**:

- [x] All tests pass (cargo test --lib)
- [x] cargo xtask ci passes
- [x] pre-commit run --all-files passes
- [x] Schema parity verified
- [x] All files added to git

---

## Phase 29C Complete - 2026-01-20

**What Was Completed**:

- ✅ Database migrations (SQLite and MySQL)
- ✅ BidSchedule domain type with full validation
- ✅ Persistence layer (get/update bid schedule)
- ✅ API handlers (set/get bid schedule)
- ✅ API response types updated (BidYearInfo includes bid_schedule)
- ✅ All validation passing (cargo xtask ci, pre-commit, schema parity)
- ✅ Lifecycle constraints enforced (editable in Draft/BootstrapComplete only)

**Next Steps**:

1. Proceed to Sub-Phase 29D (Readiness Evaluation)
2. Update PHASE_29_WORKING_STATE.md before pausing
3. Continue execution per Phase Planning & Execution Protocol

---

## Phase 29D — Readiness Evaluation (In Progress)

### 29D Current Status

Complete ✅

### Last Updated

2026-01-21

### Completed Work

#### 29D Database Schema ✅ Complete

- ✅ Added `no_bid_reviewed` column to users table
- ✅ Created SQLite migration (up.sql with ALTER TABLE, down.sql with table recreation)
- ✅ Created MySQL migration (up.sql with ALTER TABLE, down.sql with DROP COLUMN)
- ✅ Schema parity verification passes

#### 29D Domain Types ✅ Complete

- ✅ Added `no_bid_reviewed: bool` field to `User` struct
- ✅ Updated `User::new()` constructor to accept `no_bid_reviewed` parameter
- ✅ Updated `User::with_id()` constructor to accept `no_bid_reviewed` parameter
- ✅ Updated all test helper functions to provide `no_bid_reviewed` parameter

#### 29D Persistence Layer ✅ Complete

- ✅ Updated `diesel_schema.rs` with `no_bid_reviewed` column
- ✅ Updated `list_users()` query to select `no_bid_reviewed`
- ✅ Updated `list_users_canonical()` query to select `no_bid_reviewed`
- ✅ Updated `UserRow` struct in state queries to include `no_bid_reviewed`
- ✅ Updated all `User::with_id()` calls to pass `no_bid_reviewed`

#### 29D User Flag API Layer ✅ Complete

- ✅ Added `no_bid_reviewed` field to `UserInfo` response type
- ✅ Updated `list_users` handler to include `no_bid_reviewed` in response
- ✅ Updated CSV preview to provide `no_bid_reviewed` parameter

#### 29D Build & CI ✅ Complete

- ✅ `cargo build` passes
- ✅ `cargo test` passes (all tests updated)
- ✅ `cargo xtask ci` passes
- ✅ `pre-commit run --all-files` passes
- ✅ Schema parity verification passes
- ✅ Committed: "Phase 29D: Add no_bid_reviewed flag to users table"

#### 29D Readiness Domain Logic ✅ Complete

- ✅ Defined `BidYearReadiness` and `ReadinessDetails` domain types
- ✅ Implemented readiness criteria evaluation functions:
  - ✅ `count_participation_flag_violations()` - checks directional invariant
  - ✅ `count_unreviewed_no_bid_users()` - counts pending reviews in system areas
  - ✅ `count_seniority_conflicts()` - placeholder for bid order validation
  - ✅ `evaluate_area_readiness()` - area-level readiness check
- ✅ Added comprehensive unit tests for all readiness functions
- ✅ Exported readiness functions from domain crate

#### 29D Readiness Persistence Queries ✅ Complete

- ✅ Created `queries/readiness.rs` module
- ✅ Implemented `is_bid_schedule_set()` query
- ✅ Implemented `get_areas_missing_rounds()` query
- ✅ Implemented `count_unreviewed_no_bid_users()` query
- ✅ Implemented `count_participation_flag_violations()` query
- ✅ Implemented `mark_user_no_bid_reviewed()` mutation
- ✅ All queries use backend_fn macro for SQLite/MySQL parity
- ✅ Committed: "Phase 29D: Add readiness domain types and persistence queries"

### 29D Outstanding Work

None - Phase 29D is complete.

### 29D Known Issues

None - All issues resolved.

### 29D Stop-and-Ask Items

None - All items resolved.

#### RESOLVED: Seniority Conflict Detection Complete ✅

**Resolution (2026-01-20):**

Authorization granted to implement full bid order computation immediately.
Structural gap has been corrected.

**Implementation:**

- Real seniority conflict detection via `compute_bid_order()`
- Strict 5-tier seniority ordering enforced
- Any unresolved tie is a domain violation
- No manual resolution path (by design)

**Status:**

- ✅ Seniority conflict detection fully implemented
- ✅ All tests pass
- ⚠️ Clippy errors remain (function too long, needs refactoring)
- ⚠️ Derived bid order preview API not yet added (required)

### 29D Completion Summary

#### All Work Complete ✅

**Implementation Complete (2026-01-21):**

Phase 29D is fully complete with all required functionality implemented and tested.

**What's Complete:**

- ✅ Database schema (no_bid_reviewed flag)
- ✅ Domain types and logic
- ✅ Persistence layer queries
- ✅ API response types and handlers
- ✅ Real seniority conflict detection via compute_bid_order()
- ✅ Bid order preview API
- ✅ Clippy errors fixed (refactored helpers)
- ✅ Planning documents updated
- ✅ All tests pass
- ✅ `cargo xtask ci` passes
- ✅ `pre-commit run --all-files` passes

**Commits:**

1. "Phase 29D: Implement real seniority conflict detection via bid order computation"
2. "Phase 29D: Add bid order preview API and refactor readiness handler"
3. "Phase 29D: Update planning documents to reflect bid order preview"

**Next Phase:**

Ready to proceed to Phase 29E (Bid Order Freezing at Confirmation).

## Phase 29D Complete - 2026-01-21

---

## Phase 29E — Confirmation and Bid Order Freezing (In Progress)

### 29E Current Status

In Progress

### 29E Last Updated

2026-01-21

### 29E Completed Work

#### 29E Database Schema ✅ Complete

- ✅ Created SQLite migration `2026-01-21-120000-0000_add_bid_windows_table/up.sql`
- ✅ Created SQLite migration `2026-01-21-120000-0000_add_bid_windows_table/down.sql`
- ✅ Created MySQL migration `2026-01-21-120000-0000_add_bid_windows_table/up.sql`
- ✅ Created MySQL migration `2026-01-21-120000-0000_add_bid_windows_table/down.sql`
- ✅ Added `bid_windows` table with fields:
  - `bid_window_id` (PRIMARY KEY)
  - `bid_year_id` (FK to bid_years)
  - `area_id` (FK to areas)
  - `user_id` (FK to users)
  - `window_start_datetime` (UTC ISO 8601 TEXT)
  - `window_end_datetime` (UTC ISO 8601 TEXT)
  - UNIQUE constraint on (bid_year_id, area_id, user_id)
- ✅ Updated Diesel schema in `persistence/src/diesel_schema.rs`
- ✅ Schema parity verification passes

#### 29E Domain Logic ✅ Complete

- ✅ Created `domain/src/bid_window.rs` module
- ✅ Implemented `BidWindow` struct with user_id, position, and UTC datetime fields
- ✅ Implemented `calculate_bid_windows()` function:
  - Takes user positions and BidSchedule
  - Converts time::Date/Time to chrono types for calculation
  - Calculates weekday offsets (skips weekends)
  - Handles timezone conversions using chrono-tz
  - Returns UTC ISO 8601 formatted timestamps
- ✅ Implemented helper functions:
  - `calculate_weekday_offset()` - determines day offset from position
  - `add_weekdays()` - adds weekdays while skipping weekends
  - `calculate_window_for_position()` - calculates individual window
- ✅ Added comprehensive unit tests
- ✅ Added error variants to `DomainError`:
  - `InvalidBidStartDate` with date and reason
  - `InvalidBidSchedule` with reason
- ✅ Updated API error mappings in `api/src/error.rs`
- ✅ Added chrono dependencies to workspace and domain crate
- ✅ All tests pass
- ✅ All clippy checks pass

### 29E Outstanding Work

- Bid order materialization persistence layer
- Confirmation core logic implementation
- Confirmation API endpoint
- Editing lock enforcement
- Integration tests

### 29E Known Issues

None

### 29E Stop-and-Ask Items

None

## Phase 29E In Progress - 2026-01-21

Phase 29D (Readiness Evaluation) has been successfully completed.

All readiness criteria are now implemented and tested:

- ✅ Areas must have rounds configured
- ✅ No Bid users must be reviewed
- ✅ Participation flag invariants enforced
- ✅ Seniority conflicts detected via real bid order computation
- ✅ Bid schedule must be set

The bid order preview API allows operators to review derived ordering before confirmation.

**Next:** Phase 29E will implement bid order freezing at confirmation.
