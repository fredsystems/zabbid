# Phase 29 Working State

## Phase

- Phase: 29
- Title: Pre-Bid Readiness, Ordering, and Bootstrap Finalization

## Current Status

- Status: In Progress
- Last Updated: 2026-01-20
- Reason: Sub-Phase 29B semantic correction in progress - domain model mismatch identified and being corrected

## Active Sub-Phase

- Sub-Phase: 29B — Round Groups and Rounds (SEMANTIC CORRECTION)
- State: In Progress - Correcting fundamental model mismatch

## Completed Sub-Phases

- [x] Planning Pass — Sub-phase documents created
- [x] 29A — User Participation Flags

## Planned Sub-Phases

- [x] 29A — User Participation Flags (COMPLETE)
- [ ] 29B — Round Groups and Rounds (SEMANTIC CORRECTION IN PROGRESS)
- [ ] 29C — Bid Schedule Declaration
- [ ] 29D — Readiness Evaluation
- [ ] 29E — Confirmation and Bid Order Freezing
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

#### Remaining Work — Semantic Correction Phase

- [ ] **Persistence Layer** (CRITICAL - All queries need updating)
  - [ ] Update `list_areas` to populate `round_group_id`
  - [ ] Update all area creation/update queries to handle `round_group_id`
  - [ ] Fix all `Area::with_id()` calls (now requires `round_group_id` parameter)
  - [ ] Remove `area_id` from all round queries
  - [ ] Update round CRUD to work with `round_group_id` only
  - [ ] Update round list queries to query by round_group_id, not area_id

- [ ] **API Layer** (All handlers need rework)
  - [ ] Update `CreateRoundRequest` (remove area_id, only round_group_id)
  - [ ] Update `list_rounds` handler (query by round_group_id, not area_id)
  - [ ] Update all round handlers to work without area_id
  - [ ] Update area response types to include `round_group_id`
  - [ ] Add/update endpoints to assign round groups to areas
  - [ ] Update area creation/update handlers to handle round group assignment

- [ ] **API Tests** (All 16 integration tests need significant rework)
  - [ ] Update tests: rounds created per round group (not per area)
  - [ ] Add tests for area → round group assignment
  - [ ] Add tests that areas can share round groups (reusability)
  - [ ] Update all test helpers to match corrected model

- [ ] **Planning Documents**
  - [ ] Update `PHASE_29.md` to clarify corrected semantics
  - [ ] Update `PHASE_29B.md` with authoritative domain model
  - [ ] Ensure all other `PHASE_29x.md` documents do not encode this improper semantic issue.

- [ ] **Final Validation**
  - [ ] All tests pass
  - [ ] `cargo xtask ci` passes
  - [ ] `pre-commit run --all-files` passes
  - [ ] Commit semantic correction changes

## Outstanding Work

### Current Sub-Phase

- **PRIORITY**: Complete semantic correction for Sub-Phase 29B
  - Persistence layer updates (high complexity)
  - API layer updates (all handlers)
  - API test updates (all 16 tests)
  - Planning document updates

### Future Sub-Phases

- Complete Sub-Phase 29B semantic correction
- Execute Sub-Phase 29C (Bid Schedule Declaration)
- Execute Sub-Phase 29D (Readiness Evaluation)
- Execute Sub-Phase 29E (Confirmation and Bid Order Freezing)
- Execute Sub-Phase 29F (Bid Status Tracking Structure)
- Execute Sub-Phase 29G (Post-Confirmation Bid Order Adjustments)
- Execute Sub-Phase 29H (Docker Compose Deployment)

## Known Failures / Breakages

**SEMANTIC CORRECTION IN PROGRESS**:

- Persistence layer: Queries still use old model (will fail when called)
- API layer: Handlers reference old model (will fail when called)
- API tests: 16 tests reference old model (currently broken, not run)
- Domain layer: ✅ Corrected and tests passing (129 tests)
- Database schema: ✅ Corrected via migrations

## Stop-and-Ask Items

None

## Resume Instructions

### CRITICAL: Complete Semantic Correction Before Proceeding

**DO NOT proceed to Sub-Phase 29C until 29B semantic correction is complete.**

The initial implementation of 29B violated the authoritative domain model. Correction is in progress:

**Corrected Domain Model**:

1. **Round Groups**: Ordered collections of rounds (reusable across areas)
2. **Rounds**: Carry all bidding rules, belong to round groups (NOT areas)
3. **Areas**: Reference exactly one round group (non-system) or none (system)

**Corrections Applied**:

- ✅ Database schema migrations
- ✅ Diesel schema
- ✅ Domain types
- ✅ Domain tests (129 passing)

**Corrections Remaining** (in priority order):

### 1. Persistence Layer Updates (NEXT - High Priority)

- Update `list_areas()` in `queries/canonical.rs` to read `round_group_id` from database
- Fix all `Area::with_id()` calls to include `round_group_id` parameter
- Update area insert/update queries to handle `round_group_id`
- Remove `area_id` parameter from all round queries
- Update `list_rounds()` to query by `round_group_id` instead of `area_id`
- Update round CRUD operations to work without `area_id`

### 2. API Layer Updates

- Update `CreateRoundRequest` to remove `area_id` field (use `round_group_id` only)
- Update `list_rounds` handler signature (query by round_group_id)
- Update area response types to include `round_group_id`
- Update area handlers to manage round group assignment
- Update all round handlers to work with corrected model

### 3. API Tests Rework

- Rewrite all 16 integration tests to match corrected model
- Test rounds created per round group (not per area)
- Test area → round group assignment
- Test round group reusability across areas

### 4. Planning Documents

- Update `PHASE_29.md` with authoritative model
- Update `PHASE_29B.md` with corrected semantics

### 5. Final Validation

- Run `cargo test --lib` (all tests must pass)
- Run `cargo xtask ci`
- Run `pre-commit run --all-files`
- Commit semantic correction
- Mark 29B complete
- THEN proceed to 29C

### Completion Criteria for 29B (Semantic Correction)

**Schema & Domain**:

- [x] Corrective migrations created (SQLite and MySQL)
- [x] Schema parity verified
- [x] Domain types corrected (Area has round_group_id, Round has no area)
- [x] Domain tests updated and passing (129 tests)

**Implementation** (IN PROGRESS):

- [ ] Persistence layer queries updated for corrected model
- [ ] API handlers updated for corrected model
- [ ] API tests rewritten for corrected model
- [ ] Planning documents updated

**Validation**:

- [ ] All tests pass (domain + persistence + API)
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes
- [ ] Semantic correction committed

**Correctness Criteria** (MUST BE TRUE):

- [ ] Rounds belong to round groups (not areas)
- [ ] Areas reference exactly one round group (non-system) or none (system)
- [ ] Round groups are reusable across multiple areas
- [ ] Round number uniqueness is within round group (not within area)

### Reference Documents

- Completed sub-phase: `plans/PHASE_29/PHASE_29B.md`
- Next sub-phase: `plans/PHASE_29/PHASE_29C.md`
- Architectural rules: `AGENTS.md`
- Execution protocol: `plans/PHASE_EXECUTION.md`

---

## PAUSED STATE - 2026-01-20

**Uncommitted Work in Working Directory**:

- Semantic correction Part 1 is complete but UNCOMMITTED
- Changes staged but cannot commit due to persistence layer compilation errors
- This is expected - persistence layer updates are Part 2 (next step)

**What's Staged (Part 1 - Complete)**:

- ✅ Database migrations (corrective)
- ✅ Diesel schema updates
- ✅ Domain type corrections
- ✅ Domain test fixes
- ✅ Working state document update

**What's Broken (Expected)**:

- ❌ Persistence layer still references old schema (area_id in rounds)
- ❌ API handlers still use old model
- ❌ API tests still use old model

**Next Steps on Resume**:

1. Update persistence layer queries (Part 2)
2. Update API handlers
3. Update API tests
4. Commit all semantic corrections together
5. Verify all tests pass
6. Mark 29B complete
