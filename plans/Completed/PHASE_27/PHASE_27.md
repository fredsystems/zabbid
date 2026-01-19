# Phase 28 — Canonical Identity & Domain Correctness Enforcement

## Purpose

Enforce **explicit, canonical identity semantics** across the entire system and
correct domain logic that currently relies on implicit or derived assumptions.

This phase eliminates two related but distinct classes of architectural flaws:

1. **Identity misuse** — treating mutable attributes as entity identity
2. **Identity reconstruction** — inferring or re-deriving identity instead of requiring it explicitly

In addition, this phase corrects a **domain correctness bug** where the
“No Bid” area is incorrectly included in expected-area counts, violating
business intent.

After this phase, identity handling and domain invariants are:

- explicit
- non-derivable
- replay-safe
- rename-safe
- audit-safe

---

## Core Invariants (Non-Negotiable)

### Invariant 1 — Canonical Identity Only

> **Mutable domain attributes must never be used as entity identity.**

For users specifically:

- `user_id` is the **only** canonical identifier
- Initials are:
  - display metadata
  - validation input (e.g. uniqueness checks)
  - audit context only
- Initials are **never**:
  - command selectors
  - mutation targets
  - persistence lookup keys
  - audit correlation identifiers

This invariant applies to **all layers**:

- domain / core
- API / server
- persistence
- test helpers

---

### Invariant 2 — Identity Must Be Explicit, Never Inferred

> **If an entity’s identity is not already present, the operation must fail.**

The system must never:

- infer identity from mutable fields
- translate initials → `user_id` for mutation
- “helpfully” reconstruct identity from state
- fall back to lookups when identity is missing

Implications:

- Commands must _carry_ canonical identity
- Helpers must not derive identity
- Persistence queries must not return identity from non-canonical fields
- Tests must not rely on implicit resolution

If `user_id` is not available, the operation is invalid.

---

## Domain Correctness Correction

### No-Bid Area Must Not Count Toward Expected Area Totals

The domain currently includes the **No Bid** area when calculating expected
area counts for completeness and readiness checks.

This is incorrect.

#### Correct Rule

- “No Bid” is a **sentinel / structural area**
- It exists for assignment and bidding semantics
- It must **never**:
  - count toward expected area totals
  - block bootstrap completeness
  - affect readiness or progression logic

Expected-area calculations must:

- exclude No-Bid explicitly
- remain correct even if No-Bid is renamed or reordered
- rely on explicit domain intent, not incidental representation

This correction is part of Phase 28 because it stems from the same root cause:
**implicit assumptions leaking into domain logic**.

---

## Scope

This phase addresses:

- identity semantics
- command correctness
- helper behavior
- persistence guarantees
- domain invariants

### Included

- Refactoring commands to require canonical identity
- Removing or constraining identity translation helpers
- Aligning API handlers with explicit identity rules
- Hardening tests to prevent regression
- Correcting expected-area counting logic
- Adding regression tests for No-Bid exclusion

### Explicitly Excluded

- UI redesign (unless required for correctness)
- Bidding logic changes
- Performance optimizations
- Schema redesign beyond identity correctness
- Speculative or future domain rules

---

## Architectural Rules Going Forward

After Phase 28:

- No command may be constructed without canonical identity
- No helper may return a canonical identifier derived from mutable data
- Identity flows are one-directional and explicit
- Domain counts reflect business intent, not structural artifacts
- Rename, replay, and audit safety are guaranteed by construction

Any future code violating these rules is considered a **hard regression**.

---

## Files Likely to Be Affected

### Backend

- `crates/core/src/command.rs`
- `crates/core/src/apply.rs`
- `crates/domain/src/*`
- `crates/api/src/handlers/*`
- `crates/server/src/*`
- `crates/persistence/src/*`

### Tests

- Core command construction tests
- Domain invariant tests
- Persistence correctness tests
- Regression tests for identity misuse
- Regression tests for No-Bid exclusion

---

## Completion Conditions

Phase 28 is complete when:

- No command, helper, or mutation path accepts mutable attributes as identity
- No code reconstructs or infers identity
- All user mutations are keyed exclusively by `user_id`
- Initials are used only for display or validation
- No-Bid area is excluded from expected-area calculations
- Regression tests lock all invariants
- All tests pass
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes

---

## Rationale

This phase resolves a subtle but dangerous architectural inconsistency:

- Persistence and API layers already enforce canonical identity
- The core/domain layer historically relied on implicit resolution
- Domain counting logic relied on incidental structure

These shortcuts make:

- renames fragile
- audits ambiguous
- replays unsafe
- future bidding logic risky

By enforcing **explicit identity and explicit domain intent everywhere**, the
system becomes:

- easier to reason about
- safer to evolve
- harder to misuse
- future-proof

---

## When to Stop

Stop and ask if enforcing these invariants would require:

- guessing domain intent
- weakening audit guarantees
- introducing fallback identity logic
- altering business rules beyond correctness

Correctness beats convenience.
