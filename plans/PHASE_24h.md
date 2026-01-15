# Phase 24H — MySQL Persistence Backend Integration

## Objective

Complete MySQL/MariaDB support at the persistence layer by wiring the existing
MySQL infrastructure into the persistence adapter, while preserving:

- A single, canonical persistence API
- No duplication of query or mutation logic
- Backend-agnostic application and server code
- Identical persistence semantics across backends

Phase 24H is about **connection plumbing and adapter generalization**, not
rewriting queries or changing behavior.

---

## Current State

- SQLite persistence is fully implemented and battle-tested
- MySQL infrastructure exists:
  - Connection initialization
  - Migrations
  - Foreign key enforcement checks
- Server startup can explicitly select `mysql` backend (Phase 24G)
- MySQL is not yet usable at runtime
- Persistence adapter is SQLite-specific (`SqlitePersistence`)

---

## Goals

1. Generalize the persistence adapter so it is backend-agnostic
2. Avoid duplicating persistence APIs per backend
3. Reuse all existing query and mutation modules unchanged
4. Keep backend-specific logic fully isolated
5. Preserve existing SQLite behavior exactly
6. Make MySQL functionally equivalent to SQLite at the persistence boundary

---

## Design Principles

- There must be **exactly one persistence adapter type** exposed to the server
- Backend selection happens once, at construction time
- Queries and mutations must not know which backend they are using
- Backend-specific details must not leak into application logic
- No conditional logic scattered across call sites

---

## Proposed Architecture

### Persistence Adapter

Introduce a backend-agnostic adapter, conceptually:

- `Persistence` (or equivalent neutral name)
- Internally owns a backend-specific connection
- Public API remains identical to the current SQLite adapter

### Backend Isolation

Backend-specific responsibilities are limited to:

- Establishing a database connection
- Running migrations
- Verifying backend-specific invariants (e.g. foreign keys)

These live exclusively in `backend::{sqlite, mysql}` modules.

### Dispatch Strategy

Persistence methods should delegate internally based on the selected backend,
using one of the following (final choice left to implementation):

- A small internal enum wrapping backend connections
- A private trait implemented by backend connection wrappers

**Constraints:**

- No duplicated persistence structs
- No duplicated query or mutation functions
- No Diesel queries rewritten

---

## Implementation Scope

### In Scope

- Introduce a backend-agnostic persistence adapter
- Wire MySQL connections into that adapter
- Ensure all existing persistence methods work identically for SQLite
- Make MySQL backend fully operational using existing Diesel DSL queries
- Update server startup to construct the unified adapter for MySQL
- Add minimal wiring validation tests if necessary

### Explicitly Out of Scope

- Schema changes
- Query or mutation rewrites
- Performance optimizations
- Connection pooling
- Production MySQL tuning
- New CLI flags
- Feature flags
- Multi-backend runtime switching

---

## Testing Expectations

Required:

- SQLite tests must continue to pass unchanged
- MySQL backend must pass existing persistence tests where applicable
- Failures must indicate missing wiring, not altered behavior

Not Required:

- New MySQL-specific behavioral tests
- Load or stress testing
- End-to-end production validation

---

## Exit Criteria

Phase 24H is complete when:

- The server can start successfully with `--db-backend mysql`
- The persistence adapter works identically for SQLite and MySQL
- All existing persistence APIs remain unchanged
- No code duplication exists between backends
- No backend-specific logic leaks into application code
- All tests and CI pass

---

## Rationale

Phase 24H completes the multi-backend persistence work:

- Phase 24C: Diesel DSL everywhere
- Phase 24D: Multi-backend validation
- Phase 24E: Migration guardrails
- Phase 24F: Structural refactor
- Phase 24G: Explicit runtime backend selection
- Phase 24H: MySQL backend integration

After Phase 24H, the persistence layer is structurally complete and
backend-agnostic, allowing future system architecture work to proceed
without database-related risk.
