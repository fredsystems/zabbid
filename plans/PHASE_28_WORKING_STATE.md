# Phase 28 Working State

## Phase

- Phase: 28
- Title: Canonical User Identity Enforcement

## Current Status

- Status: In Progress
- Last Updated: 2026-01-20
- Reason: Phase 28A complete, starting Phase 28B — Make Commands Carry Canonical user_id

## Active Sub-Phase

- Sub-Phase: 28B — Make Commands Carry Canonical user_id
- State: Not Started

## Completed Sub-Phases

- [x] Phase 28A — Remove Identity Reconstruction Helpers & Patterns

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

## Outstanding Work

- Execute Phase 28B (make commands carry canonical user_id)
- Execute Phase 28C (fix No-Bid area exclusion in completeness logic)
- Execute Phase 28D (test hardening & validation)

## Known Failures / Breakages

None.

## Stop-and-Ask Items

None.

## Resume Instructions

1. Read PHASE_28B_COMMAND_IDENTITY.md
2. Update `Command::UpdateUser` to include `user_id`
3. Update `Command::OverrideAreaAssignment` to include `user_id`
4. Update `Command::OverrideEligibility` to include `user_id`
5. Update `Command::OverrideBidOrder` to include `user_id`
6. Update `Command::OverrideBidWindow` to include `user_id`
7. Update `core/apply.rs` to use `user_id` from commands
8. Update API handlers to pass `user_id` into commands
9. Ensure audit events reference users by `user_id` only
10. Run tests after each change
11. Update this document before pausing or completing Phase 28B
