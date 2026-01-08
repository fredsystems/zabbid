# PLAN.md

## Purpose

This document defines the current implementation plan for the project.
It exists to guide incremental, agent-assisted development while preventing speculative or out-of-scope work.

This plan is expected to evolve.
Work must stay within the current phase unless explicitly revised.

---

## Phase 0: Foundations

### Goal

Establish a **correct, testable, auditable core** that proves the domain model, state transitions, and audit guarantees.

Phase 0 exists to validate architecture and intent, not to deliver features.

---

### Phase 0: Scope

Phase 0 includes:

- Core domain modeling
- Explicit state transitions
- Structured error handling
- Audit event generation
- Workspace structure and boundaries
- Unit testing of domain and transitions

Phase 0 explicitly excludes:

- Persistence or databases
- APIs (HTTP, gRPC, etc.)
- Authentication or authorization
- UI or frontend concerns
- Configuration systems
- Performance optimization
- Async or concurrency concerns
- Changes to the Nix development environment

---

### Phase 0: Workspace Structure

Phase 0 will establish the following workspace layout:

```text
crates/
domain/ # Pure domain types and rule validation
audit/ # Audit record types and invariants
core/ # State transitions and orchestration
```

Crate responsibilities are strict and must not overlap.

---

### Phase 0: Crate Responsibilities

#### `domain`

- Defines domain types and invariants
- Implements rule validation as pure functions
- Contains no side effects
- Has no dependency on time, IO, storage, or IDs

#### `audit`

- Defines audit record types
- Represents actors, causes, and actions as data
- Produces immutable audit events
- Contains no business logic or persistence

#### `core`

- Owns system state
- Applies commands to state
- Validates commands using `domain`
- Emits audit events for every successful state transition
- Ensures transitions are atomic (all-or-nothing)

---

### Phase 0: Core Concepts

Phase 0 must define, at minimum:

- **Command**
  Represents user or system intent as data only

- **State**
  Represents the full in-memory system state (minimal is acceptable)

- **Transition Result**
  Successful transitions must produce:
  - a new state
  - a corresponding audit event

- **Errors**
  All failures must be explicit, structured, and testable

No implicit state changes are allowed.

---

### Phase 0: Audit Guarantees

- Every successful state change must emit exactly one audit event
- Audit events must include:
  - the actor
  - the cause
  - the action performed
  - the previous state
  - the new state
- Audit records are immutable once created

If a state change cannot be audited, it must not occur.

---

### Phase 0: Testing Requirements

Phase 0 tests must demonstrate:

- Domain rules accept valid input
- Domain rules reject invalid input
- State transitions succeed when valid
- State transitions fail explicitly when invalid
- Successful transitions emit audit events
- Failed transitions do not mutate state and do not emit audit events

No mocks, no infrastructure, no external dependencies.

---

### Phase 0: Exit Criteria

Phase 0 is complete when all of the following are true:

- The workspace builds successfully
- All crates contain tests
- At least one complete command → transition → audit path exists
- Invalid commands fail explicitly
- No state change occurs without an audit event
- `cargo xtask ci` passes consistently

---

### Phase 0: Working Rules

- All work must remain within Phase 0 scope
- If a requirement appears to belong to a later phase, stop and ask
- Do not speculate about future features or infrastructure
- Phase 0 prioritizes correctness, clarity, and auditability over completeness

---

### Phase 0: Plan Changes

This plan may be updated:

- after completing Phase 0
- if Phase 0 assumptions prove incorrect
- if scope must be intentionally adjusted

Changes to this plan must be explicit and intentional.

## Phase 1: Domain Rules & Enforcement

**Goal:**
Establish and prove the **pattern** for defining, enforcing, failing, and auditing real domain rules.

Phase 1 validates _how_ rules are expressed and applied, not how many rules exist.

---

### Phase 1 Objectives

- Formalize bid-year–scoped domain vocabulary
- Implement a representative domain rule end-to-end
- Enforce domain rules through explicit core state transitions
- Produce auditable state changes for successful transitions
- Establish a repeatable pattern for adding future rules

---

### Phase 1 Scope

Phase 1 includes:

- Bid-year–scoped user domain modeling
- Domain rule definition as pure, testable functions
- Core command handling and rule enforcement
- Structured domain and core error handling
- Audit event generation for successful transitions
- Unit tests demonstrating success and failure paths

Phase 1 explicitly excludes:

- Seniority ordering or tie-breaking logic
- Bidding round modeling or round-specific rules
- Persistence or database integration
- API exposure (HTTP, gRPC, etc.)
- Authentication or authorization
- Configuration-driven behavior
- Performance optimization

---

### Phase 1 Representative Rule

Phase 1 will implement **one representative domain rule** to prove the enforcement pattern.

The initial rule is:

- **Within a bid year, user initials must be unique**

This rule must:

- Be scoped strictly to a single bid year
- Treat initials as the sole user identifier
- Fail explicitly with structured domain errors
- Be enforced before any state mutation
- Produce an audit event only on success

No additional rules may be added unless explicitly approved.

---

### Phase 1 Audit Guarantees

- Every successful state transition emits exactly one audit event
- Audit events must capture:
  - actor
  - cause
  - action performed
  - before and after state
- Failed transitions must not emit audit events
- Audit records are immutable once created

---

### Phase 1 Testing Requirements

Tests must demonstrate:

- Domain rules accept valid input
- Domain rules reject invalid input with structured errors
- Core transitions enforce domain rules consistently
- Failed transitions do not mutate state
- Successful transitions emit exactly one audit event
- Audit events accurately reflect before and after state

Tests should serve as executable examples of the rule enforcement pattern.

---

### Phase 1 Exit Criteria

Phase 1 is complete when all of the following are true:

- Bid-year–scoped user domain model is established
- The representative domain rule is implemented and tested
- Rule failures are explicit and structured
- Core state transitions enforce rules atomically
- Audit events are generated only for successful transitions
- Adding an additional rule would follow an obvious, mechanical pattern
- No round-specific or seniority-resolution logic exists
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

---

## Phase 2: API Skeleton & Boundary Hardening

**Goal:**
Expose the Phase 1 domain and enforcement model through a minimal API boundary without leaking domain logic or freezing contracts.

Phase 2 proves that the domain remains authoritative when driven externally.

---

### Phase 2 Objectives

- Introduce a minimal API boundary that drives core commands
- Define request and response DTOs distinct from domain types
- Translate domain and core errors into API-facing errors explicitly
- Ensure all successful API-driven transitions emit audit events
- Ensure failed API calls do not mutate state or emit audit events

---

### Phase 2 Scope

Phase 2 includes:

- A thin API layer that translates requests into core commands
- Request/response types scoped to API concerns
- Explicit error mapping from domain/core errors to API errors
- Boundary tests exercising API → core → audit paths

Phase 2 explicitly excludes:

- Persistence or database integration
- Authentication or authorization
- Stable or versioned public API contracts
- Frontend integration
- Pagination, querying, or filtering
- Bidding round modeling or round-specific logic
- Seniority ordering or tie-breaking
- Performance optimization

---

### Phase 2 Boundary Rules

- The API layer must not contain domain rules
- The API layer must not mutate state directly
- All state changes must occur via core transitions
- Domain types must not be exposed directly through the API
- Errors must be translated, not reinterpreted or hidden

---

### Phase 2 Testing Requirements

Tests must demonstrate:

- Valid API requests result in successful state transitions
- Invalid API requests result in structured API errors
- Domain rule failures propagate correctly through the API
- Failed API calls do not mutate state
- Successful API calls emit exactly one audit event
- API responses do not leak domain internals

---

### Phase 2 Exit Criteria

Phase 2 is complete when all of the following are true:

- A minimal API layer exists and compiles
- The Phase 1 representative rule can be exercised via the API
- Domain rules are not duplicated in the API layer
- Error translation is explicit and test-covered
- Audit events are emitted only on successful API calls
- No persistence, authentication, or round logic exists
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

--

## Phase 3: Persistence, Audit History, and Rollback

**Goal:**
Introduce durable persistence for audit history and derived state while preserving domain authority, audit guarantees, and rollback semantics.

Phase 3 proves that system state can be safely reconstructed, corrected, and inspected at any point in time without rewriting history.

---

### Phase 3 Objectives

- Persist audit events in a durable, append-only store
- Persist derived state snapshots to support efficient recovery
- Support explicit rollback as a first-class, auditable action
- Enable reconstruction of effective state at any point in time
- Ensure persistence failures do not result in partial or unaudited state changes

---

### Phase 3 Scope

Phase 3 includes:

- A persistence adapter implemented below the core layer
- Durable storage of audit events, ordered in time
- Durable storage of state snapshots per bid year and area
- SQLite-backed persistence suitable for read-heavy workloads
- Atomic persistence of audit events and snapshots
- Persistence-backed tests demonstrating correctness and recovery

Phase 3 explicitly excludes:

- Authentication or authorization
- Background jobs or asynchronous processing
- Performance tuning or query optimization
- Distributed systems or multi-node coordination
- Schema migration or versioning strategies
- Alternate persistence backends
- UI or frontend concerns

---

### Persistence Semantics

- The audit log is the authoritative source of truth
- Audit events are append-only and are never rewritten or deleted
- All audit events are scoped to a single bid year and a single area
- State is derived by replaying audit events in order
- Rollback is modeled as an explicit audit event
- Rollback establishes a prior effective state as authoritative going forward
- Rollback does not erase or modify historical audit events

---

### State Snapshots

- State is conceptually a complete, materialized snapshot per bid year and area
- Snapshots exist to accelerate recovery and replay
- Snapshots must not alter the meaning of the audit log
- Full state snapshots must be persisted at:
  - rollback events
  - round finalized events
  - explicit checkpoint events
- All other audit events persist deltas only

---

### Failure Guarantees

- Persistence operations must be atomic
- If persistence fails, the transition must fail
- Partial persistence of state or audit data is forbidden
- In-memory state must not advance unless persistence succeeds

---

### Phase 3 Testing Requirements

Tests must demonstrate:

- Successful persistence of audit events
- Persistence of full state snapshots at required boundaries
- Rollback recorded as an auditable event
- Reconstruction of effective state at a given point in time
- No partial persistence on simulated failure

Tests must use deterministic infrastructure provided by the development environment.

---

### Phase 3 Exit Criteria

Phase 3 is complete when all of the following are true:

- Audit events are durably persisted and ordered
- State snapshots are persisted at defined boundaries
- Rollback is implemented as an auditable event
- Effective state can be reconstructed at any point in time
- Persistence failures do not result in partial or unaudited state changes
- Domain and core behavior are unchanged by persistence
- No persistence details leak into domain or core layers
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently.

---

## Phase 4: Read Models & Queries

**Goal:**
Expose system state and audit history in a safe, deterministic, read-only manner without weakening domain authority, audit guarantees, or rollback semantics.

Phase 4 exists to make the system observable and inspectable while preserving all correctness invariants established in earlier phases.

---

### Phase 4 Scope

Phase 4 includes:

- Read-only access to effective state per (bid year, area)
- Read-only access to historical state at a specific point in time
- Read-only access to ordered audit event timelines
- Deterministic state reconstruction using persisted audit events and snapshots
- Query helpers or interfaces that surface domain data without mutation
- Tests validating correctness of reads and reconstruction

Phase 4 explicitly excludes:

- Any form of state mutation
- Write-capable APIs or command execution
- Authentication or authorization
- New domain rules or validations
- Persistence schema changes
- Background jobs, caching layers, or async processing
- Performance optimization beyond correctness

---

### Read Semantics

- All read operations must be side-effect free
- Reads must not emit audit events
- Reads must not modify in-memory or persisted state
- Reads must not depend on mutable global state
- Read results must be fully derivable from persisted data

---

### Required Read Capabilities

Phase 4 must support, at minimum:

#### Effective State Queries

- Retrieve the current effective state for a given bid year and area
- The effective state must reflect all audit events, including rollbacks

#### Historical State Queries

- Retrieve the effective state for a given bid year and area at a specific point in time
- Time-based queries must be deterministic
- If a timestamp does not correspond exactly to an event, the most recent prior event defines the state

#### Audit Timeline Queries

- Retrieve the ordered list of audit events for a given bid year and area
- Audit events must be returned in strict chronological order
- Rollback events must appear in the timeline as first-class events

---

### State Reconstruction Rules

- State reconstruction must:
  - start from the most recent snapshot at or before the target point
  - replay audit deltas forward deterministically
- Rollback events must alter the effective state for subsequent reconstruction
- Reconstruction logic must not depend on in-memory history
- Reconstruction must yield identical results across repeated executions

---

### Error Handling

- Invalid read requests (e.g. unknown bid year, area, or timestamp) must fail explicitly
- Errors must be structured and testable
- Read errors must not leak persistence or infrastructure details

---

### Phase 4 Testing Requirements

Tests must demonstrate:

- Retrieval of current effective state
- Retrieval of historical state at a given time
- Correct handling of rollback events during reconstruction
- Deterministic reconstruction from persisted data
- No mutation of state during read operations

Tests must not rely on mocks that bypass persistence behavior.

---

### Phase 4 Exit Criteria

Phase 4 is complete when all of the following are true:

- Current and historical state can be queried safely
- Audit timelines are accessible and ordered correctly
- Rollback effects are visible in read results
- All read paths are side-effect free
- State reconstruction is deterministic and tested
- No write paths or mutation logic are introduced
- Domain, persistence, and audit semantics remain unchanged
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

---

## Phase 5: Write APIs & Authorization

**Goal:**
Expose controlled, authenticated, and authorized write access to the system while preserving all domain, audit, persistence, and rollback guarantees.

Phase 5 introduces _who is allowed to mutate state_, not _how state behaves_.

---

### Phase 5 Scope

Phase 5 includes:

- Authentication of system actors
- Authorization of state-changing actions based on actor roles
- Write-capable APIs or interfaces that execute domain commands
- Enforcement of admin vs bidder authority boundaries
- Attribution of all state changes to an authenticated actor
- Persistence and audit of all authorized state transitions
- Tests validating authorization behavior and failure modes

Phase 5 explicitly excludes:

- New domain rules or validations
- Changes to persistence, rollback, or snapshot semantics
- UI or frontend concerns
- Performance optimization
- Multi-role or fine-grained permission systems

---

### Phase 5: Actor & Authorization Model

- All write actions are performed by authenticated actors
- Actors are distinct from domain users
- Roles apply only to actors, never to domain users

The system recognizes exactly two roles:

#### Phase 5: Admin

Admins are authorized to perform structural and corrective actions, including but not limited to:

- creating or modifying bid years
- creating or modifying areas
- creating or modifying users
- performing rollbacks
- creating checkpoints
- finalizing rounds or equivalent milestone events
- any other system-level or corrective actions

#### Phase 5: Bidder

Bidders are authorized to perform bidding-related actions, including:

- entering new bids
- modifying existing bids
- withdrawing or correcting bids

Bidders perform bidding actions on behalf of domain users.
They are not the same entities as the users whose bids are represented.

---

### Phase 5: Authorization Enforcement

- Authorization must be enforced before command execution
- Unauthorized actions must fail explicitly and deterministically
- Authorization failures must not mutate state or emit audit events
- Domain logic must remain unaware of actor roles
- Core state transitions must be role-agnostic

---

### Phase 5: Write Semantics

- All state changes must occur via explicit domain commands
- All successful write operations must:
  - execute domain validation
  - persist changes atomically
  - emit audit events attributing the acting actor
- Failed write operations must:
  - fail without mutating state
  - fail without emitting audit events
  - return structured, testable errors

---

### Phase 5: Error Handling

- Authentication and authorization errors must be explicit and structured
- Errors must not leak domain or persistence implementation details
- Authorization errors must be distinguishable from domain validation failures

---

### Phase 5 Testing Requirements

Tests must demonstrate:

- Successful execution of authorized write actions
- Explicit failure of unauthorized write actions
- Correct attribution of actors in audit events
- No state mutation on authorization failure
- No audit emission on authorization failure
- Preservation of all domain, persistence, and rollback invariants

Tests must not bypass authentication or authorization logic.

---

### Phase 5 Exit Criteria

Phase 5 is complete when all of the following are true:

- Write APIs are available for all intended commands
- All write operations require authenticated actors
- Authorization rules correctly enforce admin vs bidder roles
- Unauthorized actions fail explicitly and safely
- Audit events correctly attribute acting actors
- Domain, persistence, rollback, and snapshot semantics remain unchanged
- No role or authorization logic leaks into domain or core layers
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

---

## Phase 6: Domain Rule Expansion

**Goal:**
Incrementally implement real-world business rules on top of a stable, auditable core.

Phase 6 is where domain complexity increases, not infrastructure complexity.

### Phase 6: Scope

Phase 6 includes:

- Additional domain rules (e.g. eligibility, conflicts, limits)
- New commands and validations
- Rule-specific audit events
- Expanded test coverage for domain behavior

Phase 6 explicitly excludes:

- Infrastructure changes
- Persistence changes
- API surface redesign
- Performance optimization

### Phase 6: Exit Criteria

- New rules are explicitly validated and tested
- Invalid actions fail deterministically
- Audit trails fully reflect rule-driven behavior
- No infrastructure or persistence regressions occur

---

## Phase 7: API Stabilization & External Integration

**Goal:**
Prepare the system for reliable use by external clients and tools.

Phase 7 focuses on API clarity, consistency, and operability.

### Phase 7: Scope

Phase 7 includes:

- API shape refinement
- Error stability and consistency
- Pagination and filtering for read endpoints
- Versioning strategy (if required)
- External client compatibility considerations

Phase 7 explicitly excludes:

- New domain rules
- Persistence changes
- Authorization redesign
- UI implementation

### Phase 7: Exit Criteria

- API surfaces are stable and documented
- Errors are structured and predictable
- External consumers can integrate reliably
- No domain or audit guarantees are weakened

---

## Phase 8: Operational Concerns (Optional)

**Goal:**
Support long-term operation, monitoring, and maintenance without altering system semantics.

This phase is optional and driven by operational needs.

### Phase 8: Scope

Phase 8 may include:

- Metrics and instrumentation
- Audit export and reporting
- Backup and retention strategies
- Data archival policies
- Performance tuning based on observed usage

Phase 8 explicitly excludes:

- Domain rule changes
- Audit semantics changes
- Authorization changes

### Phase 8: Exit Criteria

- Operational visibility is sufficient
- System remains correct under load
- No changes affect domain correctness or auditability

---
