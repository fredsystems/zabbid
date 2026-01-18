# Phase 27B — User Identity Correctness Verification

## Purpose

Verify that user operations use `user_id` exclusively and initials are display-only metadata.

## Scope

### Analysis Tasks

- Grep for `initials` in `crates/persistence/src/`
- Grep for `.find_by_initials` or similar methods
- Audit all functions in `crates/api/src/handlers/` that accept user identifiers
- Review `crates/core/src/` for state transitions using initials
- Check frontend forms/lookups in `ui/src/components/admin/users/`
- Review database schema for foreign key usage
- Verify audit log entries use `user_id` not initials

### Verification Targets

- All database queries use `user_id` not initials
- All API endpoints accept `user_id` not initials
- All state transition functions keyed by `user_id`
- Frontend displays initials but submits `user_id`
- Foreign keys reference `user_id` not initials
- Audit events record `user_id` as the canonical identifier

### Test Additions

- Regression test: user lookup by initials fails or is unavailable
- Regression test: initials can be changed without breaking references
- Test that verifies duplicate initials can exist (if policy allows)
- Test that user operations succeed when initials are modified

## Explicit Non-Goals

- Do NOT audit areas or bid years (separate phase 27C)
- Do NOT implement new identity mechanisms
- Do NOT change how initials are displayed in UI
- Do NOT alter CSV import behavior
- Do NOT refactor user model structure

## Files Likely to Be Affected

### Backend

- `crates/persistence/src/queries/users.rs`
- `crates/persistence/src/canonical/users.rs`
- `crates/api/src/handlers/users.rs`
- `crates/core/src/commands/` (user-related commands)
- `crates/domain/src/user.rs`

### Frontend

- `ui/src/components/admin/users/`
- Any forms that submit user data

### Tests

- `crates/api/tests/` (user endpoint tests)
- `crates/core/tests/` (user state transition tests)
- `crates/persistence/tests/` (user query tests)

## Search Patterns

Execute the following searches to identify potential violations:

```bash
# Find all uses of initials in persistence layer
grep -rn "initials" crates/persistence/src/ --include="*.rs"

# Find methods that might look up by initials
grep -rn "find.*initials\|get.*initials\|lookup.*initials" --include="*.rs"

# Find API handlers accepting user identifiers
grep -rn "Path\|Query\|Json" crates/api/src/handlers/users.rs

# Find state transitions referencing users
grep -rn "user_id\|initials" crates/core/src/ --include="*.rs"
```

## Completion Conditions

- Zero uses of initials for lookup, mutation, or foreign keys
- All new regression tests pass
- Documentation comment added to `Initials` type warning against identity usage
- All existing tests still pass
- Git commit focused solely on user identity verification

## Dependencies

- Phase 27A must be complete (requires clear identity rules documented)

## Blocks

None (parallel work allowed with 27C after 27A)

## Execution Notes

This phase may require minimal code changes if violations are found:

- Acceptable changes: Adding tests, adding documentation comments
- Acceptable changes: Fixing actual identity misuse (if found)
- Not acceptable: Refactoring for style or convenience
- Not acceptable: Changing domain models without user approval

If significant identity violations are discovered that require architectural changes, stop and report findings rather than attempting fixes.

Focus on verification first, fixes second.
