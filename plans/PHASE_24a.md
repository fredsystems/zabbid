# Phase 24A — Diesel Introduction for Schema & Migrations

## Goal

Introduce Diesel as the authoritative mechanism for database schema definition
and migration, without changing runtime persistence behavior or query logic.

This phase establishes Diesel as the **schema bootstrap layer only**.

---

## Scope (Allowed)

### Diesel Responsibilities

Diesel is introduced **only** for:

- Defining the database schema
- Managing migrations
- Bootstrapping databases at startup
- Bootstrapping databases in tests

Diesel does **not** replace existing query logic in this phase.

---

### Runtime Behavior

- The live server:
  - Uses Diesel at startup to:
    - initialize the database
    - run pending migrations
  - Continues to use existing persistence code for all queries and mutations

- Tests:
  - Use Diesel to:
    - create and migrate databases
  - Continue using existing persistence adapters for behavior testing

---

## Explicitly Out of Scope

- No query rewrites
- No domain changes
- No API changes
- No UI changes
- No audit logic changes
- No behavior refactors
- No removal of existing SQL queries
- No ORM-based runtime queries

If any of the above become necessary, **stop and ask**.

---

## Database Support Requirements

### SQLite (Required)

- Diesel must support:
  - in-memory SQLite databases (`:memory:`)
  - migration execution against in-memory databases
- Test infrastructure must not persist databases to disk by default

### Future Databases (Non-Blocking)

- Schema definitions must not prevent later use of:
  - PostgreSQL
  - MariaDB / MySQL
- Cross-database compatibility is a **future concern**, not a requirement here

---

## Required Work

### 1. Diesel Setup

- Add Diesel to the workspace
- Configure Diesel with SQLite backend
- Add diesel_cli configuration
- Define schema via Diesel migrations

---

### 2. Migration Parity

- Create Diesel migrations that match the **current canonical schema**
- Migrations must:
  - Produce the same tables
  - Preserve constraints
  - Preserve foreign keys
  - Preserve nullable semantics
- No schema redesign in this phase

---

### 3. Server Bootstrap Integration

- On server startup:
  - Initialize a Diesel connection
  - Run pending migrations
  - Hand off the connection / database to existing persistence layer

Diesel must not leak past startup boundaries.

---

### 4. Test Infrastructure Integration

- Update test persistence setup to:
  - Initialize an in-memory SQLite database via Diesel
  - Run migrations
  - Hand the database to existing test adapters

Tests must remain:

- deterministic
- isolated
- fast
- disk-free by default

---

### 5. Guardrails

- Diesel-generated structs must:
  - remain private to the persistence layer
  - never cross into domain or API layers
- No Diesel macros or types in:
  - domain
  - core
  - API
  - UI

---

## Exit Criteria

- Server starts with Diesel-managed migrations
- Tests use Diesel-managed in-memory databases
- No runtime behavior changes
- No query rewrites
- No test artifacts persisted to disk
- All tests pass
- Existing persistence code remains intact

---

## Non-Goals (Reaffirmed)

This phase does **not**:

- Make Diesel the query layer
- Introduce ORM models into the domain
- Simplify persistence logic
- Remove SQL from the codebase

Those decisions belong to later phases.
