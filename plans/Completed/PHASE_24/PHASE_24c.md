# Phase 24C — Diesel Query Migration (Eliminate Raw SQL)

## Goal

Complete the Diesel migration by replacing **all runtime raw SQL queries**
with Diesel’s typed query builder **where feasible**, while preserving
existing behavior, constraints, and domain rules exactly.

This phase removes SQL strings from the persistence layer and makes Diesel
the **single source of truth** for query construction.

---

## Scope (Allowed)

### Query Migration

- Replace raw SQL in persistence with Diesel query builder equivalents:
  - `select`
  - `filter`
  - `inner_join` / `left_join`
  - `count`
  - `insert_into`
  - `update`
  - `delete`
- Maintain **identical semantics**:
  - same WHERE clauses
  - same ordering
  - same joins
  - same NULL behavior
- Continue using existing schema as defined by Diesel migrations

---

### Diesel Schema Usage

- Use Diesel-generated `schema.rs` exclusively
- No hand-written table definitions
- No duplicated column names or aliases outside Diesel’s DSL

---

### Transactions

- Preserve all transactional boundaries
- Use Diesel transactions (`conn.transaction(|| { ... })`)
- No change to rollback or error semantics

---

### Test Compatibility

- All existing tests must pass without modification to test intent
- In-memory SQLite must continue to work for all tests
- No persistence artifacts left on disk

---

## Explicitly Out of Scope

- ❌ Domain logic changes
- ❌ API contract changes
- ❌ UI changes
- ❌ Schema changes
- ❌ Performance optimizations
- ❌ Query behavior changes
- ❌ Silent fallbacks to raw SQL

If any of the above become necessary, **stop and ask**.

---

## Raw SQL Escape Hatch (Strictly Limited)

Raw SQL (`diesel::sql_query`) is allowed **only if**:

- Diesel DSL cannot express the query _without contortions_
- The query is clearly documented with:
  - why Diesel DSL is insufficient
  - what guarantees the raw SQL relies on
- Usage is isolated and minimal

This is an exception, not the default.

---

## Required Work

### 1. Query Inventory

- Identify all remaining raw SQL usage
- Categorize by:
  - simple select
  - joins
  - aggregates
  - complex conditional logic

---

### 2. Incremental Migration

- Migrate queries file-by-file:
  - `queries.rs`
  - `operators.rs`
  - `bootstrap.rs`
- Each file:
  - converted fully
  - compiled
  - tests pass before moving on

---

### 3. Projection Correctness

- Ensure all struct projections align with:
  - selected columns
  - aliases (if any)
- Prefer explicit `.select((col1, col2, ...))`
- No reliance on implicit column order

---

### 4. Error Mapping

- Ensure Diesel errors map cleanly to existing persistence errors
- No loss of error specificity

---

## Exit Criteria

- No raw SQL used in runtime persistence code (except documented exceptions)
- Diesel query builder used everywhere possible
- All tests pass
- No behavior changes
- No performance regressions introduced
- Diesel is now the **authoritative query layer**

---

## Result

After Phase 24C:

- Schema: Diesel migrations
- Queries: Diesel DSL
- Persistence: Diesel only
- SQLite becomes an implementation detail
- Future database backends are feasible without refactors
