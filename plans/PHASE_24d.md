# Phase 24D — Multi-Backend Validation & Test Infrastructure

## Goal

Establish a robust, explicit testing and validation framework for multiple
database backends (SQLite + MariaDB/MySQL), without compromising correctness,
build determinism, or developer ergonomics.

This phase ensures the Diesel-based persistence layer is **truly backend-agnostic**
and that backend-specific behavior is **explicitly validated**, not assumed.

---

## Core Principles

- SQLite remains the **default backend** for:
  - development
  - unit tests
  - integration tests
- SQLite **must** support full in-memory operation
- Additional backends (MariaDB/MySQL):
  - are supported by default at compile time
  - are validated only via explicit, opt-in test runs
- No database backend is gated behind Cargo feature flags
- No test silently skips due to missing infrastructure

---

## Scope (Allowed)

### 1. Backend-Agnostic Diesel Validation

- Verify that:
  - All Diesel migrations apply cleanly on MariaDB/MySQL
  - All Diesel DSL queries compile and execute correctly
  - No backend-specific schema divergence exists
- Identify and document:
  - Backend-specific behavior differences
  - Required query adjustments that preserve semantic correctness

---

### 2. Explicit External Database Test Harness

- Introduce opt-in database-backed test execution via `xtask`
- External database tests must:
  - Be explicitly marked with `#[ignore]`
  - Fail fast if required environment variables are missing
  - Never run under `cargo test` by default

Example execution model:

```bash
cargo xtask test-mariadb
```

### 3. xtask-Orchestrated Infrastructure

Extend xtask to:

Validate required tools (Docker, client binaries, ports)

Start and stop database containers deterministically

Set required environment variables

Execute ignored tests intentionally

Docker lifecycle must not be embedded in tests

Tests must assume the database already exists and is reachable

### 4. Nix Environment Integration

Update flake.nix / devshell to include:

Docker client (if not already present)

MariaDB/MySQL client tools (for diagnostics)

Ensure direnv activation provides all required tooling

Agents must not work around missing tools in code

### 5. Backend-Specific Test Coverage (Minimal & Targeted)

Add only the tests necessary to prove:

Migrations apply cleanly

Canonical bootstrap succeeds

Core persistence invariants hold

Examples:

Fresh database bootstrap

Canonical ID creation

Foreign key enforcement

Transactional rollback behavior

Avoid duplicating the entire SQLite test suite.

Explicitly Out of Scope

No production deployment changes

No runtime backend switching logic

No feature flags or conditional compilation

No schema divergence per backend

No ORM-layer abstraction beyond Diesel

No changes to domain logic or API contracts

No CI pipeline changes unless explicitly requested

Exit Criteria

SQLite remains the default backend for all standard tests

MariaDB/MySQL backend can be validated via cargo xtask

All migrations apply cleanly on all supported backends

No Diesel DSL queries rely on SQLite-only behavior

Backend-specific behavior is documented and tested

No tests silently skip due to missing infrastructure

cargo test remains fast, deterministic, and infrastructure-free

Failure Conditions (Must Stop & Ask)

Agents must stop immediately if:

A backend requires schema divergence

Diesel DSL cannot express a required query for all backends

A test requires conditional compilation to pass

Infrastructure requirements cannot be provisioned via xtask

Backend-specific behavior threatens canonical identity guarantees

Correctness and architectural integrity take precedence over completeness.
