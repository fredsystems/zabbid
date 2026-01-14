# Phase 23E — Canonical Bootstrap Finalization

## Goal

Finalize canonical identity enforcement by ensuring **bootstrap paths and test infrastructure
produce real, persisted canonical entities** (bid years, areas, users) rather than metadata-only
representations.

This phase closes the remaining gaps revealed by Phase 23D’s contract honesty work.

---

## Background

After Phases 23A–23D:

- Canonical numeric IDs (`bid_year_id`, `area_id`, `user_id`) are now the **sole internal identity**
- API responses are honest and always return fully populated IDs
- Persistence correctly rejects references to non-existent canonical entities
- Tests that previously relied on metadata-only bootstrap now fail correctly

Phase 23E exists to **fix bootstrap and tests**, not to weaken enforcement.

---

## Scope

### Bootstrap Behavior

- Ensure all bootstrap helpers:
  - Persist bid years before use
  - Persist areas before use
  - Populate `BootstrapMetadata` using **persisted canonical entities**
- No bootstrap path may fabricate:
  - bid years
  - areas
  - users
  - IDs
- Metadata must reflect **what exists**, not what is implied

---

### Test Infrastructure

- Fix failing API and persistence tests by:
  - Using real bootstrap flows
  - Persisting required canonical entities explicitly
- Remove all assumptions that:
  - metadata implies persistence
  - year / area code implies existence
- Tests must fail if canonical state is missing

---

## Explicit Non-Goals

This phase MUST NOT:

- Change UI behavior
- Change API contracts
- Change domain rules
- Introduce sentinel values or fake records
- Add workaround logic to “make tests pass”
- Refactor unrelated persistence or query code
- Introduce ORMs or migrations

---

## Constraints

- Canonical identity rules remain strict
- Persistence layer must continue to reject invalid references
- All fixes must be **honest**, explicit, and minimal
- Changes must be limited to:
  - bootstrap helpers
  - test setup
  - test expectations

---

## Exit Criteria

- All tests pass without:
  - sentinel IDs
  - fake entities
  - implicit bootstrap assumptions
- Bootstrap paths always create canonical state before use
- No code paths fabricate IDs or entities
- Phase 23 closes with:
  - a stable database shape
  - honest contracts
  - correct bootstrap semantics

---

## Post-Phase Expectations

After Phase 23E:

- Canonical identity is fully enforced end-to-end
- Future features (rounds, No Bid area, bid year start) can be added **without refactors**
- ORM migration discussions can proceed from a stable foundation
