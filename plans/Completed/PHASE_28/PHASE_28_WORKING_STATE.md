# Phase 28 Working State

## Phase

- Phase: 28
- Title: Canonical User Identity Enforcement

## Current Status

- Status: Complete
- Last Updated: 2026-01-20
- Reason: Phase 28D complete — All Phase 28 sub-phases finished

## Active Sub-Phase

- Sub-Phase: None
- State: Phase 28 Complete

## Completed Sub-Phases

- [x] Phase 28A — Remove Identity Reconstruction Helpers & Patterns
- [x] Phase 28B — Make Commands Carry Canonical user_id
- [x] Phase 28C — Fix No-Bid Area Exclusion in Completeness Logic
- [x] Phase 28D — Test Hardening & Validation

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

### Phase 28C (Complete)

- Updated `get_actual_area_count()` to filter `is_system_area = 0` (flag-based exclusion)
- System areas (identified by `is_system_area = 1`) now excluded from actual area count
- Added regression test `test_actual_area_count_excludes_system_areas`
- Test verifies system area (NO BID) does not count toward expected area totals
- Filter uses flag-based identification, not area code or name string matching
- All tests passing (125 persistence tests including new test)
- `cargo xtask ci` passing
- `pre-commit run --all-files` passing
- Committed as: "Phase 28C — Fix No-Bid area exclusion in completeness logic"

### Phase 28D (Complete)

- Added compile-time validation tests in `crates/core/src/tests/command_identity_tests.rs`:
  - `test_update_user_command_has_user_id_field` — validates `UpdateUser` includes `user_id`
  - `test_override_commands_have_user_id_field` — validates all override commands include `user_id`
  - `test_no_initials_based_lookup_helpers_compile_time_validation` — validates no initials-based lookups exist
- Added API integration tests in `crates/api/src/tests/api_tests.rs`:
  - `test_register_user_creates_user_with_user_id` — validates registration creates user with `user_id` populated
  - `test_update_user_uses_user_id_from_request` — validates `UpdateUser` targets by `user_id`
- Core tests validate compile-time invariants (type system enforcement)
- API tests validate runtime behavior (integration with persistence)
- Existing tests already cover:
  - Duplicate initials across areas (`test_duplicate_initials_allowed_across_areas`)
  - User updates preserve canonical ID (`test_user_updates_preserve_canonical_id`)
  - Area counting excludes system areas (Phase 28C regression test)
- All tests passing (185 total tests)
- `cargo xtask ci` passing
- `pre-commit run --all-files` passing

## Outstanding Work

None. Phase 28 is complete.

## Known Failures / Breakages

None.

## Stop-and-Ask Items

None.

## Phase 28 Success Criteria

All success criteria met:

- ✅ No code paths translate initials → `user_id` or fallback-resolve identity
- ✅ All user-targeting commands carry `user_id` explicitly
- ✅ Audit events reference users by `user_id` only
- ✅ System-designated areas are excluded from expected area counts (flag-based)
- ✅ All invariants are validated by passing tests
- ✅ `cargo xtask ci` passes
- ✅ `pre-commit run --all-files` passes

## Resume Instructions

Phase 28 is complete. No further work required for this phase.
