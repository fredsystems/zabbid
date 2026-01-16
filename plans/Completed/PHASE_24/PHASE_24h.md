# Phase 24H — MySQL Persistence Backend Integration

## Objective

Complete MySQL/MariaDB support at the persistence layer by wiring the existing
MySQL infrastructure into the persistence adapter, while preserving:

- A single, canonical persistence API
- Backend-agnostic application and server code
- Identical persistence semantics across backends
- Zero behavioral changes

Phase 24H is about **adapter wiring and backend dispatch**,
**not** about rewriting queries, generalizing Diesel, or changing persistence behavior.

---

## Current State

- SQLite persistence is fully implemented and battle-tested
- MySQL infrastructure exists:
  - Connection initialization
  - Migrations
  - Foreign key enforcement checks
- Server startup can explicitly select `mysql` backend (Phase 24G)
- MySQL is not yet usable at runtime
- Persistence adapter is currently SQLite-specific (`SqlitePersistence`)

---

## Goals

1. Expose **one backend-agnostic persistence adapter** to the server
2. Enable MySQL/MariaDB to function at parity with SQLite
3. Preserve **all existing persistence APIs**
4. Keep backend-specific logic fully isolated
5. Avoid architectural debt that would require later refactors
6. Make Phase 24I+ independent of database concerns

---

## Design Principles

- There must be **exactly one persistence adapter type**
- Backend selection happens **once**, at construction time
- Application and server code must not branch on backend type
- Backend-specific details must not leak beyond the adapter
- No conditional backend logic scattered across call sites
- Correctness > abstraction cleverness

---

## Diesel Backend Constraint (Critical)

Diesel’s type system requires **concrete backend types at compile time**.

As a result:

- Query and mutation functions **MUST take concrete Diesel connection types**
  - e.g. `&mut SqliteConnection`, `&mut MysqlConnection`
- Query/mutation modules **MUST NOT be generic over backend traits**
- Diesel DSL usage **MUST remain monomorphic**
- Backend abstraction **MUST occur only at the persistence adapter boundary**

Backend-agnostic behavior is achieved by:

- Centralizing backend selection and dispatch in the adapter
- Calling backend-specific query/mutation functions from explicit match arms

---

## Proposed Architecture

### Public Adapter

Introduce a backend-agnostic adapter:

- `Persistence` (neutral name)
- Public API matches the existing `SqlitePersistence` API exactly
- Internally owns one backend connection

### Internal Backend Wrapper

Use a private enum to hold the concrete connection type:

- `enum BackendConn { Sqlite(SqliteConnection), Mysql(MysqlConnection) }`

Dispatch occurs inside `impl Persistence` via `match &mut self.conn`.

### Backend-Specific Responsibilities (Isolated)

Backend-specific modules are limited to:

- Opening a connection
- Running migrations
- Enforcing/verifying backend invariants (FK enforcement, etc.)

These remain exclusively in:

- `backend::sqlite`
- `backend::mysql`

---

## Implementation Scope

### In Scope

- Rename or replace `SqlitePersistence` with a neutral adapter (`Persistence`)
- Add constructors:
  - `Persistence::new_sqlite_in_memory()`
  - `Persistence::new_sqlite_file(path)`
  - `Persistence::new_mysql(database_url)`
- Wire MySQL connection initialization and migration runner into the adapter
- Implement adapter dispatch for all persistence methods:
  - Methods route to SQLite or MySQL implementations using `match`
- Preserve all SQLite behavior exactly
- Ensure the server can actually run using `--db-backend mysql`
- Add minimal wiring validation tests if needed

### Explicitly Out of Scope

- Schema changes
- DSL query rewrites beyond what is required to compile for MySQL
- Making query/mutation code generic via traits
- Performance tuning or pooling
- Production hardening for MySQL
- New CLI flags (Phase 24G already handled this)
- Feature flags
- Multi-backend runtime switching

---

## Testing Expectations

Required:

- All existing SQLite tests continue to pass unchanged
- MySQL backend can run the same persistence test suite where applicable
- Failures must indicate missing wiring or backend incompatibility, not altered behavior

Not Required:

- New MySQL-only behavioral tests
- Load testing or stress testing
- End-to-end production validation

---

## Exit Criteria

Phase 24H is complete when:

- `zab-bid-server --db-backend mysql --database-url ...` starts successfully
- The persistence adapter works for both SQLite and MySQL
- Existing persistence APIs remain unchanged
- Backend-specific logic remains isolated in `backend::*`
- No query/mutation code is made generic over backend traits
- No duplicated “SQLitePersistence vs MySQLPersistence” adapter structs exist
- All tests and CI pass

---

## Rationale

Phase 24H completes the multi-backend persistence work:

- Phase 24C: Diesel DSL everywhere
- Phase 24D: Multi-backend validation
- Phase 24E: Migration guardrails & parity enforcement
- Phase 24F: Persistence crate structural refactor
- Phase 24G: Explicit runtime backend selection
- Phase 24H: MySQL backend integration

After Phase 24H, the persistence layer is structurally complete and
backend-agnostic at the application boundary, allowing future system architecture
work to proceed without database-related risk.
