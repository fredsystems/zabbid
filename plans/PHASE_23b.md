# Phase 23B

## Phase 23B — Canonical Identity Completion (API + Server + Tests)

### Goal

Refactor the API + server layers to fully operate on canonical numeric identifiers for **bid years** and **areas**, and eliminate all remaining reliance on display identifiers (`year`, `area_code`) for mutation routing, persistence, or audit persistence.

Phase 23A established correct persistence + domain invariants. Phase 23B makes the API/server honest under those invariants and restores a fully-green test suite **without hacks**.

---

## Non-Goals

- No UI changes (UI will be updated in Phase 23C)
- No workflow changes beyond replacing identity fields
- No CSV shape changes beyond swapping identifiers under the hood
- No sentinel IDs, stub canonical records, or “auto-create to satisfy FK” behavior

---

## Core Requirements

### 1) Canonical IDs everywhere (API contract)

All **mutating** API requests that currently accept any of the following as identity MUST be refactored to use canonical IDs:

- `bid_year` / `year` (display) → `bid_year_id`
- `area_id` string / `area_code` (display) → `area_id` (numeric)

All **read** models MUST include canonical IDs plus the display fields:

- BidYear read models include:
  - `bid_year_id`
  - `year`
- Area read models include:
  - `area_id`
  - `area_code` (and `area_name` if present)
  - `bid_year_id` (or the bid year display field, if already included)
- User read models include:
  - `user_id` (already done in Phase 21)
  - `area_id` (numeric)
  - `bid_year_id` (numeric)
  - plus display fields as needed (`initials`, etc.)

---

## Operator Management Audit Scope Fix (Critical)

Phase 23A correctly rejects audit events that reference non-existent bid years/areas.

Currently operator-management actions (create/disable/enable/delete/reset/change password) emit audit events scoped to:

- `BidYear(0)`
- `Area("_operator_management")`

This must be removed.

### Policy

Operator-management audit events MUST be persisted without requiring a real bid year/area.

### Implementation Options (pick one, do not mix)

Option A — Make bid_year_id nullable for operator-management events

- Allow `bid_year_id` to be nullable in `audit_events` for global/non-bid-year events
- Persist operator-management audit events with:
  - `bid_year_id = NULL`
  - `area_id = NULL`
- Update persistence timeline queries to include these events (ordered, stable)

Option B — Introduce a dedicated audit stream for operator management

- New canonical table for operator-management audit events
- Read endpoints merge streams only when explicitly requested

**Preferred option:** Option A (simpler, consistent with existing “global event” handling like CreateBidYear)

> Note: area_id is already nullable for some events; Phase 23B must formalize and extend this properly for operator-management events without hacks.

---

## Endpoint Refactor Requirements

### Active Bid Year APIs

- Active bid year must be tracked by `bid_year_id` end-to-end
- Read API returns:
  - active bid year id + display year
- Mutating endpoints must not accept a year number for identity

### Bootstrap APIs

All bootstrap mutations should:

- Resolve active bid year by `bid_year_id`
- Use canonical `area_id` for area-scoped operations
- Never accept `area_code` for mutation identity

---

## Test Refactor Scope

Phase 23B must fix the failures introduced by Phase 23A by updating setup to create canonical state properly and by refactoring operator-management audit semantics.

### Must be updated

- `crates/api` tests
- `crates/server` tests
- Any persistence tests that were previously relying on “year 0” or “fake” bid year/area existence

### Principles

- Tests must bootstrap real bid years/areas when required
- Tests must not use sentinel values (0, "\_operator_management") for identity
- Operator-management tests must no longer require bid year/area bootstrap at all if using global audit scoping

---

## Implementation Tasks

### 1) API Contracts

- Update request/response DTOs:
  - Replace `year` identity fields with `bid_year_id`
  - Replace `area_code`/string identity fields with numeric `area_id`
- Update API handlers:
  - Resolve entities by IDs (not display fields)
  - Ensure errors remain structured and non-leaky

### 2) Server HTTP Surface

- Update route payloads to accept IDs instead of display values
- Update route handlers accordingly
- Update `api_cli.py` for all endpoint schema changes (required)

### 3) Operator Management Audit Events

- Implement selected audit scoping option (A preferred)
- Update persistence timeline queries to handle global events correctly
- Ensure no state mutation creates canonical bid years/areas implicitly

### 4) Tests

- Update test setup across `api` + `server` to use canonical IDs
- Remove all uses of `BidYear::new(0)` and area sentinel strings
- Ensure operator-management tests pass without requiring bid-year bootstrap

---

## Exit Criteria

Phase 23B is complete when:

- All operator-management endpoints work and tests pass **without** requiring bid year/area scope
- No mutating API accepts year number or area_code as identity
- All read APIs include canonical IDs + display fields
- All tests across workspace pass:
  - `cargo xtask ci`
  - `pre-commit run --all-files`
- No sentinel IDs, stub state, or implicit canonical creation exists anywhere

---

## Notes / Follow-Up

Phase 23C (UI) will:

- Update UI to consume canonical IDs
- Continue using display fields for presentation
- Keep routing and selection logic stable while switching underlying identity usage
