# Phase 28 Working State

## Phase

- Phase: 28
- Title: Canonical User Identity Enforcement

## Current Status

- Status: In Progress
- Last Updated: 2026-01-20
- Reason: Phase 28A and 28B complete, ready for Phase 28C — No-Bid Area Exclusion

## Active Sub-Phase

- Sub-Phase: 28C — No-Bid Area Exclusion
- State: Not Started

## Completed Sub-Phases

- [x] Phase 28A — Remove Identity Reconstruction Helpers & Patterns
- [x] Phase 28B — Make Commands Carry Canonical user_id

## Work Completed

### Phase 28A (Complete)

- Removed `Persistence::get_user_id()` method (violates canonical identity invariant)
- Removed `extract_user_id_from_state()` from server layer
- Modified `insert_new_user_sqlite()` and `insert_new_user_mysql()` to return `user_id` via `last_insert_rowid()` / `LAST_INSERT_ID()`
- Created `PersistTransitionResult` struct to hold `event_id` and optional `user_id`
- Updated `persist_transition_sqlite()` and `persist_transition_mysql()` to return `user_id` for `RegisterUser` actions
- Updated `handle_register_user()` to use `user_id` from persist result instead of searching state by initials
- Updated all `persist_transition()` call sites (checkpoint, finalize, rollback) to extract `event_id`
- Updated persistence tests to handle new return type
- All tests passing
- `cargo xtask ci` passing
- `pre-commit run --all-files` passing
- Committed as: "Phase 28A — Remove identity reconstruction helpers"

### Verification

- ✅ `grep -rn "get_user_id" crates/` returns zero matches
- ✅ `grep -rn "extract_user_id" crates/` returns zero matches
- ✅ `RegisterUserResponse` includes `user_id: i64`
- ✅ Registration flow returns `user_id` without intermediate lookup
- ✅ No fallback resolution logic exists

### Phase 28B (Complete)

- Added `user_id: i64` field to `Command::UpdateUser`
- Added `user_id: i64` field to `Command::OverrideAreaAssignment`
- Added `user_id: i64` field to `Command::OverrideEligibility`
- Added `user_id: i64` field to `Command::OverrideBidOrder`
- Added `user_id: i64` field to `Command::OverrideBidWindow`
- Updated `core/apply.rs` to use `user_id` for state lookup instead of searching by initials
- Updated API `update_user()` handler to pass `request.user_id` into command
- Updated command documentation to reflect explicit identity requirement
- Updated audit event to reference `user_id` as primary identifier with initials as metadata only
- Updated test to include `user_id` in `UpdateUser` command construction
- All tests passing
- `cargo xtask ci` passing
- `pre-commit run --all-files` passing
- Committed as: "Phase 28B — Make commands carry canonical user_id"

## Outstanding Work

- Execute Phase 28C (fix No-Bid area exclusion in completeness logic)
- Execute Phase 28D (test hardening & validation)

## Known Failures / Breakages

None.

## Stop-and-Ask Items

None.

## Resume Instructions

1. Read PHASE_28C_NOBID_EXCLUSION.md
2. Review bootstrap completeness logic
3. Identify where No-Bid area is incorrectly counted
4. Fix completeness logic to exclude No-Bid area from counts
5. Update tests to verify No-Bid exclusion
6. Run tests and CI
7. Update this document before pausing or completing Phase 28C
