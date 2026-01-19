# Phase 28A — Remove Identity Reconstruction Helpers & Patterns

## Purpose

Eliminate all code paths that translate `initials` → `user_id` or reconstruct canonical identity from mutable display fields.

This sub-phase enforces the architectural invariant:

> **No layer may reconstruct, infer, or fallback-resolve `user_id` from initials if `user_id` is missing.**

If `user_id` is not present, the operation must fail immediately with no fallback resolution.

---

## Scope

### Files to Modify

#### Persistence Layer

- `crates/persistence/src/lib.rs`
  - **Remove:** `Persistence::get_user_id()` method (lines ~1842-1900)
  - This method queries `user_id` by `(bid_year_id, area_id, initials)`
  - Violates the "no reconstruction" rule

#### Server Layer

- `crates/server/src/main.rs`
  - **Remove:** `extract_user_id_from_state()` function (lines ~935-950)
  - This function searches `State.users` by initials to retrieve `user_id`
  - Used after registration to correlate initials → `user_id`
  - Violates the "no reconstruction" rule

- `crates/server/src/main.rs`
  - **Refactor:** `handle_register_user()` (lines ~950-1050)
  - Must return `user_id` directly from registration operation
  - Cannot rely on `extract_user_id_from_state()`

#### API Layer

- `crates/api/src/handlers.rs`
  - **Refactor:** `register_user()` function
  - Must return `user_id` in `RegisterUserResponse`
  - Persistence must provide `user_id` immediately after insert

---

## Invariants Being Enforced

1. **No initials-based lookup:** No function may accept initials as input and return `user_id` as output
2. **No state reconstruction:** No function may search canonical state by initials to retrieve `user_id`
3. **No fallback resolution:** If `user_id` is not present, operations must fail immediately — no layer may attempt to resolve identity from initials
4. **Identity must be returned explicitly:** Operations that create users must return `user_id` directly

This applies to:

- API handlers
- Server helpers
- Domain/core logic
- Persistence helpers
- Test helpers

---

## Explicit Non-Goals

- **Do NOT** change Command definitions yet (that's Phase 28B)
- **Do NOT** modify domain types or core logic
- **Do NOT** change how commands are constructed (API still receives `user_id` in requests)
- **Do NOT** alter completeness logic (that's Phase 28C)

---

## Implementation Strategy

### Step 1: Update Registration Flow

**Current Pattern (Violates Invariant):**

```rust
// API handler returns RegisterUserResponse with initials
let result = register_user(...)?;

// Server layer reloads state and searches by initials
let user_id = extract_user_id_from_state(&state, &result.initials)?;
```

**Correct Pattern:**

```rust
// API handler returns RegisterUserResponse with user_id
let result = register_user(...)?;
let user_id = result.user_id; // Already present, no lookup needed
```

**Required Changes:**

- `RegisterUserResponse` must include `user_id` field
- `register_user()` handler must retrieve `user_id` from persistence after insert
- Persistence layer must expose `user_id` after `INSERT` via `last_insert_rowid()` or `RETURNING`

---

### Step 2: Remove Helper Functions

**Delete:**

- `Persistence::get_user_id()`
- `extract_user_id_from_state()`

**Rationale:**
These functions exist solely to reconstruct identity from initials. Their presence creates architectural risk and violates Phase 28 invariants.

---

### Step 3: Update Tests

**Affected Tests:**

- Any test calling `get_user_id()` directly
- Any test relying on `extract_user_id_from_state()`
- Registration flow tests expecting initials-only responses

**Correction Strategy:**

- Tests must assert on `user_id` returned from registration
- Tests must not reconstruct identity from initials
- Tests may query by `user_id` directly

---

## Completion Conditions

Phase 28A is complete when:

- ✅ `Persistence::get_user_id()` has been removed
- ✅ `extract_user_id_from_state()` has been removed
- ✅ `RegisterUserResponse` includes `user_id`
- ✅ Registration flow returns `user_id` without intermediate lookup
- ✅ No `grep -r "get_user_id"` matches remain in non-test code
- ✅ No `grep -r "extract_user_id"` matches remain
- ✅ No fallback resolution paths exist (operations fail if `user_id` missing)
- ✅ All tests pass
- ✅ `cargo xtask ci` passes
- ✅ `pre-commit run --all-files` passes

---

## Expected Risks & Edge Cases

### Risk: Breaking Existing Tests

**Likelihood:** High

**Mitigation:**

- Update test assertions to expect `user_id` in responses
- Remove test helpers that rely on initials-based lookup

---

### Risk: Registration Flow Complexity

**Likelihood:** Medium

**Concern:** Retrieving `user_id` immediately after `INSERT` may require backend-specific logic.

**Mitigation:**

- SQLite: Use `last_insert_rowid()`
- MySQL: Use `LAST_INSERT_ID()` or `RETURNING` clause
- Both patterns already exist in codebase (e.g., audit event insertion)

---

### Risk: Cascading Changes

**Likelihood:** Low

**Mitigation:**

- This phase is narrowly scoped to helper removal
- Commands are not changed yet (Phase 28B)
- API request structures already use `user_id`

---

## Stop-and-Ask Conditions

If any of the following occur, stop and request guidance:

- Registration cannot return `user_id` without significant schema changes
- Removing helpers breaks core domain logic (not just tests)
- Backend-specific behavior creates irreconcilable divergence
- More than 10 test files require modification
- Any persistence query still requires initials-based lookup for correctness

---

## Validation Checklist

Before marking Phase 28A complete:

- [ ] `grep -rn "get_user_id" crates/` returns zero matches in non-test code
- [ ] `grep -rn "extract_user_id" crates/` returns zero matches
- [ ] `RegisterUserResponse` struct includes `user_id: i64`
- [ ] `handle_register_user()` uses `user_id` from response, not lookup
- [ ] All registration tests assert on `user_id` presence
- [ ] No fallback resolution logic exists in any layer
- [ ] Operations fail immediately if `user_id` is not present
- [ ] CI passes: `cargo xtask ci`
- [ ] Linting passes: `pre-commit run --all-files`
- [ ] No new clippy warnings introduced

---

## References

- **Authoritative Spec:** `plans/PHASE_28.md`
- **Identity Rules:** `AGENTS.md` § Canonical Identity Enforcement
- **Related Phases:** Phase 23A (established canonical identity model)
