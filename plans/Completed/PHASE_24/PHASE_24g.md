# Phase 24G — Database Backend Selection & Server Startup Semantics

## Objective

Finalize database backend selection at **server startup**, enabling explicit selection between SQLite and MySQL/MariaDB while preserving:

- SQLite in-memory as the default developer experience
- Clear, explicit configuration
- Zero ambiguity in runtime behavior
- Clean separation between backend choice and persistence logic

**Phase 24G is about wiring and validation only.**
It must NOT introduce new persistence abstractions, traits, or backend logic.

---

## Current State

The server currently exposes:

- `--database <PATH>` for SQLite
- Default behavior: SQLite in-memory database if no path is provided
- No explicit backend selection flag
- MySQL/MariaDB support exists **only at the persistence layer**
- The server cannot currently select MySQL at runtime

Once multiple backends exist, this becomes ambiguous and unsafe.

---

## Goals

1. Make the database backend **explicit at startup**
2. Preserve SQLite in-memory as the **zero-config default**
3. Support MySQL/MariaDB **without feature flags**
4. Prevent backend-specific behavior from leaking past initialization
5. Keep configuration ergonomic and unsurprising
6. Avoid introducing architectural commitments prematurely

---

## Proposed CLI Design

### Backend Selection

Introduce an explicit backend selector:

- `--db-backend <backend>`
- Allowed values: `sqlite`, `mysql`
- Default: `sqlite`

Backend-specific flags are interpreted **only** for the selected backend.

---

### SQLite Configuration (Default)

SQLite remains the default backend.

Flags:

- `--db-backend sqlite` (optional)
- `--database <PATH>` (optional)

Behavior:

- If `--database` is **not provided** → SQLite in-memory
- If `--database` **is provided** → SQLite file-based database

Examples:

- `zab-bid-server`
- `zab-bid-server --database ./zabbid.db`
- `zab-bid-server --db-backend sqlite --database ./zabbid.db`

---

### MySQL / MariaDB Configuration

MySQL is **explicit and opt-in**.

Flags:

- `--db-backend mysql`
- `--database-url <URL>` (**required**)

Behavior:

- Startup fails if `--database-url` is missing
- No defaults
- No inference
- No fallback behavior

Example:

- `zab-bid-server --db-backend mysql --database-url mysql://user:pass@localhost/zabbid`

---

## Validation Rules (Strict)

The server MUST enforce:

- `mysql` backend **requires** `--database-url`
- `sqlite` backend **ignores** `--database-url`
- `--database` is **invalid** when `--db-backend mysql`
- Unknown backend values fail **during argument parsing**
- Validation occurs **before** persistence initialization

There must be **no runtime ambiguity**.

---

## Persistence Integration

- Backend selection happens **exactly once** at startup
- The server constructs the appropriate persistence adapter:
  - SQLite (in-memory or file)
  - MySQL (URL-based)
- All downstream code remains backend-agnostic
- No conditional backend logic outside initialization
- No duplication of persistence adapters

---

## Explicit Non-Goals

This phase MUST NOT:

- Rename or refactor persistence adapters
- Introduce traits, enums, or abstractions for backend dispatch
- Duplicate persistence structs
- Implement MySQL query or mutation logic
- Change Diesel schema or queries
- Modify migration behavior
- Add runtime MySQL usage beyond construction
- “Prepare” for Phase 24H

This phase is **wiring only**.

---

## Testing Expectations

Required:

- Argument parsing tests covering:
  - Default SQLite behavior
  - SQLite with file
  - MySQL requiring `--database-url`
  - Invalid flag combinations

Out of scope:

- End-to-end MySQL runtime tests
- Persistence correctness tests
- Schema validation tests

---

## Exit Criteria

Phase 24G is complete when:

- Server supports explicit backend selection
- SQLite in-memory remains the default
- MySQL is opt-in and strictly validated
- Startup semantics are unambiguous
- No persistence behavior changes
- No new abstractions added
- All tests and CI pass

---

## Rationale

This phase completes **startup semantics**, not persistence design:

- Phase 24C: Diesel DSL everywhere
- Phase 24D: Multi-backend validation
- Phase 24E: Guardrails and enforcement
- Phase 24F: Structural refactor
- **Phase 24G: Explicit runtime backend selection**
- Phase 24H: Actual MySQL persistence wiring

Phase 24G must leave the persistence layer untouched and boring.
