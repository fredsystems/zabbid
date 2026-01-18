# Phase 27 — Correctness, Audit, and Refinement Pass

## Phase Objective

Phase 27 is a **system-wide correctness and hygiene pass**.  
No new domain features are introduced.

The goal is to:

- Eliminate ambiguity in identity, lookup, and mutation semantics
- Make test coverage explicit, deterministic, and measurable
- Remove accumulated technical and behavioral debt
- Align code, tests, and AGENTS.md with the system as it exists today

This phase prioritizes **trustworthiness over velocity**.

---

## Phase Scope Overview

Phase 27 is composed of the following workstreams:

- 27A — Agent Rules & Invariant Audit
- 27B — Identity & Initials Correctness
- 27C — Test Audit, Coverage, and Flake Elimination
- 27D — UI Refinement and Precision Fixes
- 27E — Tooling & Dependency Cleanup
- 27F — Override Visibility & Polish (Deferred / Out of Scope)

---

## Workstream 27A — Agent Rules & Invariant Audit

### Workstream 27A Purpose

Ensure AGENTS.md accurately reflects:

- Current canonical identity rules
- Editing semantics post-canonicalization
- Override intent and audit requirements
- Frontend and backend responsibility boundaries

AGENTS.md is **authoritative** and must not drift behind the codebase.

### Workstream 27A Scope

- Audit AGENTS.md against:
  - Implemented behavior through Phase 26
  - Canonical identity rules for users, areas, and bid years
  - Editing and override semantics
- Clarify language where agents previously misinterpreted intent
- Explicitly forbid patterns that have caused prior errors:
  - Inline styles in UI
  - Identifier misuse (e.g., initials, area codes)
  - Compensating logic for missing canonical state

### Workstream 27A Deliverables

- Updated AGENTS.md committed to main
- No rule ambiguity around:
  - User identity vs display metadata
  - Canonical vs derived state
  - Agent stopping conditions

### Workstream 27A Completion Conditions

- AGENTS.md passes markdownlint
- Rules align with actual system behavior
- No unresolved contradictions between documentation and code

---

## Workstream 27B — Identity & Initials Correctness

### Workstream 27B Purpose

Eliminate all remaining misuse or overloading of **user initials**.

Initials are **display metadata**, not identity.

### Workstream 27B Scope

- Audit all code paths for:
  - User lookup
  - Mutation
  - State transitions
- Replace any remaining initials-based logic with `user_id`
- Ensure initials:
  - Are editable at all lifecycle stages
  - Are never used as stable identifiers
- Update tests to reflect correct identity semantics

### Workstream 27B Deliverables

- All persistence, lookup, and mutation paths use `user_id`
- Initials treated as mutable display data only
- Regression tests preventing future misuse

### Workstream 27B Completion Conditions

- No code path uses initials as an identifier
- All tests pass with initials freely editable
- Identity rules in AGENTS.md are enforced in code

---

## Workstream 27C — Test Audit, Coverage, and Flake Elimination

### Workstream 27C Purpose

Make the test suite:

- Deterministic
- Complete
- Auditable
- Trustworthy as a correctness signal

### Workstream 27C Scope

Ignored Test Review:

- Enumerate all `#[ignore]` tests
- For each ignored test:
  - Remove if obsolete
  - Gate behind explicit `xtask` runners if integration-only
  - Unignore if now hermetic
- No ignored tests may remain without explicit justification

Coverage Pass:

- Run coverage using `llvm-cov`
- Identify untested mutation paths and failure cases
- Add tests covering missing behavior, especially:
  - Authorization failures
  - Validation errors
  - Lifecycle gating
  - Canonicalization boundaries

Flaky Test Elimination:

- Investigate nondeterministic failures, including:
  - `tests::test_invalid_session_token_rejected`
- Audit for:
  - Shared mutable state
  - Ordering assumptions
  - Randomness
  - Clock or time dependence
- Fix via:
  - Per-test databases
  - Explicit setup and teardown
  - Fixed secrets and deterministic inputs

### Workstream 27C Deliverables

- LLVM coverage report generated via `llvm-cov`
- New tests covering previously untested paths
- Removal or justification of all ignored tests
- Elimination of flaky or nondeterministic behavior

### Workstream 27C Completion Conditions

- `cargo test` passes reliably across repeated runs
- Coverage confirms all critical mutation paths are exercised
- No unexplained ignored or flaky tests remain

---

## Workstream 27D — UI Refinement and Precision Fixes

### Workstream 27D Purpose

Correct UI imprecision and visual inconsistencies without altering domain behavior.

### Workstream 27D Scope

- Fix imprecise layout constraints:
  - Remove ham-fisted width constraints
  - Ensure consistent input sizing without arbitrary containers
- Resolve hover and visited-state inconsistencies:
  - No underline on action buttons
  - Text color remains readable and consistent
- Ensure styling compliance:
  - No inline styles
  - SCSS only
  - Mobile-first layouts preserved

### Workstream 27D Deliverables

- Cleaned and consistent admin UI
- No regressions in mobile usability
- Styling aligned with Bootstrap Completeness patterns

### Workstream 27D Completion Conditions

- UI behaves consistently across navigation paths
- No inline styles introduced
- Manual verification on mobile and desktop

---

## Workstream 27E — Tooling & Dependency Cleanup

### Workstream 27E Purpose

Remove unused or misleading tooling that implies unsupported workflows.

### Workstream 27E Scope

- Remove `api_cli.py` requirements and references
- Ensure CLI tooling reflects actual supported surfaces
- Verify no documentation or scripts depend on removed tools

### Workstream 27E Deliverables

- Clean dependency graph
- No dead tooling referenced in code or docs

### Workstream 27E Completion Conditions

- Tooling removal causes no regressions
- `cargo xtask ci` passes cleanly
- Documentation reflects supported workflows only

---

## Explicitly Out of Scope

- New domain features
- Bidding logic
- Performance optimization
- Phase 26F override visibility work

---

## Phase 27 Exit Criteria

Phase 27 is complete when:

- AGENTS.md is accurate, enforced, and lint-clean
- Identity semantics are unambiguous and correct
- Test suite is deterministic and coverage-backed
- UI precision issues are resolved
- Tooling reflects actual supported usage
- No correctness debt remains hidden
