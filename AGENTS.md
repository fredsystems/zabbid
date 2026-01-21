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
- AGENTS.md is authoritative
- Agents must not reinterpret, weaken, or “improve” rules found here
- If a rule appears inconsistent with the codebase, stop and ask

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

- Rust: idiomatic, clippy-clean, no `.unwrap()` or `.expect()` in production code
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
- For casting primitive types that don't fit within each other, use `num-traits`.

### Dead Code Policy

- `#[allow(dead_code)]` is forbidden in production modules to silence
  planned-but-unwired functionality.
- Acceptable uses:
  - test-only helpers inside `#[cfg(test)]`
  - temporary refactors with an explicit TODO and issue reference
- Future-phase functionality must be:
  - gated behind a feature flag, or
  - isolated in a module not compiled into the main path, or
  - not implemented yet.
- If a symbol exists in production code, it must be reachable from a
  public API or intentionally hidden behind a compile-time gate.

### Panic-Free Production Code (Non-Negotiable)

- `unwrap()` and `expect()` are **forbidden** in all production code
- Panics must never be used to enforce domain invariants
- All invariant violations must surface as typed, recoverable errors
- The only permitted use of `unwrap()` / `expect()` is in:
  - test code (explicitly marked with `#[cfg(test)]` or in `tests/` modules)
  - test-only helpers
- Any production `unwrap` / `expect` is a correctness bug, not a shortcut

**Rationale:**

- Panics erase recoverable error paths and prevent graceful degradation
- Panics cause non-deterministic production failures that cannot be handled
- Panics hide future refactor bugs by masking invariant violations
- Clippy is configured to enforce this invariant at compile time via `clippy::unwrap_used` and `clippy::expect_used`

This rule is enforced through:

- `#![deny(clippy::unwrap_used, clippy::expect_used)]` in all production crate roots
- `#![allow(clippy::unwrap_used, clippy::expect_used)]` in test module roots only

## Code Semantics & Readability

### Testing

Testing is mandatory and treated as first-class code.

- Every non-trivial behavior change **must be testable**
- Every test must document a **specific domain invariant**
- Success and failure cases are both required unless one is provably impossible

- Any bug fix **must** include a regression test unless explicitly justified by context
- Tests must be hermetic:
  - No shared mutable state
  - No reliance on execution order
  - No hidden setup or teardown

- Tests must be written for **humans first**
  - Clarity is more important than reuse
  - Duplication in tests is acceptable if it improves readability

- Tests must assert **observable outcomes**, not internal mechanics
- A failing test should immediately explain _why_ the behavior is invalid

### Boolean Usage Guidelines

Booleans are permitted and encouraged when they represent:

- Simple, independent flags
- Obvious yes/no state
- Local implementation details
- Small structs with 1–2 boolean fields

However, when a group of booleans collectively represents:

- Capabilities
- Permissions
- States in a workflow
- Policy decisions
- Conceptual sets of allowed actions

agents should prefer a **semantic representation** internally, such as:

- Enums
- Enum sets
- Structs wrapping meaningful types

This improves readability, reasoning, and maintainability.

### Internal vs External Representation

It is acceptable — and often desirable — to:

- Use semantic types (e.g. enums) internally
- Expose simplified booleans at API boundaries for ergonomics

Example pattern:

- **Internal**: enum-based capability or state model
- **API layer**: serialized boolean fields derived from that model

Agents should **not** suppress clippy warnings about excessive booleans
unless the boolean representation is clearly the most readable and appropriate choice.

If unsure, prefer clarity over mechanical consistency and ask.

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

### External Database Testing Rules

- External databases (e.g. MariaDB, MySQL) are **runtime test dependencies**, not feature-gated components
- Code must compile with support for all database backends enabled by default
- Tests that require external databases must:
  - be explicitly marked with `#[ignore]`
  - fail fast if required environment variables are missing
  - never run as part of `cargo test` by default

- External database tests must be executed **only** via explicit tooling
  (e.g. `cargo xtask test-mariadb`)

- Agents must NOT:
  - use Cargo feature flags to gate database backends
  - conditionally compile code to avoid missing databases
  - modify production code to “accommodate” test infrastructure

If a test requires an external database and no explicit execution path exists,
the agent must stop and request guidance.

### xtask Responsibilities

- `xtask` is the authoritative orchestration layer for:
  - external service lifecycle (e.g. Docker containers)
  - environment validation (tools, daemons, credentials)
  - explicit opt-in test execution

- Agents may assume:
  - `xtask` may start and stop containers
  - `xtask` may set environment variables
  - `xtask` may gate tests by intent

- Agents must NOT:
  - embed Docker or service lifecycle logic into tests
  - rely on ambient system state
  - assume CI or local environments are identical

If external infrastructure is required and no `xtask` entry point exists,
the correct action is to add one or stop and ask.

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
- Users are uniquely identified by a canonical `user_id` (i64)
- **All persistence, lookup, and state transitions must use `user_id`**
- User initials are **display metadata only**
- Initials:
  - are unique within a bid year by policy
  - may change at any time
  - must never be treated as a stable identifier
  - must not be used as primary keys, foreign keys, or authoritative lookup inputs

### Areas

- Areas are scoped to a single bid year
- Areas are uniquely identified by a canonical `area_id` (i64)
- **All persistence, lookup, and state transitions must use `area_id`**
- Area code is **display metadata only**
- Area codes:
  - are unique within a bid year by policy
  - are normalized to uppercase for consistency
  - are immutable after creation
  - must never be treated as a stable identifier for persistence or mutations
  - must not be used as primary keys or foreign keys
- Area names are optional display metadata and may be changed at any time
- System areas (e.g., "No Bid") cannot be renamed
- Area metadata edits are prohibited after canonicalization (lifecycle constraint)

### Bid Year Identity

- Bid years are identified by a canonical `bid_year_id` (i64)
- **All persistence, lookup, and state transitions must use `bid_year_id`**
- The year value (u16) is **display metadata only**
- Year values:
  - are unique across all bid years by policy
  - are immutable after creation
  - must never be treated as a stable identifier for persistence or mutations
  - must not be used as primary keys or foreign keys
- Label and notes are optional display metadata and may be changed at any time
- Exactly one bid year may be active at any given time

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

## Override & Edit Semantics

### Direct Edits (Canonical State Mutations)

Canonical data may be edited directly when lifecycle and authorization constraints allow.

Direct edits:

- Mutate canonical tables in place
- Generate audit events recording the change
- Require admin authorization
- Are subject to lifecycle constraints (e.g., no area edits after canonicalization)

Examples of direct edits:

- Updating a user's initials, name, area assignment, or seniority data
- Updating an area's display name
- Updating a bid year's label or notes

Direct edits are **not** overrides. They represent legitimate data corrections or updates.

### Overrides (Exceptional State Changes)

Overrides apply to derived canonical state (e.g., area assignments, eligibility, bid order).

Overrides:

- Require an explicit reason (min 10 characters)
- Are recorded in separate canonical tables with `is_overridden` flags
- Generate audit events with override semantics
- Supersede algorithmically derived values
- Are permanent and auditable

Examples of overrides:

- Overriding a user's area assignment
- Overriding a user's eligibility status
- Overriding a user's bid order

Overrides are **not** edits. They represent exceptional corrections to system-computed state.

### Identity Constraints

- `user_id`, `area_id`, and `bid_year_id` are **always immutable**
- Display metadata (initials, area codes, year values) may be mutable depending on context
- Edits and overrides must always reference entities by their canonical IDs

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

## Capabilities & Authorization Semantics

Roles define who an operator is.
Capabilities define what an operator can do right now.

### Rules

- The backend is the sole authority for determining capabilities

- Capabilities may depend on:
  - operator role
  - system state
  - domain invariants (e.g. “last active admin”)

- The frontend must:
  - render pages consistently
  - gate actions only (buttons, forms, destructive controls)
  - rely on capability flags provided by the backend

- The frontend must NOT:
  - infer permissions from roles
  - encode domain rules (e.g. “admins can always X”)
  - assume an action is allowed because a button is visible

### Capability Design

- Capabilities must be expressed as explicit booleans
  (e.g. can_disable_operator, can_delete_user)
- Capability payloads must not explain why an action is disallowed
- Capabilities must be deterministic and testable

### Backend Enforcement

- Capability exposure does NOT replace authorization checks
- All mutating endpoints must still enforce authorization
- Capability mismatches must fail safely with authorization errors

### This ensures

- UI and backend cannot drift
- New permissions can be added without UI rewrites

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

## UI & Frontend Design Constraints

### Mobile-First Requirement

All frontend UI work **must be designed mobile-first**.

This applies especially to the **public-facing interface**, which must be:

- Fully usable on mobile devices
- Designed assuming a small viewport first
- Touch-friendly (no hover-only interactions)
- Legible without zooming
- Navigable without relying on precise pointer input

Desktop layouts may enhance or expand the UI, but **mobile usability is the baseline**.

---

### Admin Interface Expectations

- The admin interface **must function correctly on mobile**, but:
  - It is acceptable for it to be less ergonomic than desktop
  - Dense tables or advanced workflows may degrade gracefully
- Admin UX should prioritize correctness and clarity over compactness

---

### Design Rules

Agents must:

- Use responsive layouts (`flex`, `grid`, fluid widths)
- Avoid fixed-width assumptions
- Avoid desktop-only affordances
- Prefer vertical stacking over horizontal density
- Ensure critical actions are reachable on mobile screens

Agents must **not**:

- Treat mobile support as a later enhancement
- Design desktop-only layouts and “adapt later”
- Introduce UI patterns that are unusable without a mouse

---

### Frontend Logic Boundary (Reminder)

Frontend validation is permitted **only** to improve UX.

- The frontend may:
  - Prevent obviously invalid input
  - Provide early feedback
  - Disable impossible actions
- The backend remains the **sole authority**
- All frontend checks must assume they can be bypassed
- The frontend may disable or hide actions based on capabilities, not roles.

---

### Failure to Comply

If a UI design cannot be reasonably adapted to mobile **without significant refactoring**, the agent must stop and ask before proceeding.

### UI Refactor Allowance (Mobile Compliance)

If existing UI code violates the mobile-first requirement (e.g. desktop-first `<table>` layouts),
agents are **explicitly permitted and expected** to refactor those components to achieve mobile usability.

Refactors must:

- Preserve all existing functionality
- Preserve API interactions and semantics
- Avoid introducing new domain logic
- Prefer incremental component replacement over wholesale rewrites

Acceptable refactor patterns include:

- Replacing `<table>` layouts with:
  - stacked card layouts
  - definition lists
  - responsive grids
- Using progressive disclosure on mobile (collapse / expand)
- Rendering different layouts at different breakpoints using the same data source

Unacceptable refactors include:

- Changing API contracts
- Reworking backend logic to suit the UI
- Introducing new state models or abstractions "for convenience"
- Rewriting the entire UI when targeted refactors suffice

---

### UI Styling Guidelines

UI styling should follow these proven patterns from the Bootstrap Completeness implementation:

### Styling Enforcement

- Inline styles (`style={{ ... }}` or `style="..."`) are **not permitted**
- All styling must be implemented via:
  - SCSS modules
  - Shared style partials
  - Existing design tokens and variables
- Exceptions require explicit user approval

Inline styles obscure intent, bypass design consistency, and are not auditable.

#### Component Organization

- **Logical sections** with clear headings (`<section className="bootstrap-section">`)
- **Item-based layouts** using cards for lists (not tables)
- **Inline editing** - toggle between view and edit modes within the same component
- **Progressive disclosure** - create forms collapsed by default, expand when needed

#### Visual Hierarchy

- **Status overview** at top with large, prominent badges
- **Section titles** with bottom borders to separate concerns
- **Item cards** with clear headers and bodies
- **Actions** positioned consistently (e.g., "Set Active" in header, "Edit" in body)

#### Form Controls

All form inputs must be readable with proper contrast:

```scss
input[type="text"],
input[type="number"],
input[type="date"],
select {
  background: $color-bg-base; // Dark background, NOT white
  color: $color-text-primary;
  border: 2px solid $color-border;
  padding: $spacing-sm $spacing-md;

  &:focus {
    border-color: $color-accent-primary;
  }
}
```

#### Button Styling

Buttons must be visually distinct and readable:

- **Primary/Create buttons**: Colored background (`$color-accent-primary`) with dark text
- **Save buttons**: Teal background (`$color-accent-teal`) with **dark text** for readability
- **Edit buttons**: Subtle, bordered style for secondary actions
- **Cancel buttons**: Border-only style, clearly different from primary
- **Toggle/Set Active buttons**: Prominent, clearly clickable

All buttons must have:

- Clear hover states
- Box shadows for depth
- Proper disabled states (opacity: 0.5)
- Adequate padding for touch targets

#### Color Usage

- **Complete/Success**: Green borders and badges
- **Incomplete/Warning**: Yellow borders and badges
- **Active**: Blue badges
- **Errors**: Red with semi-transparent backgrounds
- **Text on colored backgrounds**: Always use dark (`$color-bg-base`) for readability

#### Layout Patterns

**Item cards** (bid years, areas, users):

```scss
.item {
  background: $color-bg-surface;
  border: 2px solid $color-border;
  border-radius: $radius-md;

  .item-header {
    display: flex;
    justify-content: space-between;
    padding-bottom: $spacing-md;
    border-bottom: 1px solid $color-border;
  }

  .item-body {
    // View mode: dl with inline edit button
    // Edit mode: form fields with save/cancel
  }
}
```

**Create forms**:

```scss
.create-form {
  background: $color-bg-surface-elevated;
  border: 2px solid $color-accent-primary;
  border-radius: $radius-md;
  margin-top: $spacing-md;
}
```

#### Responsive Behavior

- Mobile: Stack all elements vertically
- Tablet (600px+): Side-by-side labels and inputs
- Desktop (768px+): More generous padding, wider max-widths

#### Empty States

Guide users with clear empty state messages:

```tsx
<p className="empty-state">
  No items configured. Create one below to get started.
</p>
```

These patterns create a consistent, readable, and usable interface across all admin workflows.

## Authentication & Security Error Handling

Authentication is a **security boundary**, not a usability feature.

Agents must ensure that authentication failures do **not** leak
security-sensitive information to unauthenticated clients.

### Authentication Failures (Pre-Session)

For login and session-establishment endpoints:

- Failures must be indistinguishable to the client
- The API must NOT reveal whether:
  - the username does not exist
  - the password is incorrect
  - the operator account is disabled
  - the operator lacks required roles
- All authentication failures must return a single, generic error message
  (e.g. “invalid credentials”)

Internal causes:

- MAY be logged for diagnostics
- MAY be recorded in audit or security logs
- MUST NOT be exposed via API responses or UI messaging

### Authorization Failures (Post-Authentication)

Once a user is authenticated and has an active session:

- Authorization failures MAY be explicit
- It is acceptable to return errors such as:
  - “insufficient permissions”
  - “admin role required”
- These errors must still avoid leaking internal state details
  (e.g. counts, existence of other operators)

### UI Responsibilities

- The UI must treat authentication errors as opaque
- The UI must not branch on HTTP status codes or error bodies to infer authentication causes
- The UI must never attempt to infer or display the underlying cause
- The UI may display a single generic error message only

This rule applies **only** to authentication and session-establishment flows.

## Canonical Identity Enforcement (Phase 23A Complete)

Phase 23A (Canonical Identity for Area & Bid Year) is complete.
The following rules are now **active and enforced**:

### Identity Model

All primary domain entities use canonical numeric identifiers:

- Users: `user_id` (i64)
- Areas: `area_id` (i64)
- Bid Years: `bid_year_id` (i64)

Display metadata (initials, area codes, year values) must **never** be used as:

- Primary keys
- Foreign keys
- Lookup inputs for persistence operations
- Authoritative identifiers in state transitions

### Persistence Rules

Agents must NOT:

- Create sentinel or fake canonical records (e.g. negative IDs, year = 0)
- Insert placeholder rows solely to satisfy foreign key constraints
- Hardcode identity mappings (e.g. year → ID magic values)
- Auto-create canonical entities as a side effect of unrelated operations
- Mutate persistence logic to "heal" missing state for test compatibility
- Add filtering logic to hide non-domain records from queries
- Modify schema or persistence behavior to make out-of-scope tests pass

Canonical tables must contain **only real domain entities** created via
explicit bootstrap or domain transitions.

### Lookup Functions

When display metadata must be converted to canonical IDs, use authoritative lookup functions:

- `lookup_bid_year_id(year: u16) -> Result<i64>`
- `lookup_area_id(bid_year_id: i64, area_code: &str) -> Result<i64>`

These lookups are **input validation only** and must not be used as primary identifiers
within already-validated domain operations.

### Tooling Restrictions for Refactors

When performing large refactors or identity migrations, agents must NOT:

- Use Python, shell scripts, sed/awk, or external tooling to modify code
- Generate or apply mechanical edits outside the Rust codebase
- Bypass normal refactoring patterns due to context or token pressure

If remaining work exceeds available context or requires unsafe shortcuts,
the agent must stop and request guidance.

Correctness and architectural integrity take precedence over completion speed.

## Database Tooling

- Default to Diesel DSL for all persistence queries.
  Raw SQL is allowed only when the DSL cannot express the query cleanly, safely, or without obscuring intent.
- Any raw SQL must be narrowly scoped and documented with the reason DSL was rejected.
- If Diesel CLI or migration tooling is required, it must be provided via the Nix environment or xtask; agents must not assume its presence.
- SQLite-specific helpers such as `last_insert_rowid()` are acceptable when required by backend limitations, but must be narrowly scoped and isolated behind Diesel abstractions.

### Multi-Backend Database Policy

- SQLite is the default testing backend and must support in-memory operation
- Other backends (e.g. MariaDB/MySQL) must:
  - be supported by the same Diesel schema and queries
  - be validated via explicit, opt-in tests

- Diesel is the canonical persistence abstraction
- Backend-specific behavior must be:
  - isolated
  - documented
  - tested explicitly

Agents must NOT:

- introduce backend-specific schema divergence
- modify queries to "fit" one backend at the expense of others
- add compatibility hacks to satisfy a single database engine

### Migration Guardrails & Schema Parity Enforcement

Database migrations exist in backend-specific directories to accommodate syntax differences
between SQLite and MySQL/MariaDB. These migrations must remain **schema-equivalent** at all times.

#### Migration Directory Structure

- `migrations/` — SQLite-specific migrations (default for development and testing)
- `migrations_mysql/` — MySQL/MariaDB-specific migrations (for production and opt-in validation)

#### Backend-Specific Migrations Are Allowed

Separate migration directories exist because SQL syntax differs between backends:

- **Auto-increment**: SQLite uses `AUTOINCREMENT`, MySQL uses `AUTO_INCREMENT`
- **Integer types**: SQLite uses `INTEGER`, MySQL uses `BIGINT` or `INT`
- **Text types**: SQLite uses `TEXT`, MySQL uses `VARCHAR(n)` or `TEXT`
- **Boolean types**: SQLite uses `INTEGER`, MySQL uses `TINYINT`
- **Storage engines**: MySQL requires explicit `ENGINE=InnoDB`

These are **syntax differences only**. The resulting schemas must be semantically identical.

#### Schema Equivalence Is Mandatory

Backend-specific migrations must produce schemas that are:

- Structurally identical (same tables, columns, relationships)
- Semantically equivalent (same constraints, nullability, uniqueness)
- Functionally interchangeable (same Diesel schema applies to both)

Schema equivalence is **not a convention**. It is **enforced by tooling**.

#### Verification Tooling

Schema parity is verified via:

```bash
cargo xtask verify-migrations
```

This command:

1. Provisions ephemeral databases (SQLite in-memory, MariaDB via Docker)
2. Applies backend-specific migrations to each database
3. Introspects resulting schemas (tables, columns, types, constraints)
4. Normalizes backend-specific type representations
5. Compares schemas structurally
6. Fails hard on any mismatch
7. Cleans up all resources (even on failure)

This command must pass before any migration changes are considered complete.

#### Agent Responsibilities

When adding or modifying migrations, agents must:

- Create equivalent migrations in **both** `migrations/` and `migrations_mysql/`
- Use backend-appropriate syntax in each directory
- Ensure the resulting schemas are semantically identical
- Run `cargo xtask verify-migrations` to confirm parity
- Never assume migrations are "close enough"

Agents must NOT:

- Modify only one migration directory and hope it works
- Introduce schema differences between backends
- Relax constraints to make one backend "easier"
- Add backend-specific tables, columns, or relationships
- Assume SQLite migrations will work on MySQL unchanged
- Bypass verification tooling

#### When Migrations Diverge

If schema verification fails:

- The agent must stop immediately
- The agent must NOT modify runtime code to compensate
- The agent must NOT relax schema constraints
- The agent must fix the migration divergence at the source

If the divergence appears irreconcilable without changing domain semantics, stop and ask.

#### Enforcement Philosophy

- Tooling enforces invariants — humans do not
- Correctness over convenience
- Schema parity is a **hard requirement**, not a guideline
- Silent divergence is considered a critical failure

## When to Stop

If any of the following are true:

- Requirements are ambiguous
- A change would violate rules above
- The solution requires guessing intent
- The change affects auditability or domain rules in unclear ways
- Failures appear to be caused by missing tools, packages, or environment configuration
- A test failure appears to be caused by backend-specific behavior
- A database backend requires additional services not provisioned by xtask
- The agent is tempted to alter schema or persistence logic to satisfy a test

→ Ask the user.
