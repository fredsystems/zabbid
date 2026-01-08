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

## When to Stop

If any of the following are true:

- Requirements are ambiguous
- A change would violate rules above
- The solution requires guessing intent
- The change affects auditability or domain rules in unclear ways
- Failures appear to be caused by missing tools, packages, or environment configuration

→ Ask the user.
