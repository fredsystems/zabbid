# Phase 28 — Canonical Identity & Domain Correctness Enforcement

## Overview

Phase 28 enforces three critical architectural invariants and fixes one domain correctness bug:

1. **Canonical Identity Only** — `user_id` is the sole identifier; initials must never be used for lookup, mutation, or correlation
2. **Identity Must Be Explicit** — Identity cannot be inferred, reconstructed, or derived; if `user_id` is not present, the operation must fail immediately with no fallback resolution
3. **Audit Identity Invariant** — Audit events must reference users by `user_id` only; initials may appear only as contextual metadata, never as identity or correlation keys
4. **System Area Exclusion** — System-designated areas (identified by `is_system_area = true`, including No Bid) must not count toward expected area totals in completeness/readiness logic

---

## Sub-Phase Breakdown

Phase 28 is decomposed into four independently executable sub-phases:

### **Phase 28A — Remove Identity Reconstruction Helpers & Patterns**

**Purpose:** Eliminate all code paths that translate initials → `user_id` or reconstruct identity from mutable fields. No layer may fallback-resolve `user_id` from initials if `user_id` is missing.

**Scope:**

- Remove `Persistence::get_user_id()` method
- Remove `extract_user_id_from_state()` from server layer
- Refactor registration flow to return `user_id` directly
- Remove any other initials-based lookup patterns

**Deliverable:** No helper functions remain that derive canonical identity from display metadata. All operations fail immediately if `user_id` is not present.

---

### **Phase 28B — Make Commands Carry Canonical user_id**

**Purpose:** Ensure all commands that target users carry `user_id` explicitly, not just initials. Audit events emitted by commands must reference users by `user_id` only.

**Scope:**

- Update `Command::UpdateUser` to include `user_id`
- Update `Command::OverrideAreaAssignment` to include `user_id`
- Update `Command::OverrideEligibility` to include `user_id`
- Update `Command::OverrideBidOrder` to include `user_id`
- Update `Command::OverrideBidWindow` to include `user_id`
- Update `core/apply.rs` to use `user_id` from commands
- Update API handlers to pass `user_id` into commands

**Deliverable:** Commands no longer allow identity to be inferred; `user_id` is required and explicit. Audit events reference users by canonical ID.

---

### **Phase 28C — Fix No-Bid Area Exclusion in Completeness Logic**

**Purpose:** Correct the domain bug where system-designated areas are incorrectly counted toward expected area totals.

**Scope:**

- Update `get_actual_area_count()` to filter by `is_system_area = false` (flag-based, not name-based)
- Update related completeness/readiness queries if needed
- Add regression tests verifying system area exclusion

**Deliverable:** Completeness logic correctly excludes system-designated areas (identified by flag, not name) from expected counts.

---

### **Phase 28D — Test Hardening & Validation**

**Purpose:** Ensure all Phase 28 invariants are covered by comprehensive tests.

**Scope:**

- Add tests verifying no initials-based lookup paths remain
- Add tests verifying commands require explicit `user_id` with no fallback resolution
- Add tests verifying audit events reference users by `user_id` only
- Add tests verifying system areas do not block completeness
- Add tests for edge cases (e.g., duplicate initials across areas)

**Deliverable:** Full test coverage for canonical identity enforcement, audit identity invariant, and system area exclusion.

---

## Execution Order

Sub-phases must be executed in order:

1. **28A** removes the architectural violations (lookup helpers)
2. **28B** enforces explicit identity in commands
3. **28C** fixes the domain counting bug
4. **28D** validates all invariants via tests

Each sub-phase is independently reviewable and mergeable.

---

## Success Criteria

Phase 28 is complete when:

- ✅ No code paths translate initials → `user_id` or fallback-resolve identity
- ✅ All user-targeting commands carry `user_id` explicitly
- ✅ Audit events reference users by `user_id` only
- ✅ System-designated areas are excluded from expected area counts (flag-based)
- ✅ All invariants are validated by passing tests
- ✅ `cargo xtask ci` passes
- ✅ `pre-commit run --all-files` passes

---

## Risk Assessment

- **Phase 28A:** Medium risk — touches registration flow, may break existing tests
- **Phase 28B:** High risk — large cascade through command/apply/API layers
- **Phase 28C:** Low risk — isolated query change with clear test validation
- **Phase 28D:** Low risk — additive test coverage only

---

## References

- **Authoritative Spec:** `plans/PHASE_28.md`
- **Architectural Rules:** `AGENTS.md` (Canonical Identity Enforcement section)
- **Related Phases:** Phase 23A (Canonical Identity for Area & Bid Year)
