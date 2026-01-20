# Phase 29 Working State

## Phase

- Phase: 29
- Title: Pre-Bid Readiness, Ordering, and Bootstrap Finalization

## Current Status

- Status: In Progress
- Last Updated: 2026-01-19
- Reason: Sub-Phase 29A complete, ready to begin 29B

## Active Sub-Phase

- Sub-Phase: 29B — Round Groups and Rounds
- State: In Progress

## Completed Sub-Phases

- [x] Planning Pass — Sub-phase documents created
- [x] 29A — User Participation Flags

## Planned Sub-Phases

- [x] 29A — User Participation Flags (COMPLETE)
- [ ] 29B — Round Groups and Rounds (IN PROGRESS)
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

### Sub-Phase 29B (In Progress)

#### Completed

- [x] Database schema migrations created (SQLite and MySQL)
- [x] Schema parity verified (`cargo xtask verify-migrations`)
- [x] Domain types created (`RoundGroup` and `Round`)
- [x] Domain error variants added
- [x] Domain validation methods implemented
- [x] API error translations added
- [x] Code compiles without errors
- [x] All existing tests pass
- [x] Clippy passes
- [x] Pre-commit hooks pass

#### Remaining Work

- [ ] Persistence layer CRUD operations for round groups
- [ ] Persistence layer CRUD operations for rounds
- [ ] Core layer commands for round group management
- [ ] Core layer commands for round management
- [ ] API endpoints for round groups (POST, GET, PATCH, DELETE)
- [ ] API endpoints for rounds (POST, GET, PATCH, DELETE)
- [ ] System area constraint enforcement in API layer
- [ ] Lifecycle constraint enforcement in API layer
- [ ] Unit tests for domain validation
- [ ] Integration tests for API endpoints
- [ ] Constraint tests (system area rejection, unique round numbers, etc.)

## Outstanding Work

### Future Sub-Phases

- Complete Sub-Phase 29B (Round Groups and Rounds)
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

### Immediate Next Steps for Sub-Phase 29B

1. **Persistence Layer**:
   - Add queries for round groups (list, get, insert, update, delete)
   - Add queries for rounds (list, get, insert, update, delete)
   - Add query to check if round group is in use before deletion
   - Test queries compile and work with existing database

2. **Core Layer**:
   - Add `CreateRoundGroup`, `UpdateRoundGroup`, `DeleteRoundGroup` commands
   - Add `CreateRound`, `UpdateRound`, `DeleteRound` commands
   - Implement command handlers with lifecycle constraint checks
   - Validate system area constraint for rounds
   - Generate appropriate audit events

3. **API Layer**:
   - Implement round group endpoints (POST, GET, PATCH, DELETE)
   - Implement round endpoints (POST, GET, PATCH, DELETE)
   - Add request/response types
   - Enforce lifecycle constraints (Draft/BootstrapComplete only)
   - Return appropriate error responses

4. **Testing**:
   - Write domain validation tests
   - Write core command tests
   - Write API integration tests
   - Test system area rejection
   - Test unique constraints (round group name, round number)
   - Test lifecycle constraints

5. **Final Validation**:
   - Run `cargo test --lib`
   - Run `cargo xtask ci`
   - Run `pre-commit run --all-files`
   - Update Phase 29B checklist in `plans/PHASE_29/PHASE_29B.md`
   - Update this working state document
   - Commit all changes

6. **Move to Sub-Phase 29C**:
   - Only after 29B is fully complete with all tests passing

### Completion Criteria for 29B

- [x] Database schema migrations created (both SQLite and MySQL)
- [x] Schema parity verified
- [x] Domain types created
- [x] Domain error variants added
- [x] API error translations added
- [ ] Persistence layer CRUD operations
- [ ] Core layer commands implemented
- [ ] API endpoints implemented
- [ ] System area constraint enforced
- [ ] Lifecycle constraints enforced
- [ ] Unit tests for domain types
- [ ] Integration tests for API endpoints
- [ ] Constraint tests (system area, unique round numbers)
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes

### Reference Documents

- Sub-phase checklist: `plans/PHASE_29/PHASE_29A.md`
- Next sub-phase: `plans/PHASE_29/PHASE_29B.md`
- Architectural rules: `AGENTS.md`
- Execution protocol: `plans/PHASE_EXECUTION.md`
