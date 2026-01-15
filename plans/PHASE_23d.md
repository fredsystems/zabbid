# Phase 23D — API Contract Stabilization (Canonical Identity)

## Goal

Stabilize the API contract so it is **fully consistent, honest, and consumable**
under the new canonical identity model (bid_year_id, area_id, user_id).

This phase resolves ambiguity introduced during Phase 23C and establishes a
frozen API surface suitable for test repair and UI consumption.

---

## Scope

**Included:**

- API handlers (`crates/api`)
- API request/response types
- Server glue (`crates/server`)
- API tests **only where contract meaning changes**

**Explicitly Excluded:**

- UI changes
- Persistence changes
- Domain model changes
- Test fixture refactors beyond what is required to validate the API contract

---

## Core Principles

- The API must never “guess” or infer identity.
- If an ID is present in a response, it must be:
  - Canonical
  - Valid
  - Persisted
- If an ID cannot be guaranteed, it must be **explicitly optional** and
  documented as such.
- Temporary shims (e.g. `Option<T>` added just to compile) must be resolved.

---

## Required Work

### 1. Contract Review (Mandatory)

For **every API response type** containing:

- `bid_year_id`
- `area_id`
- `user_id`

Decide and enforce:

- Is this field **always present**, or **conditionally present**?
- If conditional:
  - Under what exact circumstances?
  - Is the API consumer expected to re-fetch?

No field may remain optional “just in case”.

---

### 2. Handler Corrections

- Ensure all handlers:
  - Populate canonical IDs from persisted state or metadata **only when guaranteed**
  - Do not rely on domain-only objects to supply IDs
- Remove partial or speculative ID extraction
- Ensure error cases are explicit (do not return partial success with missing IDs)

---

### 3. Response Shape Alignment

- Update response structs so:
  - Required fields are non-optional
  - Optional fields are justified and documented
- Ensure server-layer response mapping mirrors API types exactly
- Eliminate duplicate or divergent response structs

---

### 4. API Test Updates (Contract-Level Only)

- Update or add API tests **only to validate the corrected contract**
- Tests may fail if setup is invalid — that is acceptable
- Do NOT:
  - Add sentinel values
  - Add test-only branches
  - Relax assertions to “make tests pass”

---

## Explicit Non-Goals

- Do NOT fix test fixture setup globally (that is Phase 23E)
- Do NOT introduce workaround logic for missing canonical state
- Do NOT modify persistence behavior
- Do NOT modify UI types or logic

---

## Exit Criteria

- All API handlers compile and behave consistently
- API response types accurately reflect canonical identity guarantees
- API tests fail **only** due to invalid test setup, not API ambiguity
- No response contains an ID that may be invalid or inferred
- API surface is considered **frozen** for Phase 23E and Phase 23F

---

## Notes

This phase intentionally prioritizes **correctness over convenience**.

Breaking tests here is acceptable.
Lying to consumers is not.

If ambiguity arises, STOP and ask.
