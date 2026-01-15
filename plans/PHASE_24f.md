# Phase 24F — Persistence Crate Structural Refactor

## Goal

Refactor the internal structure of the `persistence` crate so that **backend choice (SQLite, MariaDB/MySQL)** is an implementation detail rather than a structural axis.

This phase is about **architecture, boundaries, and clarity**, not new functionality.

---

## Problem Statement

The persistence crate currently encodes backend identity (e.g. `sqlite`) into
module paths and file organization, even though:

- Most queries are now Diesel DSL
- Query logic is backend-agnostic
- MariaDB support exists and should reuse the same logic

This structural mismatch will cause duplication, drift, or conditional logic
when additional backends are fully wired in.

---

## Scope (Allowed)

### Structural Refactor Only

Agents may:

- Move files
- Rename modules
- Split large files by responsibility
- Introduce new internal modules
- Adjust imports and visibility
- Re-export items for compatibility

### Backend Isolation

- Backend-specific code **must** be isolated to clearly named modules
- Backend-agnostic logic **must not** live under backend-named paths

---

## Explicitly Out of Scope

- ❌ No schema changes
- ❌ No Diesel query rewrites
- ❌ No logic changes
- ❌ No behavioral changes
- ❌ No new database features
- ❌ No MariaDB runtime wiring changes
- ❌ No test behavior changes

If any of the above become necessary, **stop and ask**.

---

## Target Structural Shape (Guidance)

The persistence crate should resemble:

```text
crates/persistence/src/
├── lib.rs
├── error.rs
├── diesel_schema.rs
├── migrations/
├── migrations_mysql/
│
├── backend/
│ ├── mod.rs
│ ├── sqlite.rs
│ └── mysql.rs
│
├── queries/
│ ├── mod.rs
│ ├── canonical.rs
│ ├── audit.rs
│ ├── state.rs
│ ├── operators.rs
│ └── completeness.rs
│
├── mutations/
│ ├── mod.rs
│ ├── users.rs
│ ├── areas.rs
│ ├── bid_years.rs
│ └── audit.rs
│
└── bootstrap/
├── mod.rs
└── bootstrap.rs
```

This is **guidance**, not a rigid requirement, but the final structure must:

- Remove backend names from query paths
- Make backend-specific code explicit and minimal
- Allow MariaDB to reuse all queries unchanged

---

## Required Work

### 1. Eliminate Backend-Named Query Paths

- Remove or deprecate `src/sqlite/*` as a structural namespace
- Queries and mutations must live in backend-agnostic modules

---

### 2. Isolate Backend-Specific Code

Backend-specific logic must be limited to:

- Connection initialization
- Migration execution
- Backend configuration (PRAGMA, engine checks)
- Backend validation helpers

This code must live in explicit backend modules (e.g. `backend/sqlite.rs`).

---

### 3. Improve Responsibility Separation

Split large files where appropriate so that:

- Queries are grouped by _what they do_, not _how they run_
- Bootstrap logic is clearly separated from runtime persistence
- Operator/session persistence is isolated from domain entities

---

### 4. Preserve External API

- Public interfaces of the persistence crate must remain unchanged
- Tests must not require modification
- Callers must not be aware of the refactor

---

## Exit Criteria

- No backend names appear in query module paths
- Backend-specific code is clearly isolated and minimal
- MariaDB backend can reuse all query and mutation logic
- All tests pass unchanged
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes

---

## Success Definition

After Phase 24F:

- Backend choice is an implementation detail
- The persistence crate reflects _domain responsibilities_, not engines
- Adding or fully wiring a new backend does not require copying queries
- The structure matches the architectural reality of the code
