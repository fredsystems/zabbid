# Phase 21

## Phase 21 — Canonical User Identity Refactor

### Objective

Introduce a stable, opaque internal identifier for users (`user_id`) while preserving existing domain rules around initials. This refactor eliminates reliance on mutable, display-oriented identifiers as canonical identity and prepares the system for safe user editing, deletion, and reassignment.

Phase 21 is intentionally split into sub-phases to control blast radius and keep agent work tractable.

---

## Phase 21A — Persistence & Domain Foundation

### Phase 21A Scope

- Persistence schema
- Persistence queries
- Persistence tests
- Minimal domain adjustments (only if required)

### Phase 21A Requirements

- Introduce a new internal `user_id` field:
  - Opaque (numeric or UUID)
  - Primary key for users
- Preserve existing domain rules:
  - Initials remain required
  - Initials remain unique per bid year
  - Initials remain human-facing and editable (future)
- Update all persistence queries to:
  - Use `user_id` as canonical identity
  - Treat initials as data, not identity
- Update audit events to reference `user_id` where applicable
- Update persistence tests to reflect the new identity model

### Phase 21A Explicit Non-Goals

- No API changes
- No UI changes
- No behavior changes beyond identity plumbing

---

## Phase 21B — API Contract Refactor

### Phase 21B Scope

- API request/response types
- API handlers
- Authorization remains unchanged

### Phase 21B Requirements

- Refactor APIs to reference users by `user_id`
- Preserve ergonomic lookup by initials where appropriate (creation, CSV import)
- Ensure all user mutations target `user_id`
- Update API-level errors to reflect identity changes
- Maintain audit clarity (`target_user_id` always present)

### Phase 21B Explicit Non-Goals

- No persistence changes
- No UI changes
- No test refactors outside the API crate

---

## Phase 21C — Test Migration (Core, API, Server)

### Phase 21C Scope

- Core tests
- API tests
- Server tests

### Phase 21C Requirements

- Update all tests to use `user_id` where required
- Remove assumptions that initials are canonical identity
- Preserve all existing test coverage and assertions
- No logic changes
- No new features

### Phase 21C Explicit Non-Goals

- No persistence changes
- No UI changes
- No behavior changes

---

## Phase 21D — UI Adaptation

### Phase 21D Scope

- Admin UI
- Bootstrap UI
- User management UI

### Phase 21D Requirements

- UI uses `user_id` as internal identity
- Initials remain visible and editable only where allowed
- No UI-driven domain assumptions
- No change to CSV import UX semantics

### Phase 21D Explicit Non-Goals

- No backend changes
- No new domain rules
- No styling refinements (handled later)

---

## Phase 21 Completion Criteria

Phase 21 is complete when:

- Users have a stable internal identity
- Initials are no longer used as canonical identifiers
- All tests pass without ignored cases
- API contracts are explicit and stable
- UI correctly consumes the new identity model
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
