# Phase 28B — Make Commands Carry Canonical user_id

## Purpose

Ensure all user-targeting commands carry `user_id` explicitly as canonical identity, not just initials as "domain vocabulary."

This sub-phase enforces the architectural invariant:

> **Commands must carry identity explicitly**
>
> **Audit events must reference users by `user_id` only; initials may appear only as contextual metadata, never as identity or correlation keys.**

---

## Scope

### Files to Modify

#### Core Layer

- `crates/core/src/command.rs`
  - **Update:** `Command::UpdateUser` — add `user_id: i64` field
  - **Update:** `Command::OverrideAreaAssignment` — add `user_id: i64` field
  - **Update:** `Command::OverrideEligibility` — add `user_id: i64` field
  - **Update:** `Command::OverrideBidOrder` — add `user_id: i64` field
  - **Update:** `Command::OverrideBidWindow` — add `user_id: i64` field
  - Update command documentation to reflect explicit identity requirement

- `crates/core/src/apply.rs`
  - **Update:** Command pattern matching to extract and use `user_id`
  - **Update:** State mutation logic to operate on `user_id`, not initials
  - Ensure no command handler reconstructs identity from initials

#### API Layer

- `crates/api/src/handlers.rs`
  - **Update:** `update_user()` — pass `request.user_id` into Command
  - **Update:** `override_area_assignment()` — pass `request.user_id` into Command
  - **Update:** `override_eligibility()` — pass `request.user_id` into Command
  - **Update:** `override_bid_order()` — pass `request.user_id` into Command
  - **Update:** `override_bid_window()` — pass `request.user_id` into Command

#### Tests

- `crates/core/src/tests/*.rs`
  - Update all command construction to include `user_id`
  - Ensure test commands do not use sentinel or fake `user_id` values
  - Add tests verifying commands fail without explicit `user_id`

- `crates/api/src/tests/api_tests.rs`
  - Update API test commands to include `user_id`
  - Ensure integration tests validate `user_id` presence

---

## Invariants Being Enforced

1. **Explicit Identity:** All user-targeting commands MUST include `user_id: i64`
2. **No Inference:** Commands MUST NOT allow identity to be derived from initials
3. **No Optional Identity:** `user_id` is required, not `Option<i64>`
4. **No Fallback Resolution:** If `user_id` is not present, operations must fail immediately with no fallback to initials-based lookup
5. **Initials Are Metadata:** Initials remain in commands for audit/display purposes only, never for selection
6. **Audit Identity Invariant:** Audit events emitted by command handlers MUST reference users by `user_id` only; initials may appear as contextual metadata but never as identity or correlation keys

---

## Design Decision: Keep Initials or Remove?

### Option A: Keep Initials as Metadata

**Pattern:**

```rust
Command::UpdateUser {
    user_id: i64,           // Required, canonical
    initials: Initials,     // Mutable metadata
    name: String,
    // ...
}

```

**Rationale:**

- Initials are part of the update payload
- Audit events benefit from denormalized display values
- Core layer remains UI-agnostic (doesn't query persistence for labels)

### Option B: Remove Initials Entirely

**Pattern:**

```rust
Command::UpdateUser {
    user_id: i64,           // Required, canonical
    name: String,
    // ... (no initials field)
}

```

**Rationale:**

- Eliminates any temptation to use initials for lookup
- Forces audit layer to denormalize from canonical state
- Stricter architectural enforcement

### **Recommended: Option A (Keep Initials as Metadata)**

**Justification:**

- UpdateUser explicitly changes initials as part of the mutation
- Audit events should record what was requested, not just the canonical ID
- Removing initials from UpdateUser makes the command semantically incomplete
- Override commands can keep initials for audit clarity without risk

**Enforcement:**

- Core layer MUST use `user_id` for all state lookups
- Core layer MUST NOT search state by initials
- Initials are written to audit events as metadata only
- Audit events MUST reference users by `user_id` for identity/correlation
- No fallback resolution from initials to `user_id` is permitted

---

## Explicit Non-Goals

- **Do NOT** change persistence layer queries (already use `user_id`)
- **Do NOT** change API request structures (already include `user_id`)
- **Do NOT** refactor State representation (out of scope)
- **Do NOT** fix No-Bid area logic (that's Phase 28C)

---

## Implementation Strategy

### Step 1: Update Command Definitions

**Before:**

```rust
UpdateUser {
    initials: Initials,
    name: String,
    area: Area,
    // ...
}

```

**After:**

```rust
UpdateUser {
    user_id: i64,          // NEW: explicit canonical identity
    initials: Initials,    // Kept for mutation semantics
    name: String,
    area: Area,
    // ...
}

```

**Repeat for:**

- `OverrideAreaAssignment`
- `OverrideEligibility`
- `OverrideBidOrder`
- `OverrideBidWindow`

---

### Step 2: Update core/apply.rs Command Handlers

**Current Pattern (Hypothetical Violation):**

```rust
Command::UpdateUser { initials, name, area, .. } => {
    let user = state.users.iter_mut()
        .find(|u| u.initials == initials)  // ❌ Identity reconstruction
        .ok_or(...)?;
    user.name = name;
}

```

**Correct Pattern:**

```rust
Command::UpdateUser { user_id, initials, name, area, .. } => {
    let user = state.users.iter_mut()
        .find(|u| u.user_id == Some(user_id))  // ✅ Canonical identity
        .ok_or(...)?;
    user.initials = initials;  // Update mutable metadata
    user.name = name;
}

```

**Enforcement:**

- All command handlers MUST pattern-match on `user_id`
- All state searches MUST use `user_id`, never `initials`
- Add defensive assertions if needed (e.g., `debug_assert!(user.user_id == Some(user_id))`)

---

### Step 3: Update API Layer Command Construction

**Current:**

```rust
let command = Command::UpdateUser {
    initials: Initials::new(&request.initials),
    name: request.name.clone(),
    // ...
};

```

**Updated:**

```rust
let command = Command::UpdateUser {
    user_id: request.user_id,  // NEW: pass explicit identity
    initials: Initials::new(&request.initials),
    name: request.name.clone(),
    // ...
};

```

**Affected Functions:**

- `update_user()`
- All `override_*()` handlers (already have `request.user_id` available)

---

### Step 4: Update Tests

**Pattern:**
All test command construction must include valid `user_id`:

```rust
let cmd = Command::UpdateUser {
    user_id: 42,  // Use actual canonical ID from test setup
    initials: Initials::new("ABC"),
    // ...
};

```

**Prohibited Patterns:**

- Sentinel IDs: `user_id: -1`, `user_id: 0`
- Placeholder IDs without corresponding canonical records
- Relying on initials for correlation in assertions

---

## Completion Conditions

Phase 28B is complete when:

- ✅ All user-targeting commands include `user_id: i64` field
- ✅ `core/apply.rs` uses `user_id` for all state lookups
- ✅ No command handler searches state by `initials`
- ✅ No fallback resolution from initials to `user_id` exists
- ✅ Audit events reference users by `user_id` only (initials as metadata only)
- ✅ API handlers pass `request.user_id` into commands
- ✅ All tests construct commands with explicit `user_id`
- ✅ Command documentation reflects explicit identity requirement
- ✅ `grep -rn "\.initials ==" crates/core/src/apply.rs` returns zero matches
- ✅ All tests pass
- ✅ `cargo xtask ci` passes
- ✅ `pre-commit run --all-files` passes

---

## Expected Risks & Edge Cases

### Risk: Large Cascade of Changes

**Likelihood:** High

**Impact:**

- Command enum changes propagate to all command construction sites
- Every test that builds these commands must be updated
- Pattern matching in `apply.rs` must be updated

**Mitigation:**

- Make changes incrementally (one command at a time if needed)
- Use compiler errors as a checklist
- Ensure each command compiles before moving to the next

---

### Risk: State Lookup Failures

**Likelihood:** Medium

**Concern:** If `state.users` doesn't contain a user with the given `user_id`, lookups will fail.

**Mitigation:**

- Ensure state is properly loaded before command application
- Add clear error messages for missing canonical records
- Tests must set up canonical state with matching `user_id` values

---

### Risk: Breaking Existing Command Tests

**Likelihood:** High

**Mitigation:**

- Update test fixtures to include `user_id`
- Use realistic `user_id` values from canonical setup
- Avoid placeholder or sentinel IDs

---

## Stop-and-Ask Conditions

If any of the following occur, stop and request guidance:

- Core layer requires identity reconstruction for correctness
- State representation doesn't include `user_id` consistently
- Command semantics conflict with explicit identity requirement
- More than 50 test sites require modification
- Persistence layer changes are needed (should already be complete)
- Removing initials from commands breaks audit trail integrity

---

## Validation Checklist

Before marking Phase 28B complete:

- [ ] `Command::UpdateUser` includes `user_id: i64`
- [ ] `Command::OverrideAreaAssignment` includes `user_id: i64`
- [ ] `Command::OverrideEligibility` includes `user_id: i64`
- [ ] `Command::OverrideBidOrder` includes `user_id: i64`
- [ ] `Command::OverrideBidWindow` includes `user_id: i64`
- [ ] `apply.rs` uses `user_id` for all user lookups
- [ ] `grep -rn "find.*initials ==" crates/core/` returns zero matches in apply.rs
- [ ] No fallback resolution from initials to `user_id` exists in any layer
- [ ] Audit events reference users by `user_id` only
- [ ] API handlers pass `request.user_id` to commands
- [ ] All tests construct commands with valid `user_id`
- [ ] CI passes: `cargo xtask ci`
- [ ] Linting passes: `pre-commit run --all-files`
- [ ] No new clippy warnings introduced

---

## References

- **Authoritative Spec:** `plans/PHASE_28.md`
- **Identity Rules:** `AGENTS.md` § Canonical Identity Enforcement
- **Command Architecture:** `crates/core/src/command.rs` module documentation
- **Related Phases:** Phase 28A (removed reconstruction helpers)
