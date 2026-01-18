# Phase 26A — Lifecycle-Aware Capability Computation

## Objective

Make backend capability computation **authoritative and lifecycle-aware**.

Currently, `UserCapabilities` are hardcoded in `crates/api/src/capabilities.rs` with `TODO` comments indicating that Phase 26 will implement lifecycle restrictions. Admins currently receive `can_delete: Allowed` and `can_move_area: Allowed` regardless of the bid year's lifecycle state.

This phase makes capabilities **reflect lifecycle state**, ensuring the backend is the sole source of truth for what actions are permitted.

---

## In-Scope

### Backend Changes Only

1. **Update `compute_user_capabilities` function**
   - Accept bid year lifecycle state as a parameter
   - Implement lifecycle-aware rules:
     - `can_delete`: `Denied` after canonicalization (any state >= `Canonicalized`)
     - `can_move_area`: `Denied` after canonicalization (direct assignment requires override)
     - `can_edit_seniority`: Always allowed for admins/bidders (no lifecycle restriction)

2. **Update handler call sites**
   - Thread lifecycle state into capability computation
   - May require fetching lifecycle state from persistence in `list_users` handler

3. **Integration tests**
   - Validate capability computation across all lifecycle states
   - Test transitions (Draft → BootstrapComplete → Canonicalized → etc.)
   - Verify bidder vs admin differences persist

4. **Remove TODO comments**
   - Clean up placeholder comments in `capabilities.rs`

---

## Out-of-Scope

- Frontend changes (Phase 26B will consume these capabilities)
- New lifecycle states
- New capability flags
- Override execution (already exists)
- Role changes
- Area-level capabilities
- Bid-year-level capabilities

---

## Backend Changes

### Files Affected

#### Primary Implementation

**`crates/api/src/capabilities.rs`**

- Update function signature:

  ```rust
  pub fn compute_user_capabilities(
      actor: &AuthenticatedActor,
      actor_operator: &OperatorData,
      lifecycle_state: BidYearLifecycle, // NEW PARAMETER
  ) -> Result<UserCapabilities, &'static str>
  ```

- Implement lifecycle logic:

  ```rust
  let is_canonicalized_or_later = matches!(
      lifecycle_state,
      BidYearLifecycle::Canonicalized
          | BidYearLifecycle::BiddingActive
          | BidYearLifecycle::BiddingClosed
  );

  let can_delete = if is_canonicalized_or_later {
      Capability::Denied
  } else {
      Capability::Allowed
  };

  let can_move_area = if is_canonicalized_or_later {
      Capability::Denied
  } else {
      Capability::Allowed
  };
  ```

- Apply role-based overrides (bidders always denied)

#### Handler Updates

**`crates/api/src/handlers.rs`**

Update `list_users` handler (and any other handlers that compute user capabilities):

- Fetch lifecycle state from persistence
- Parse lifecycle state string to `BidYearLifecycle` enum
- Pass lifecycle state to `compute_user_capabilities`

Example:

```rust
let lifecycle_state_str = persistence.get_lifecycle_state(bid_year_id)?;
let lifecycle_state = lifecycle_state_str.parse::<BidYearLifecycle>()?;

let capabilities = compute_user_capabilities(
    authenticated_actor,
    actor_operator,
    lifecycle_state,
)?;
```

#### Tests

**`crates/api/src/capabilities.rs` (test module)**

Add test cases:

- `test_user_capabilities_draft_state`
- `test_user_capabilities_bootstrap_complete_state`
- `test_user_capabilities_canonicalized_state`
- `test_user_capabilities_bidding_active_state`
- `test_user_capabilities_bidding_closed_state`
- `test_user_capabilities_admin_vs_bidder_across_states`

Validate:

- Draft/BootstrapComplete: `can_delete` and `can_move_area` are `Allowed` for admins
- Canonicalized and later: both are `Denied`
- Bidders always `Denied` for `can_delete` and `can_move_area`
- `can_edit_seniority` unaffected by lifecycle

---

## Domain & UX Invariants

### Rules That Must Not Be Violated

1. **Backend is authoritative**
   - Capabilities must never be computed client-side
   - Frontend must trust backend capabilities

2. **Lifecycle as one-way lock**
   - Canonicalization is irreversible
   - Capabilities reflect this immutability

3. **Override semantics preserved**
   - Denied capabilities do not mean actions are impossible
   - Overrides exist for post-canonicalization changes
   - Capabilities indicate _direct_ action availability

4. **Role hierarchy intact**
   - Bidders remain more restricted than admins
   - Disabled operators have no capabilities (existing behavior)

5. **Seniority editing unrestricted**
   - Seniority data is informational in Phase 26
   - Editing does not affect bidding logic yet
   - No lifecycle restriction needed

### Lifecycle Rules

| Lifecycle State   | can_delete (Admin) | can_move_area (Admin) | can_edit_seniority (Admin) |
| ----------------- | ------------------ | --------------------- | -------------------------- |
| Draft             | Allowed            | Allowed               | Allowed                    |
| BootstrapComplete | Allowed            | Allowed               | Allowed                    |
| Canonicalized     | **Denied**         | **Denied**            | Allowed                    |
| BiddingActive     | **Denied**         | **Denied**            | Allowed                    |
| BiddingClosed     | **Denied**         | **Denied**            | Allowed                    |

Bidders: `can_delete` and `can_move_area` always `Denied`, regardless of lifecycle.

---

## Risks & Ambiguities

### 1. Performance Impact

**Risk**: Fetching lifecycle state in `list_users` handler adds a database query.

**Mitigation**:

- Lifecycle state is small (single string)
- Already fetched in many handlers
- Could be cached in request context if needed (future optimization)

**Decision**: Accept the query. Correctness over premature optimization.

---

### 2. Multi-Bid-Year Scenarios

**Ambiguity**: What if a handler needs capabilities for users across multiple bid years?

**Resolution**: Phase 26 assumes single active bid year. `list_users` is scoped to one bid year. Not a concern.

---

### 3. Capability Propagation

**Risk**: If new handlers are added that return `UserInfo`, they must also compute lifecycle-aware capabilities.

**Mitigation**: Document requirement clearly. Consider adding integration test that validates all `UserInfo` responses.

**Decision**: Document in code comments. Not blocking for Phase 26A.

---

### 4. Error Handling for Lifecycle Parse

**Risk**: Lifecycle state string may fail to parse.

**Mitigation**: Use existing `translate_domain_error` pattern. Lifecycle state is controlled by backend, parse failure indicates data corruption.

**Decision**: Return `ApiError::Internal` on parse failure.

---

### 5. No Active Bid Year

**Risk**: If no active bid year exists, how should capabilities behave?

**Resolution**:

- `list_users` requires a bid year ID
- If no active bid year, handler returns error before computing capabilities
- Not a Phase 26A concern

---

## Exit Criteria

Phase 26A is complete when:

1. ✅ `compute_user_capabilities` accepts `lifecycle_state` parameter
2. ✅ Lifecycle rules implemented correctly (see table above)
3. ✅ `list_users` handler passes lifecycle state to capability computation
4. ✅ All existing capability tests still pass
5. ✅ New lifecycle-aware capability tests added and passing
6. ✅ `TODO` comments removed from `capabilities.rs`
7. ✅ `cargo xtask ci` passes
8. ✅ `pre-commit run --all-files` passes
9. ✅ API responses include lifecycle-aware capabilities
10. ✅ No frontend changes made (Phase 26B will consume)

---

## Implementation Notes

### Suggested Implementation Order

1. **Add lifecycle parameter** to `compute_user_capabilities`
2. **Update test helpers** to pass a default lifecycle state
3. **Implement lifecycle logic** for admins
4. **Add lifecycle-specific test cases**
5. **Update `list_users` handler** to fetch and pass lifecycle state
6. **Run integration tests** to validate end-to-end behavior
7. **Remove TODO comments**
8. **Verify CI passes**

### Code Location Reference

- **Capability computation**: `crates/api/src/capabilities.rs`
- **Handler integration**: `crates/api/src/handlers.rs` (`list_users` function)
- **Lifecycle enum**: `crates/domain/src/lifecycle.rs` or similar
- **Persistence**: `crates/persistence/src/lib.rs` (`get_lifecycle_state`)

### Type Safety Note

Use the `BidYearLifecycle` enum from domain, not raw strings. Parse once in handler, pass typed value to capability computation.

---

## Testing Strategy

### Unit Tests (in `capabilities.rs`)

Test capability computation for each lifecycle state:

```rust
#[test]
fn test_admin_can_delete_in_draft() {
    let actor = create_test_admin();
    let operator = create_operator_data(1, "admin", "Admin", false);
    let lifecycle = BidYearLifecycle::Draft;

    let caps = compute_user_capabilities(&actor, &operator, lifecycle).unwrap();

    assert!(caps.can_delete.is_allowed());
    assert!(caps.can_move_area.is_allowed());
}

#[test]
fn test_admin_cannot_delete_after_canonicalized() {
    let actor = create_test_admin();
    let operator = create_operator_data(1, "admin", "Admin", false);
    let lifecycle = BidYearLifecycle::Canonicalized;

    let caps = compute_user_capabilities(&actor, &operator, lifecycle).unwrap();

    assert!(!caps.can_delete.is_allowed());
    assert!(!caps.can_move_area.is_allowed());
    assert!(caps.can_edit_seniority.is_allowed()); // Still allowed
}
```

### Integration Tests (optional, in handler tests or server tests)

Validate that `list_users` API response includes correct capabilities:

1. Create bid year in Draft state
2. Call `list_users`, verify `can_delete: true`
3. Transition to Canonicalized
4. Call `list_users` again, verify `can_delete: false`

---

## Dependencies

### Required Existing Code

- `BidYearLifecycle` enum (from Phase 25A)
- `Capability` enum (`Allowed` / `Denied`)
- `UserCapabilities` struct
- `get_lifecycle_state` persistence method
- `list_users` handler

### No New Dependencies

This phase uses only existing infrastructure.

---

## Rollout Considerations

### Backward Compatibility

**API Contract Change**: `UserInfo` responses will have different capability values post-deployment.

**Impact**: Frontend currently does not consume capabilities (verified in Phase 25E assessment). No breaking change.

**Frontend Behavior**: Phase 26B will add UI gating. Until then, backend enforcement is sufficient.

### Rollback Plan

If Phase 26A needs to be reverted:

1. Restore `compute_user_capabilities` to always return `Allowed` for admins
2. Remove lifecycle parameter
3. Restore TODO comments

No data migration needed (capabilities are computed, not stored).

---

## Non-Goals

- Changing override semantics
- Adding new capabilities
- Gating API endpoints (backend already enforces lifecycle rules)
- Frontend work
- Audit log changes
- Performance optimization

---

## Validation Checklist

Before marking Phase 26A complete, verify:

- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (all tests)
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes
- [ ] Manual API test: `list_users` in Draft state shows `can_delete: true`
- [ ] Manual API test: `list_users` in Canonicalized state shows `can_delete: false`
- [ ] No frontend changes introduced
- [ ] Code review confirms lifecycle logic matches table above
- [ ] Documentation comments updated (if needed)

---

## Next Phase

**Phase 26B** will consume these lifecycle-aware capabilities to gate UI actions, add override workflows, and make user editing lifecycle-honest.
