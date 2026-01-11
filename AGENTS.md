# AGENTS.md

## Project Objective

This project exists to:

- Provide the API for the front end to interface with the data
- All data decisions are made by the back end. The front end is just a UI.
- Data integrity is PARAMOUNT. There are strict rules governing leave and days-off bidding.
- The system must be able to validate these rules and provide a complete, auditable trail for every state-changing action.

All changes must advance these goals. If unsure, stop and ask.

## Non-Negotiable Rules

- No unsafe code unless explicitly requested
- Prefer clarity over cleverness
- No public APIs without tests
- No breaking changes without migration notes
- All code paths must have corresponding tests, including error cases
- Correctness and auditability always take precedence over performance

## Architectural Constraints

- Core logic must remain UI-agnostic
- Side effects must be isolated
- All state transitions must be explicit and observable
- Domain rules must have a single authoritative implementation (no duplication)

## Development Environment & Tooling

- The development environment is declaratively defined using Nix
- Missing tools, libraries, or services indicate an incomplete environment, not broken code
- Agents must not work around missing dependencies by modifying code
- If additional system dependencies or services are required (e.g. databases),
  the correct action is to update the Nix environment or ask for intervention
- Environment-related failures must not trigger refactors or logic changes

## API changes

- `api_cli.py` must be updated whenever:
  - API endpoints change
  - Request schemas change
  - Response schemas change
- CLI updates are considered **required**, not optional
- CLI behavior must remain aligned with the current API surface
- CLI drift is considered a correctness failure, not a tooling issue

## Audit & Data Integrity Rules

- All state changes must be attributable to an actor and a cause
- Historical records must be immutable once written
- Mutations must be additive (no silent overwrites or deletes)
- Validation failures must be explicit and surfaced as structured errors

## Code Style

- Rust: idiomatic, clippy-clean, no `.unwrap()` in library code
- Prefer small, composable functions
- Avoid macros unless justified
- Document public functions, methods, types, and non-obvious logic
- Prefer explicit type annotations for variables, fields, and function signatures; do not rely on type inference for clarity
- Use consistent naming conventions and formatting
- Follow Rust naming conventions for variables, functions, and types
- All dependencies must be declared in the root workspace Cargo.toml.
- Member crates must reference dependencies exclusively via `<package>.workspace = true`.
- Do not specify versions, overrides, or duplicate dependency entries in member crates.
- All markdown must comply with existing rules enforced by `cargo xtask ci`
- All markdown must comply with existing rules enforced by `pre-commit run --all-files`
- Before completing a phase, or any work, ensure git add has been run on all of the modified files, and `cargo xtask ci` and `pre-commit run --all-files` pass without errors.

## Workspace structure

- The workspace may be expanded with new member crates when doing so meaningfully reduces complexity or clarifies responsibility
- Each workspace member must have a clearly defined, limited focus
- New crates should exist to enforce boundaries, not to speculate about future needs
- Do not create crates “just in case” or for hypothetical reuse
- If the need for a new crate is unclear, prefer extending an existing one and ask before splitting
- Crate boundaries are architectural decisions and should be treated as such
- Adding new files should invoke git add <file name> so that pre-commit will pick it up.

## Testing Infrastructure

- Some tests may require external infrastructure (e.g. databases)
- Such requirements must be explicit and deterministic
- Test infrastructure should be provisioned via the development environment, not ad-hoc setup
- If required infrastructure is unavailable, stop and ask rather than altering behavior

## AI-Specific Rules

- Do NOT invent APIs
- Do NOT silently change behavior
- Do NOT infer or assume domain rules not explicitly defined
- If requirements conflict, call it out explicitly

## Documentation Rules

- Do NOT create new markdown files by default
- Documentation must be created only when it serves a clear, durable purpose
- Summaries, restatements, or intermediate reasoning must NOT be written to files
- If documentation seems useful, propose it first and wait for approval
- Avoid duplicating information already present in existing documents
- Do NOT create files whose sole purpose is summarization, restatement, or planning notes
- New markdown files are allowed only for:
  - Canonical project documents (e.g. README.md, AGENTS.md, PLAN.md)
  - Formal design decisions explicitly requested by the user
  - User-facing documentation explicitly approved
- All markdown must comply with existing markdownlint rules enforced by `cargo xtask ci`
- All markdown must comply with existing markdownlint rules enforced by `pre-commit run --all-files`
- Creating or modifying markdown is a higher-friction action than writing code or tests

## Domain Invariants

### Users

- Users are scoped to a single bid year
- A user’s initials are the sole identifier for that user within a bid year
- User names are informational and are not unique
- A user must belong to exactly one area
- A user must belong to exactly one crew

### Crews

- Crews are a fixed, global set of predefined scheduling groups
- Exactly seven crews exist, identified by numbers 1 through 7
- Each crew has a predefined RDO pattern
- Crews are domain constants and are not created, modified, or deleted
- Crews are not persisted as mutable data
- A user may have zero or one crew assignment at any given time
- Crew assignment is a state transition and must be explicitly audited

### Seniority Data

- Seniority-related fields exist as domain data
- Seniority inputs include:
  - cumulative NATCA bargaining unit time
  - NATCA bargaining unit time
  - EOD / FAA date
  - service computation date (SCD)
  - optional lottery value
- Seniority data must not be used for ordering, ranking, or decision-making unless explicitly enabled by a later phase

### Seniority Constraints

- Seniority-related fields are inputs, not behavior
- No seniority comparison, ranking, or tie-breaking logic may be implemented in Phase 1
- The presence of seniority data must not imply ordering or priority without an explicit rule

## Logging & Instrumentation

- Logging and instrumentation must use the `tracing` crate
- Use appropriate `tracing` macros (`trace!`, `debug!`, `info!`, `warn!`, `error!`)
- Logging configuration must respect environment-based filtering
  (e.g. `RUST_LOG` via `tracing-subscriber::EnvFilter`)
- Do NOT use `println!`, `eprintln!`, or ad-hoc logging
- Instrumentation must not affect program logic or control flow
- API-facing errors must be derived from domain/core errors, not replace them

## Error Handling

- Domain and core errors must be expressed as explicit, typed enums
- Errors must carry structured, testable information
- Do NOT use `anyhow::Error` in domain, core, or API layers
- `anyhow` may be used only in binaries, tooling, or top-level application glue
- Error context must not replace structured error variants

## Persistence & State Derivation

- The audit log is append-only and is never rewritten or deleted
- All state is derived from the ordered audit log
- Rollback is modeled as an explicit, auditable event
- Rollback does not erase or modify prior audit events
- Rollback selects a prior effective state and establishes it as authoritative going forward
- Audit events are scoped to a single bid year and a single area

## Canonical State vs Derived State

The system distinguishes clearly between **canonical operational state** and **derived historical state**.

### Canonical Operational State

- Canonical state represents the **current, authoritative data** of the system
- Canonical state is stored in **explicit relational tables** (e.g. users, areas, bid years, bids)
- Canonical state is the source of truth for:
  - current users
  - current areas
  - current bids
  - any other “what exists right now” queries
- Canonical state is mutated **only** via core state transitions
- Canonical state must be:
  - directly queryable
  - human-readable
  - validated by domain rules
  - transactionally consistent

Read-only APIs that expose current data (e.g. `/users`, `/areas`, `/bid_years`)
**must query canonical tables**, not snapshots or audit logs.

---

### Derived Historical State

- Derived state exists to support:
  - historical inspection
  - rollback semantics
  - time-based reconstruction
- Derived state is computed from:
  - the ordered audit log
  - optional persisted snapshots
- Derived state is **never authoritative on its own**
- Derived state must never be mutated directly
- Derived state may be discarded and recomputed at any time

Snapshots are **derived artifacts**, not primary storage.

---

### Snapshots

- Snapshots are serialized representations of **canonical state at a specific audit event**
- Snapshots exist solely to accelerate historical reconstruction
- Snapshots:
  - are not queried directly for current state
  - must not be treated as canonical storage
  - may be replaced, regenerated, or discarded
- Snapshots must reflect the canonical state **as it existed at the associated event**

---

### Audit Log Relationship

- The audit log records **what happened**, not canonical data models
- Audit events describe actions, actors, causes, and ordering
- Audit events must not be relied upon as a substitute for canonical tables
- Canonical state + audit log together define system correctness

---

### Prohibited Patterns

Agents must NOT:

- Treat snapshots as primary storage
- Derive current state by replaying audit events unless explicitly required
- Query snapshots to answer “current state” APIs
- Encode domain data models exclusively inside audit events

## State Snapshots

- State is conceptually a complete, materialized state per (bid year, area)
- Snapshots exist only to accelerate recovery and replay
- Snapshots must not alter the meaning of the audit log
- Full state snapshots must be persisted at:
  - rollback events
  - round finalized events
  - explicit checkpoint events
- All other audit events persist deltas only

## Actors & Roles

- Actors are authenticated system operators who execute commands
- Actors are distinct from domain users whose data is being managed
- Roles apply only to actors, never to domain users

### Roles

#### Admin

Admins are system operators with structural and corrective authority.

Admins may perform:

- creation and modification of bid years
- creation and modification of areas
- creation and modification of users
- rollback operations
- checkpoint creation
- round finalization and similar milestone actions
- any other system-level or corrective actions

#### Bidder

Bidders are system operators authorized to perform bidding actions.

Bidders may:

- enter new bids
- modify existing bids
- withdraw or correct bids
- perform bidding actions on behalf of any domain user

Bidders are not the same entities as domain users.
They act as trusted operators entering data provided by many users.

### Authorization Boundary

- Authorization is enforced before command execution
- Domain logic must not inspect actor roles
- Core state transitions must be role-agnostic
- All state-changing actions must record the acting actor in the audit log

## Module & File Structure Rules

- `lib.rs` files must be small and act as module indices only
- Substantial logic must not live directly in `lib.rs`
- Files should be split by domain responsibility, not by layer or convenience

### File Size Guidance

- If a file exceeds ~300–500 lines of non-test code, it should be split
- Test code should not exceed logic code within the same file
- Large test suites must live in a `tests/` submodule

### Test Organization

- Tests should be grouped by behavior or invariant
- Prefer `crate/tests/*.rs` or `module/tests/*.rs` over large inline `#[cfg(test)]` blocks
- Inline tests are acceptable only for small, local invariants

### Refactoring Expectation

- When adding new functionality, prefer creating a new module over extending an existing large file
- Refactoring existing code to improve structure is allowed **within the current phase**
- Structural refactors must not change observable behavior

## Frontend Validation Rules

- Frontend validation is permitted for user experience and early error detection
- Frontend validation must never be authoritative
- All domain validation must be enforced by the backend
- Backend validation failures must be explicit and surfaced to the frontend
- Frontend validation must not encode domain rules as decision logic

## API Ergonomics & Read Models

- Read-only APIs may be reshaped to support operator workflows
- Composite or aggregated read endpoints are allowed
- Ergonomic APIs must not introduce new domain logic
- Read APIs must remain side-effect free
- Domain invariants must not be bypassed for convenience

## Bid Years

- Exactly one bid year may be active at any given time
- All operational workflows are scoped to the active bid year
- APIs must not support simultaneous multi-bid-year operation
- Historical bid years may be queried only via explicit historical read APIs

## When to Stop

If any of the following are true:

- Requirements are ambiguous
- A change would violate rules above
- The solution requires guessing intent
- The change affects auditability or domain rules in unclear ways
- Failures appear to be caused by missing tools, packages, or environment configuration

→ Ask the user.
