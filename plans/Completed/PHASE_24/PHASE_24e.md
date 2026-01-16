# Phase 24E — Migration Guardrails & Schema Parity Enforcement

## Goal

Harden the multi-backend database architecture by:

1. Making backend-specific migration constraints **explicit and unavoidable**
2. Enforcing **schema parity** between SQLite and MySQL/MariaDB at the tooling level
3. Preventing silent divergence through **automated xtask checks**
4. Updating project documentation and agent rules to encode these guardrails permanently

This phase is about **preventing future correctness drift**, not adding features.

---

## Scope (Allowed)

### Documentation & Rules

- Update **AGENTS.md** to:
  - Explicitly document backend-specific migration constraints
  - Prohibit silent schema divergence
  - Require xtask enforcement for external database parity
- Add **inline code comments** in:
  - Diesel migration runners
  - MySQL initialization module
  - xtask commands
- Update **README.md** _only if necessary_ to explain:
  - How database backends are validated
  - How to run backend-specific tests

---

### Tooling (xtask)

- Add new xtask command(s) to enforce migration correctness:
  - `cargo xtask verify-migrations`
- xtask is the **only** place allowed to:
  - Spin up external databases
  - Apply migrations for validation
  - Compare schemas across backends

---

## Explicitly Out of Scope

- No domain logic changes
- No API changes
- No persistence behavior changes
- No schema changes
- No query changes
- No UI changes
- No feature flags
- No conditional compilation

If any of the above become necessary, **stop and ask**.

---

## Required Work

### 1. Documentation Guardrails

#### AGENTS.md Updates (Required)

Add a new section under **Database Tooling** or **Persistence Rules** that states:

- Migrations **may be backend-specific**
- Backend-specific migrations **must remain semantically equivalent**
- Schema equivalence **must be enforced by tooling**, not convention
- Agents must NOT:
  - Assume SQLite migrations will work on MySQL
  - Modify schema to “make tests pass”
  - Introduce backend-specific hacks without parity checks
- Any backend-specific behavior must be:
  - Documented
  - Tested
  - Explicitly enforced via xtask

---

#### Inline Code Warnings (Required)

Add clear warnings in:

- `diesel_migrations.rs`
- `mysql/mod.rs`

Warnings must explain:

- Why separate migration directories exist
- That they **must remain schema-equivalent**
- That xtask verification exists and is mandatory

---

### 2. xtask: Schema Parity Verification

#### New Command: `cargo xtask verify-migrations`

This command must:

1. **Provision ephemeral databases**
   - SQLite (in-memory)
   - MariaDB/MySQL (Docker, same pattern as `test-mariadb`)
2. **Apply migrations**
   - SQLite → `migrations/`
   - MySQL → `migrations_mysql/`
3. **Introspect resulting schemas**
   - Tables
   - Columns
   - Types (normalized)
   - Nullability
   - Primary keys
   - Foreign keys
   - Unique constraints
4. **Normalize backend differences**
   - INTEGER vs BIGINT → i64
   - TEXT vs VARCHAR → string
   - TINYINT vs INTEGER → bool
5. **Compare schemas structurally**
6. **Fail hard** if any mismatch exists

This command must:

- Be deterministic
- Fail loudly
- Clean up all external resources
- Require explicit opt-in (never run implicitly)

---

### 3. CI / Workflow Integration

- `cargo xtask ci` must:
  - Continue to run SQLite-only tests by default
  - NOT run MySQL tests automatically
- `verify-migrations` must be:
  - Easy to run locally
  - Documented as required before schema changes
- No CI environment assumptions may be baked into tests

---

## Exit Criteria

- AGENTS.md updated with explicit migration and parity rules
- Clear inline warnings added to backend initialization code
- `cargo xtask verify-migrations` exists and works
- Schema mismatches between SQLite and MySQL cause hard failures
- No runtime behavior changes
- No test behavior changes
- All existing tests still pass
- All tooling checks pass (`cargo xtask ci`, `pre-commit`)

---

## Design Principles Reinforced

- Tooling enforces correctness — humans do not
- Backends are interchangeable only if proven equivalent
- Drift is a correctness bug, not a documentation issue
- xtask is the single authority for infrastructure orchestration

---

## When to Stop

Stop and ask if:

- Schema normalization rules are unclear
- Diesel introspection cannot express required constraints
- Backend differences appear unresolvable without schema changes
- The agent is tempted to “make it work” by relaxing rules

Correctness > convenience.
