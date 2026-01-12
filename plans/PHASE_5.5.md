# PHASE_5.5.md

## Phase 5.5: Server Binary & Operator Interface

### Goal

Make the system runnable and observable by a human operator without introducing new domain logic, persistence semantics, or authorization rules.

Phase 5.5 exists to validate correctness through interaction, not to finalize APIs or user experience.

---

### Phase 5.5 Scope

Phase 5.5 includes:

- A real server binary that hosts the system
- HTTP endpoints for:
  - executing existing write commands
  - performing read-only queries
- Wiring of persistence, read models, and authorization
- Minimal authentication stubs sufficient to produce an Actor
- Structured JSON request and response bodies
- Logging and instrumentation suitable for inspection
- Manual and automated tests exercising end-to-end flows

Phase 5.5 explicitly excludes:

- New domain rules or validations
- UI or frontend implementation
- API stability or versioning guarantees
- Real authentication mechanism design
- Changes to audit, rollback, or snapshot semantics
- Performance optimization
- New markdown documentation beyond existing project documents

---

### Phase 5.5 Interface Philosophy

- The HTTP interface is an operator interface, not a public API
- Endpoints may be verbose, explicit, or awkward
- JSON responses should favor completeness over ergonomics
- No attempt should be made to normalize, simplify, or prettify outputs

---

### Phase 5.5 Operator Semantics

- All write requests are executed by authenticated Actors
- Actors are produced by a minimal authentication boundary
- Actor roles are limited to Admin and Bidder
- Authorization is enforced before command execution
- Authorization logic must not leak into domain or core layers

---

### Phase 5.5 Write Semantics

- All write endpoints must:
  - authenticate an Actor (stub is acceptable)
  - emit audit events on success only
- Failed requests must:
  - return structured errors
  - not emit audit events

---

### Phase 5.5 Read Semantics

- Read endpoints must be strictly side-effect free
- Reads must not depend on mutable in-memory state
- Reads must reflect persisted audit and snapshot data
- Reads must support:
  - current effective state
  - ordered audit event timelines

---

### Phase 5.5 Exit Criteria

Phase 5.5 is complete when all of the following are true:

- The server binary can be run locally
- State can be mutated via HTTP write requests
- State and audit history can be queried via HTTP
- Rollback behavior can be exercised interactively
- All interactions reflect true persisted state
- No new domain or persistence semantics were introduced
- Operator roles are enforced only at the boundary
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
