# Phase 29 Working State

## Phase

- Phase: 29
- Title: Pre-Bid Readiness, Ordering, and Bootstrap Finalization

## Current Status

- Status: In Progress
- Last Updated: 2026-01-19
- Reason: Sub-Phase 29A implementation complete, pending unit tests and route wiring

## Active Sub-Phase

- Sub-Phase: 29A — User Participation Flags
- State: Nearly Complete (tests and route wiring remaining)

## Completed Sub-Phases

- [x] Planning Pass — Sub-phase documents created

## Planned Sub-Phases

- [ ] 29A — User Participation Flags (implementation complete, tests pending)
- [ ] 29B — Round Groups and Rounds
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

## Outstanding Work

### Sub-Phase 29A Remaining Work

1. **Unit Tests (Required)**:
   - Write unit tests for `User::validate_participation_flags()`:
     - Test valid: both false
     - Test valid: both true
     - Test valid: excluded_from_bidding=true, excluded_from_leave_calculation=false
     - Test invalid: excluded_from_bidding=false, excluded_from_leave_calculation=true (must error)
   - Write tests for `UpdateUserParticipation` command in `core/src/tests/apply_tests.rs`:
     - Test successful update
     - Test invariant violation
     - Test user not found
     - Test state mutation is correct

2. **Integration Tests (Required)**:
   - Write API integration test for `update_user_participation` handler:
     - Test successful flag update
     - Test invariant violation returns error
     - Test lifecycle constraint (Draft/BootstrapComplete only)
     - Test user not found returns error
     - Test audit event is created

3. **Route Wiring (Required)**:
   - Wire up `POST /api/users/{user_id}/participation` route
   - Remove `#[allow(dead_code)]` attributes
   - Export handler from lib if needed
   - Test via HTTP endpoint

4. **Documentation (Optional but Recommended)**:
   - Update API documentation with new endpoint
   - Document participation flag semantics
   - Add examples to handler documentation

### Future Sub-Phases

- Execute Sub-Phase 29B (Round Groups and Rounds)
- Execute Sub-Phase 29C (Bid Schedule Declaration)
- Execute Sub-Phase 29D (Readiness Evaluation)
- Execute Sub-Phase 29E (Confirmation and Bid Order Freezing)
- Execute Sub-Phase 29F (Bid Status Tracking Structure)
- Execute Sub-Phase 29G (Post-Confirmation Bid Order Adjustments)
- Execute Sub-Phase 29H (Docker Compose Deployment)

## Known Failures / Breakages

None - all compilation errors resolved, all tests passing, CI passing.

## Stop-and-Ask Items

None

## Resume Instructions

### Immediate Next Steps

1. **Write Unit Tests**:
   - Create `test_validate_participation_flags_*` tests in `domain/src/tests/validation.rs`
   - Create `test_update_user_participation_*` tests in `core/src/tests/apply_tests.rs`
   - Run `cargo test --lib` to verify

2. **Write Integration Tests**:
   - Create test file or add to existing integration tests in `api/src/tests/`
   - Test all success and failure paths
   - Run `cargo test --lib` to verify

3. **Wire Routes** (if needed):
   - Add route definition in appropriate router file
   - Test via HTTP client
   - Remove `#[allow(dead_code)]` attributes

4. **Final Validation**:
   - Run `cargo xtask ci`
   - Run `pre-commit run --all-files`
   - Ensure all tests pass
   - Update Phase 29A checklist in `plans/PHASE_29/PHASE_29A.md`

5. **Mark Sub-Phase 29A Complete**:
   - Check off all items in `plans/PHASE_29/PHASE_29A.md`
   - Update this working state to mark 29A as complete
   - Commit all changes

6. **Move to Sub-Phase 29B**:
   - Only after 29A is fully complete with all tests passing

### Completion Criteria for 29A

- [x] Database schema migrations created (both SQLite and MySQL)
- [x] Schema parity verified
- [x] Domain types updated
- [x] Directional invariant enforced in domain
- [x] Persistence layer supports new fields
- [x] API endpoint implemented
- [x] API response types updated
- [x] Lifecycle constraints enforced
- [ ] Unit tests for invariant enforcement (PENDING)
- [ ] Integration tests for API endpoint (PENDING)
- [ ] Routes wired up (PENDING)
- [x] `cargo xtask ci` passes
- [x] `pre-commit run --all-files` passes

### Reference Documents

- Sub-phase checklist: `plans/PHASE_29/PHASE_29A.md`
- Architectural rules: `AGENTS.md`
- Execution protocol: `plans/PHASE_EXECUTION.md`
