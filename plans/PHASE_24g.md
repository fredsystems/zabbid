# Phase 24G — Database Backend Selection & Server Startup Semantics

## Objective

Finalize database backend selection at server startup, enabling explicit selection between SQLite and MySQL/MariaDB while preserving:

- SQLite in-memory as the default developer experience
- Clear, explicit configuration
- Zero ambiguity in runtime behavior
- Clean separation between backend choice and persistence logic

Phase 24G is about wiring, not new persistence behavior.

## Current State

The server currently exposes:

- `--database <PATH>` for SQLite
- Default behavior: SQLite in-memory database if no path is provided
- No explicit backend selection flag
- MySQL/MariaDB support exists at the persistence layer but is not selectable from the server CLI

This becomes ambiguous once multiple backends exist.

## Goals

1. Make the database backend explicit
2. Preserve SQLite in-memory as the zero-config default
3. Support MySQL/MariaDB without feature flags
4. Avoid backend-specific behavior leaking into application logic
5. Keep configuration ergonomic and unsurprising

## Proposed CLI Design

### Backend Selection

Introduce explicit backend selection:

- `--db-backend <backend>`
- Allowed values: `sqlite`, `mysql`
- Default: `sqlite`

Backend-specific options are interpreted only for the selected backend.

### SQLite Configuration

SQLite remains the default backend.

Options:

- `--db-backend sqlite` (optional)
- `--database <PATH>` (optional)

Behavior:

- If `--database` is not provided, use SQLite in-memory
- If `--database` is provided, use SQLite at the given path

Examples:

- `zab-bid-server`
- `zab-bid-server --database ./zabbid.db`
- `zab-bid-server --db-backend sqlite --database ./zabbid.db`

### MySQL / MariaDB Configuration

MySQL is explicit and opt-in.

Options:

- `--db-backend mysql`
- `--database-url <URL>` (required)

Behavior:

- Startup fails if `--database-url` is missing
- No defaults and no inference

Example:

- `zab-bid-server --db-backend mysql --database-url mysql://user:pass@localhost/zabbid`

## Validation Rules

The server must enforce:

- `mysql` backend requires `--database-url`
- `sqlite` backend ignores `--database-url`
- `--database` is invalid for `mysql`
- Unknown backend values fail during argument parsing

All validation occurs before persistence initialization.

## Persistence Integration

- Backend selection happens once at startup
- The server constructs the appropriate persistence backend:
  - SQLite (in-memory or file)
  - MySQL (URL-based)
- All downstream code remains backend-agnostic
- No conditional backend logic outside initialization

## Non-Goals

This phase does not include:

- Migrating production to MySQL
- Running MySQL by default
- Adding feature flags
- Changing Diesel schema or queries
- Adding new persistence behavior
- Adding new test suites beyond wiring validation

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

## Exit Criteria

Phase 24G is complete when:

- Server supports explicit backend selection
- SQLite in-memory remains the default
- MySQL is opt-in and validated
- Startup semantics are unambiguous
- No behavior changes outside initialization
- All tests and CI pass

## Rationale

This phase completes the backend-agnostic persistence work:

- Phase 24C: Diesel DSL everywhere
- Phase 24D: Multi-backend validation
- Phase 24E: Guardrails and enforcement
- Phase 24F: Structural refactor
- Phase 24G: Explicit runtime backend selection

After Phase 24G, the system is structurally ready for real MySQL deployment with no architectural debt.
