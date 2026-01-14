# Phase 23C — Canonical Identity Propagation (API → UI)

## Goal

Complete the canonical identity migration by propagating `bid_year_id` and `area_id`
through the API and UI layers, while preserving all existing workflows and user-visible behavior.

This phase makes numeric IDs first-class _everywhere_, without introducing new features
or changing domain rules.

---

## Scope

### In Scope

#### API Layer

- Ensure **all read APIs** expose canonical IDs:
  - `bid_year_id`
  - `area_id`
- Preserve human-facing display fields:
  - `year`
  - `area_code`
  - `area_name`
- Ensure **all mutation APIs** accept canonical IDs where identity is required
- Remove any remaining reliance on:
  - bid year number (`year`) as identity
  - area code as identity
- Maintain backward-compatible payload shapes where possible, but IDs must be present

#### UI Layer

- Update UI state and routing to treat:
  - `bid_year_id` as canonical bid year identity
  - `area_id` as canonical area identity
- Continue displaying:
  - bid year by `year`
  - areas by `area_code` / `area_name`
- Replace any implicit identity assumptions based on:
  - route params like `/bid-year/2026`
  - area code lookups
- Ensure all UI actions use IDs when invoking API calls

#### Tests

- Update tests in:
  - `api`
  - `server`
  - `ui`
- Tests must:
  - Assert IDs are present
  - Use IDs for identity
  - Avoid sentinel values or display fields as identifiers

---

## Explicit Non-Goals

- ❌ No workflow changes
- ❌ No new UI features
- ❌ No CSV changes
- ❌ No permission or capability changes
- ❌ No domain rule changes
- ❌ No bootstrap behavior changes

---

## Constraints

- Domain and persistence layers are already correct — do not rework them
- No sentinel values, fake IDs, or auto-creation logic
- No test-skipping or test-ignoring to “get green”
- No scripts, regex replacement, or out-of-band refactors
- Changes must be incremental, explicit, and test-driven

---

## Exit Criteria

- All internal references use:
  - `bid_year_id` for bid years
  - `area_id` for areas
- No API mutation or lookup uses:
  - `year` or `area_code` as identity
- UI state and routing are ID-based
- Display fields are presentation-only
- All tests pass:
  - `cargo test --workspace`
  - `cargo xtask ci`
  - `pre-commit run --all-files`
- No UI regressions or behavior changes observed

---

## Architectural Notes

- This phase completes the identity model started in Phase 21 (users) and Phase 23A (areas/bid years)
- After this phase:
  - IDs are canonical everywhere
  - Display values are just display
  - Future features (No Bid area, rounds, locks) can be implemented cleanly

---

## When to Stop

If any of the following occur:

- An API change would require guessing UI behavior
- A UI change would require inventing new domain rules
- A test failure suggests missing bootstrap setup
- The scope expands beyond identity propagation

→ Stop and ask before proceeding.
