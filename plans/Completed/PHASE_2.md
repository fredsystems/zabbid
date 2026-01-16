# PHASE_2.md

## Phase 2: API Skeleton & Boundary Hardening

**Goal:**
Expose the Phase 1 domain and enforcement model through a minimal API boundary without leaking domain logic or freezing contracts.

Phase 2 proves that the domain remains authoritative when driven externally.

---

### Phase 2 Objectives

- Introduce a minimal API boundary that drives core commands
- Define request and response DTOs distinct from domain types
- Translate domain and core errors into API-facing errors explicitly
- Ensure all successful API-driven transitions emit audit events
- Ensure failed API calls do not mutate state or emit audit events

---

### Phase 2 Scope

Phase 2 includes:

- A thin API layer that translates requests into core commands
- Request/response types scoped to API concerns
- Explicit error mapping from domain/core errors to API errors
- Boundary tests exercising API → core → audit paths

Phase 2 explicitly excludes:

- Persistence or database integration
- Authentication or authorization
- Stable or versioned public API contracts
- Frontend integration
- Pagination, querying, or filtering
- Bidding round modeling or round-specific logic
- Seniority ordering or tie-breaking
- Performance optimization

---

### Phase 2 Boundary Rules

- The API layer must not contain domain rules
- The API layer must not mutate state directly
- All state changes must occur via core transitions
- Domain types must not be exposed directly through the API
- Errors must be translated, not reinterpreted or hidden

---

### Phase 2 Testing Requirements

Tests must demonstrate:

- Valid API requests result in successful state transitions
- Invalid API requests result in structured API errors
- Domain rule failures propagate correctly through the API
- Failed API calls do not mutate state
- Successful API calls emit exactly one audit event
- API responses do not leak domain internals

---

### Phase 2 Exit Criteria

Phase 2 is complete when all of the following are true:

- A minimal API layer exists and compiles
- The Phase 1 representative rule can be exercised via the API
- Domain rules are not duplicated in the API layer
- Error translation is explicit and test-covered
- Audit events are emitted only on successful API calls
- No persistence, authentication, or round logic exists
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
