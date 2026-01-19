# Phase 28D — Test Hardening & Validation

## Purpose

Ensure all Phase 28 invariants are covered by comprehensive, explicit tests that validate canonical identity enforcement and No-Bid area exclusion.

This sub-phase provides regression protection for:

1. **Canonical Identity Only** — No initials-based lookup paths remain
2. **Identity Must Be Explicit** — Commands require explicit `user_id` with no fallback resolution
3. **Audit Identity Invariant** — Audit events reference users by `user_id` only
4. **System Area Exclusion** — System-designated areas (identified by `is_system_area = true`) don't block completeness

---

## Scope

### Test Coverage Goals

#### Identity Enforcement Tests

- **No Reconstruction Paths:** Verify no code translates initials → `user_id`
- **No Fallback Resolution:** Verify operations fail immediately if `user_id` is not present
- **Explicit Command Identity:** Verify commands fail without `user_id`
- **Persistence Invariants:** Verify all mutations use canonical IDs
- **Duplicate Initials Handling:** Verify system handles duplicate initials across areas correctly
- **Audit Identity:** Verify audit events reference users by `user_id` only

#### System Area Exclusion Tests

- **Area Count Correctness:** Verify system-designated areas (identified by flag) excluded from actual count
- **Completeness Logic:** Verify system-designated areas don't block readiness
- **Lifecycle Enforcement:** Verify system area assignment rules still apply (cannot assign users to system-designated areas after canonicalization)
- **Flag-Based Identification:** Verify system areas identified by `is_system_area` flag, not name/code matching

---

## Files to Create/Modify

### Core Layer Tests

- `crates/core/src/tests/command_identity_tests.rs` (NEW)
  - Test: Commands require explicit `user_id`
  - Test: Commands fail gracefully if `user_id` not found in state
  - Test: No fallback resolution from initials to `user_id`
  - Test: Initials changes via UpdateUser work correctly
  - Test: Override commands target correct user by `user_id`
  - Test: Audit events emitted by commands reference users by `user_id` only

### Persistence Layer Tests

- `crates/persistence/src/tests/canonical_tests/identity_enforcement.rs` (NEW)
  - Test: No lookup functions remain that accept initials
  - Test: User queries by `user_id` succeed
  - Test: Duplicate initials across areas handled correctly
  - Test: User update by `user_id` modifies correct record
  - Test: Operations fail immediately if `user_id` is not present (no fallback)

- `crates/persistence/src/tests/canonical_tests/area_counting.rs` (NEW or EXTEND)
  - Test: `get_actual_area_count()` excludes system-designated areas (flag-based)
  - Test: Multiple system-designated areas all excluded
  - Test: Zero system-designated areas doesn't break counting
  - Test: System area flag mutation reflected in counts
  - Test: No area code or name string matching used for system area identification

### API Layer Tests

- `crates/api/src/tests/api_tests.rs` (EXTEND)
  - Test: Registration returns `user_id` in response
  - Test: UpdateUser uses `user_id` from request
  - Test: Override operations target correct user
  - Test: Duplicate initials in different areas both updatable
  - Test: Completeness logic with system-designated area present
  - Test: Completeness logic without system-designated area
  - Test: Audit events from API operations reference users by `user_id` only

### Integration Tests

- `crates/api/src/tests/integration/identity_flows.rs` (NEW)
  - Test: End-to-end user lifecycle using only `user_id`
  - Test: Override flow with duplicate initials across areas
  - Test: Bootstrap completeness with system-designated areas
  - Test: Audit continuity for user operations (user_id as correlation key)

---

## Invariants Being Validated

### Identity Invariants

1. **No Reconstruction Helpers Exist**
   - `grep -rn "get_user_id" crates/` returns no persistence helper
   - `grep -rn "extract_user_id" crates/` returns no state lookup helper

2. **No Fallback Resolution**
   - Operations fail immediately if `user_id` is not present
   - No layer attempts to resolve identity from initials as a fallback

3. **Commands Carry Explicit Identity**
   - All user-targeting commands include `user_id: i64`
   - Command construction without `user_id` fails to compile

4. **State Lookups Use Canonical ID**
   - `apply.rs` searches by `user_id`, never `initials`
   - Persistence queries filter by `user_id`, never `initials`

5. **Duplicate Initials Supported**
   - Users with same initials in different areas independently mutable
   - Operations target by `user_id`, not initials

6. **Audit Events Use Canonical Identity**
   - Audit events reference users by `user_id` only
   - Initials may appear as contextual metadata only
   - No audit event uses initials as identity or correlation key

### System Area Exclusion Invariants

1. **System Areas Excluded from Counts**
   - `get_actual_area_count()` returns count without system-designated areas
   - Exclusion is flag-based (`is_system_area = true`), not name-based
   - Expected vs actual comparison excludes system areas

2. **Completeness Logic Correct**
   - System-designated area presence doesn't prevent readiness
   - Expected count matches actual count when system-designated areas exist

3. **Flag-Based Identification**
   - System areas identified exclusively by `is_system_area` flag
   - No area code or name string matching used

4. **System Area Assignment Rules Unchanged**
   - Cannot assign users to system-designated areas after canonicalization (existing rule)
   - Users in system-designated areas block bootstrap completion (existing rule)

---

## Explicit Non-Goals

- **Do NOT** add tests for features not yet implemented
- **Do NOT** test seniority logic (future phase)
- **Do NOT** test bidding workflows (future phase)
- **Do NOT** add UI tests (backend only)
- **Do NOT** test performance or scalability

---

## Test Implementation Strategy

### Pattern 1: Identity Enforcement Tests

#### Example: Command Requires user_id

```rust
#[test]
fn test_update_user_command_requires_user_id() {
    let mut state = setup_test_state();
    let bid_year = BidYear::new(2026);

    // Create user with known user_id
    let user_id = 42;
    state.users.push(User {
        user_id: Some(user_id),
        initials: Initials::new("ABC"),
        name: "Test User".to_string(),
        // ...
    });

    // Command with correct user_id should succeed
    let cmd = Command::UpdateUser {
        user_id,
        initials: Initials::new("XYZ"),
        name: "Updated Name".to_string(),
        // ...
    };

    let result = apply(&metadata, &state, &bid_year, cmd, actor, cause);
    assert!(result.is_ok());

    // Verify initials were updated
    let updated_user = result.unwrap().new_state.users
        .iter()
        .find(|u| u.user_id == Some(user_id))
        .unwrap();
    assert_eq!(updated_user.initials.value(), "XYZ");
}

#[test]
fn test_update_user_fails_if_user_id_not_found() {
    let state = setup_test_state();
    let bid_year = BidYear::new(2026);

    // Command with non-existent user_id should fail
    let cmd = Command::UpdateUser {
        user_id: 999,  // Does not exist
        initials: Initials::new("ABC"),
        // ...
    };

    let result = apply(&metadata, &state, &bid_year, cmd, actor, cause);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CoreError::UserNotFound { .. }));
}

```

---

### Pattern 2: Duplicate Initials Tests

#### Example: Duplicate Initials Across Areas

```rust
#[test]
fn test_duplicate_initials_across_areas_independently_mutable() {
    let mut persistence = setup_test_persistence();

    // Create two areas
    let north_id = create_area(&mut persistence, 2026, "North");
    let south_id = create_area(&mut persistence, 2026, "South");

    // Create users with same initials in different areas
    let north_user_id = create_user(&mut persistence, north_id, "ABC", "North User");
    let south_user_id = create_user(&mut persistence, south_id, "ABC", "South User");

    assert_ne!(north_user_id, south_user_id);

    // Update North user by user_id
    persistence.update_user(
        north_user_id,
        &Initials::new("XYZ"),
        "North Updated",
        // ...
    ).unwrap();

    // Verify only North user changed
    let north_user = persistence.get_user_by_id(north_user_id).unwrap();
    let south_user = persistence.get_user_by_id(south_user_id).unwrap();

    assert_eq!(north_user.initials.value(), "XYZ");
    assert_eq!(south_user.initials.value(), "ABC");
}

```

---

### Pattern 3: No-Bid Exclusion Tests

#### Example: Area Count Excludes System Areas

```rust
#[test]
fn test_actual_area_count_excludes_system_areas() {
    let mut persistence = setup_test_persistence();
    let bid_year = BidYear::new(2026);
    let bid_year_id = persistence.get_bid_year_id(2026).unwrap();

    // Create regular areas
    create_area(&mut persistence, bid_year_id, "North", false);
    create_area(&mut persistence, bid_year_id, "South", false);

    // Create system area
    create_area(&mut persistence, bid_year_id, "NO BID", true);

    let count = persistence.get_actual_area_count(&bid_year).unwrap();

    assert_eq!(count, 2, "System areas must be excluded from count");
}

#[test]
fn test_completeness_with_no_bid_does_not_block() {
    let mut persistence = setup_test_persistence();
    let metadata = setup_test_metadata(&mut persistence);

    let bid_year_id = metadata.bid_years[0].bid_year_id().unwrap();

    // Set expected area count to 2
    persistence.set_expected_area_count(&BidYear::new(2026), 2).unwrap();

    // Create 2 regular + 1 system area
    create_area(&mut persistence, bid_year_id, "North", false);
    create_area(&mut persistence, bid_year_id, "South", false);
    create_area(&mut persistence, bid_year_id, "NO BID", true);

    let completeness = get_bootstrap_completeness(&mut persistence, &metadata).unwrap();

    let bid_year_info = &completeness.bid_years[0];
    assert_eq!(bid_year_info.actual_area_count, 2);
    assert!(bid_year_info.is_complete);
    assert!(bid_year_info.blocking_reasons.is_empty());
}

```

---

### Pattern 4: Registration Flow Tests

#### Example: Registration Returns user_id

```rust
#[test]
fn test_register_user_returns_user_id_in_response() {
    let mut persistence = setup_test_persistence();
    let metadata = setup_test_metadata(&mut persistence);
    let state = setup_test_state(&mut persistence);

    let request = RegisterUserRequest {
        initials: "ABC".to_string(),
        name: "Test User".to_string(),
        // ...
    };

    let result = register_user(
        &mut persistence,
        &metadata,
        &state,
        &request,
        &actor,
        &operator,
        cause,
    ).unwrap();

    // Response MUST include user_id
    assert!(result.response.user_id > 0);

    // Verify user exists with that ID
    let user = persistence.get_user_by_id(result.response.user_id).unwrap();
    assert_eq!(user.initials.value(), "ABC");
}

```

---

## Completion Conditions

Phase 28D is complete when:

- ✅ All identity enforcement tests pass
- ✅ All no-fallback resolution tests pass
- ✅ All duplicate initials tests pass
- ✅ All system area exclusion tests pass (flag-based)
- ✅ All audit identity tests pass
- ✅ All registration flow tests pass
- ✅ Test coverage includes both success and failure cases
- ✅ No test uses initials for user lookup
- ✅ No test relies on identity reconstruction or fallback resolution
- ✅ All audit event tests verify `user_id` as correlation key
- ✅ `cargo test` passes without warnings
- ✅ `cargo xtask ci` passes
- ✅ `pre-commit run --all-files` passes

---

## Expected Risks & Edge Cases

### Risk: Test Fixtures Require Extensive Setup

**Likelihood:** Medium

**Mitigation:**

- Create reusable test helper functions
- Use builder patterns for complex fixtures
- Document fixture setup clearly

---

### Risk: Tests Depend on Phases 28A-C Being Complete

**Likelihood:** High

**Concern:** Tests cannot pass if prior phases have bugs.

**Mitigation:**

- Ensure Phases 28A-C are fully complete before starting 28D
- Use test failures as validation of prior work
- If tests reveal regressions, stop and fix prior phases

---

### Risk: Excessive Test Duplication

**Likelihood:** Low

**Mitigation:**

- Share test helpers across modules
- Use parameterized tests where appropriate
- Focus on behavioral coverage, not exhaustive permutations

---

## Stop-and-Ask Conditions

If any of the following occur, stop and request guidance:

- Tests reveal that Phases 28A-C are incomplete
- Identity reconstruction paths still exist in production code
- No-Bid exclusion logic doesn't work as expected
- Test coverage cannot be achieved without modifying production code
- More than 20 new test cases are required (excessive scope)
- Backend-specific test behavior creates irreconcilable differences

---

## Validation Checklist

Before marking Phase 28D complete:

- [ ] Test added: Commands require explicit `user_id`
- [ ] Test added: Commands fail if `user_id` not found
- [ ] Test added: No fallback resolution from initials to `user_id`
- [ ] Test added: Operations fail immediately if `user_id` missing
- [ ] Test added: Duplicate initials independently mutable
- [ ] Test added: No reconstruction helpers exist
- [ ] Test added: Registration returns `user_id`
- [ ] Test added: Area count excludes system-designated areas (flag-based)
- [ ] Test added: Completeness with system-designated area doesn't block
- [ ] Test added: Override operations use `user_id`
- [ ] Test added: Audit events reference users by `user_id` only
- [ ] Test added: No area code/name string matching for system areas
- [ ] All new tests pass
- [ ] No test uses initials for lookup
- [ ] No test relies on fallback resolution
- [ ] Test coverage report shows increased coverage
- [ ] CI passes: `cargo test`
- [ ] CI passes: `cargo xtask ci`
- [ ] Linting passes: `pre-commit run --all-files`
- [ ] No new clippy warnings introduced

---

## References

- **Authoritative Spec:** `plans/PHASE_28.md`
- **Identity Rules:** `AGENTS.md` § Canonical Identity Enforcement
- **Test Patterns:** `AGENTS.md` § Testing
- **Related Phases:**
  - Phase 28A (removed reconstruction helpers)
  - Phase 28B (explicit command identity)
  - Phase 28C (No-Bid exclusion)
