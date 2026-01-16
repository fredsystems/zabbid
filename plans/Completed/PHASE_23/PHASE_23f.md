# Phase 23F — UI Stabilization Against Canonical Identity

## Goal

Stabilize the UI after Phase 23 canonical identity changes by ensuring all identity usage is based on canonical numeric IDs, without changing UX, workflows, or visuals.

This phase is about **correctness and alignment**, not new functionality.

---

## Scope (Allowed)

### Canonical Identity Usage

- The UI must use numeric canonical IDs exclusively for:
  - routing
  - API mutations
  - comparisons
  - list keys
- Display fields (`year`, `area_code`, `area_name`, initials) are **presentation-only**

---

### Defensive Assumptions

- UI must assume:
  - Canonical IDs are always present where required by the API contract
  - Display values may change independently of identity
- UI must not:
  - compare entities by display values
  - rely on array index or composite strings for identity

---

### Capability Alignment

- UI actions must continue to be gated only by capability flags
- No role-based permission logic
- No inferred permissions

---

## Explicitly Out of Scope

- No new UI features
- No UX or visual changes
- No navigation redesign
- No styling changes
- No workflow changes
- No backend or API changes

If any of the above become necessary, **stop and ask**.

---

## Required Work

### 1. Routing & Navigation

- Ensure all routes use canonical IDs:
  - `/bid-year/:bidYearId`
  - `/bid-year/:bidYearId/areas/:areaId`
- Remove any routing logic that depends on:
  - year numbers
  - area codes

---

### 2. API Consumption

- Ensure all API calls:
  - pass canonical IDs
  - never pass display values as identifiers
- Audit API client code to confirm:
  - IDs are threaded through consistently
  - no fallback behavior exists

---

### 3. UI State & Keys

- All React keys must use canonical IDs
- Do not use:
  - array indices
  - composite strings (e.g. `${year}-${area}`)

---

### 4. Type Safety

- TypeScript types must:
  - require canonical IDs where guaranteed by the API
  - not mark IDs as optional unless the API contract allows it
- Remove any outdated optional fields (`?`) that no longer reflect reality

---

### 5. Runtime Safety Checks (Lightweight)

- Where reasonable:
  - assert IDs exist before use
  - fail loudly in development if invariants are violated
- Do not introduce silent fallback behavior

---

## Exit Criteria

- UI builds cleanly
- UI functions correctly using canonical IDs only
- No UI logic relies on display fields for identity
- Capability-based gating remains unchanged
- No visual or workflow changes
- No new TODOs introduced
