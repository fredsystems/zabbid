# PHASE_11.md

## Phase 11: Operator-Centric Read Models & API Ergonomics

### Phase 11 Goal

Design and implement **operator-focused, ergonomic read APIs** that support real-world workflows and UI development **without introducing new domain logic or weakening invariants**.

Phase 11 exists to shape the API around **how the system is used**, not how it is stored or validated.

This phase intentionally responds to UI-driven needs while preserving strict backend authority.

---

### Phase 11 Scope

Phase 11 includes:

- Operator-centric read API design
- Composite and derived read models
- Ergonomic APIs for bootstrapping and operational workflows
- Explicit support for UI-driven data access patterns
- Read-only API refactoring and additions
- API surface changes driven by UX needs
- Updates to API tooling (`api_cli.py`) to match API changes
- Tests validating read correctness and determinism

Phase 11 explicitly excludes:

- Any new domain rules
- Any write semantics changes
- Any persistence model changes
- Any audit or rollback semantic changes
- Any authorization rule changes
- UI implementation itself
- Performance optimization
- Caching or background processing

---

### Phase 11 Core Principles

- **The backend remains the sole authority**
- **The frontend may validate, but never decide**
- **All business rules live in the domain**
- **All state mutation remains auditable**
- **Read APIs may be reshaped freely**
- **No domain invariants may be bypassed**

---

### Phase 11 Active Context Model

- The system operates on **exactly one active bid year at a time**
- Only one bid year may be active in the system
- This invariant must be enforced at the domain or core level
- All operator-facing APIs implicitly or explicitly reference the active bid year
- UI must not manage multiple bid years simultaneously

---

### Phase 11 Read API Ergonomics

Phase 11 allows the introduction of **ergonomic, composite read APIs**, including but not limited to:

- Listing all users in the active bid year
- Listing users by area
- Returning users with derived data (e.g. leave availability)
- Returning area summaries including:
  - user counts
  - bid year metadata
- Returning bid year metadata alongside operational data

These APIs may:

- Join canonical tables
- Compute derived, read-only values
- Aggregate related data
- Flatten nested data for UI consumption

These APIs must NOT:

- Perform state mutation
- Emit audit events
- Depend on mutable in-memory state
- Introduce new business rules

---

### Phase 11 Leave Availability Exposure

- Leave accrual calculations from Phase 9 may be exposed via read APIs
- Leave availability must be:
  - Derived server-side
  - Deterministic
  - Explainable
- UI must not implement accrual logic
- UI may only display or pre-check values for user experience

---

### Phase 11 Frontend Validation Rules

- Frontend validation is permitted for:
  - UX feedback
  - Early error detection
- Frontend validation is NOT authoritative
- Backend validation remains final and required
- All backend validation failures must still be handled gracefully by the UI

---

### Phase 11 API Contract Rules

- API changes are explicitly allowed in this phase
- Any API change must:
  - Update request/response DTOs
  - Update server handlers
  - Update `api_cli.py`
  - Be covered by tests
- No silent or implicit API changes are permitted
- API responses should favor clarity over minimalism

---

### Phase 11 Tooling Requirements

- `api_cli.py` must be updated whenever:
  - Endpoints are added or removed
  - Request schemas change
  - Response schemas change
- Tooling drift is considered a failure condition
- CLI behavior must match the current API surface

---

### Phase 11 Testing Requirements

Tests must demonstrate:

- Correctness of new read APIs
- Deterministic derived values
- No state mutation on read paths
- Accurate leave availability exposure
- Correct handling of active bid year context
- API and CLI consistency

---

### Phase 11 Exit Criteria

Phase 11 is complete when all of the following are true:

- Operator workflows are supported by ergonomic read APIs
- Users can be listed meaningfully without manual API composition
- Leave availability is accessible via read APIs
- Active bid year context is enforced consistently
- No domain rules were added or modified
- No persistence semantics were changed
- No audit behavior was altered
- `api_cli.py` reflects the current API surface
- All read paths are fully tested
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
