# Phase 24B — Diesel as the Sole Persistence Layer

## Goal

Remove `rusqlite` entirely and make Diesel the **only** database interaction layer used at runtime and in tests, while preserving all existing behavior and correctness.

This phase is about **consolidation and correctness**, not refactoring query logic yet.

---

## Core Principle

After Phase 24B:

- **All database connections, transactions, and query execution go through Diesel**
- No code paths may use `rusqlite`
- Raw SQL is temporarily permitted, but only via Diesel
- The observable behavior of the system must remain unchanged

---

## Scope (Allowed)

### 1. Remove rusqlite

- Remove `rusqlite` from:
  - workspace dependencies
  - persistence crate
  - test helpers
- Replace all `rusqlite::Connection` usage with:
  - `diesel::SqliteConnection`
- Update persistence interfaces accordingly

---

### 2. Connection & Transaction Management

- All persistence entry points must:
  - accept or construct a `diesel::SqliteConnection`
  - use Diesel-managed transactions
- Ensure transactional semantics are preserved exactly:
  - atomic writes
  - rollback behavior
  - error propagation

---

### 3. Runtime Queries (Transitional)

- Existing SQL queries may be retained **verbatim**, but must:
  - be executed via Diesel (`sql_query`)
  - live behind existing persistence interfaces
- No query rewrites are required in this phase

This is a **mechanical migration**, not a semantic one.

---

### 4. Test Infrastructure

- All tests must:
  - use Diesel-backed connections
  - continue to support in-memory SQLite databases
- No on-disk database artifacts may be introduced
- Test isolation must remain intact

---

### 5. Schema Source of Truth

- Diesel migrations and generated `schema.rs` are authoritative
- No hand-maintained schema duplication is allowed
- All runtime code must align with Diesel’s schema definitions

---

## Explicitly Out of Scope

- ❌ Rewriting queries into Diesel DSL
- ❌ Query optimization or cleanup
- ❌ Domain, core, or API changes
- ❌ UI changes
- ❌ New features
- ❌ Behavior changes
- ❌ Performance tuning

If any of the above become necessary, **stop and ask**.

---

## Constraints

- No sentinel values
- No fallback logic
- No weakening of invariants
- No silent error swallowing
- No partial migrations

If something breaks, it must break loudly and honestly.

---

## Exit Criteria

Phase 24B is complete when:

- `rusqlite` is completely removed from the workspace
- All database access goes through Diesel
- All tests pass using Diesel-backed SQLite
- No behavioral changes are observed
- Diesel migrations + schema.rs are the single schema authority
- `cargo xtask ci` and `pre-commit run --all-files` pass cleanly

---

## Rationale

This phase creates a **single, coherent persistence foundation**:

- Phase 24A: Diesel defines schema & migrations
- Phase 24B: Diesel owns all database access
- Phase 24C: Queries migrate to Diesel DSL safely and incrementally

Skipping this step would entangle query refactors with connection churn,
making correctness harder to reason about.

This phase keeps the blast radius contained.
