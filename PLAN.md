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

## Phase 5.5: Server Binary & Operator Interface

**Goal:**
Make the system runnable and observable by a human operator without introducing new domain logic, persistence semantics, or authorization rules.

Phase 5.5 exists to validate correctness through interaction, not to finalize APIs or user experience.

---

### Phase 5.5 Scope

Phase 5.5 includes:

- A real server binary that hosts the system
- HTTP endpoints for:
  - executing existing write commands
  - performing read-only queries
- Wiring of persistence, read models, and authorization
- Minimal authentication stubs sufficient to produce an Actor
- Structured JSON request and response bodies
- Logging and instrumentation suitable for inspection
- Manual and automated tests exercising end-to-end flows

Phase 5.5 explicitly excludes:

- New domain rules or validations
- UI or frontend implementation
- API stability or versioning guarantees
- Real authentication mechanism design
- Changes to audit, rollback, or snapshot semantics
- Performance optimization
- New markdown documentation beyond existing project documents

---

### Phase 5.5 Interface Philosophy

- The HTTP interface is an operator interface, not a public API
- Endpoints may be verbose, explicit, or awkward
- JSON responses should favor completeness over ergonomics
- No attempt should be made to normalize, simplify, or prettify outputs

---

### Phase 5.5 Operator Semantics

- All write requests are executed by authenticated Actors
- Actors are produced by a minimal authentication boundary
- Actor roles are limited to Admin and Bidder
- Authorization is enforced before command execution
- Authorization logic must not leak into domain or core layers

---

### Phase 5.5 Write Semantics

- All write endpoints must:
  - authenticate an Actor (stub is acceptable)
  - enforce authorization
  - delegate to existing core commands
  - persist changes atomically
  - emit audit events on success only

- Failed requests must:
  - return structured errors
  - not mutate state
  - not emit audit events

---

### Phase 5.5 Read Semantics

- Read endpoints must be strictly side-effect free
- Reads must not depend on mutable in-memory state
- Reads must reflect persisted audit and snapshot data
- Reads must support:
  - current effective state
  - historical state at a point in time
  - ordered audit event timelines

---

### Phase 5.5 Exit Criteria

Phase 5.5 is complete when all of the following are true:

- The server binary can be run locally
- State can be mutated via HTTP write requests
- State and audit history can be queried via HTTP
- Rollback behavior can be exercised interactively
- All interactions reflect true persisted state
- No new domain or persistence semantics were introduced
- Operator roles are enforced only at the boundary
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

---

## Phase 6.1: Bootstrap & Structural Domain Rules

### Phase 6.1 Goal

Establish a valid, enforceable system baseline by implementing required bootstrap commands and structural domain constraints.

Phase 6.1 ensures the system cannot enter an invalid or partially-initialized state.

---

### Phase 6.1 Scope

Phase 6.1 includes:

- Bid year creation and validation
- Area creation within a bid year
- Listing existing bid years
- User creation with explicit structural validation
- Enforcement of baked-in crew semantics
- Enforcement of user classification (CPC, CPC-IT, Dev-D, Dev-R)

Phase 6.1 explicitly excludes:

- Bidding logic
- Crew reassignment or bid modification
- Seniority ordering or comparison
- Eligibility rules
- Round modeling
- Limits, capacity, or availability rules

---

### Phase 6.1 Bootstrap Requirements

- A fresh database with no data is a valid initial state
- No commands may succeed unless a bid year exists
- Bid years must:
  - be unique
  - represent a valid calendar year
- Areas must:
  - be explicitly created per bid year
  - exist before users may be created

Bootstrap order is enforced and must not be inferred.

---

### Phase 6.1 User Creation Rules

- Users are scoped to exactly one bid year
- User initials must be unique within a bid year
- User names are informational and not unique
- Users must belong to exactly one area
- Users may have zero or one crew assignment
- If provided, crew values must be one of 1–7
- User type must be one of:
  - CPC
  - CPC-IT
  - Dev-D
  - Dev-R

User creation must fail explicitly if any rule is violated.

---

### Phase 6.1 Crew Semantics

- Crews are baked-in domain constants
- Exactly seven crews exist, identified by numbers 1 through 7
- Each crew has a predefined RDO pattern
- Crews are not created, modified, or deleted
- Crews are not persisted as mutable data
- Crew assignment is optional at user creation
- Crew assignment is modeled as state and must be auditable

---

### Phase 6.1 Failure Semantics

Commands must fail explicitly if:

- The referenced bid year does not exist
- The referenced area does not exist
- User initials already exist within the bid year
- A provided crew value is invalid
- A provided user type is invalid

Failure guarantees:

- No state mutation
- No audit event emission
- Deterministic, structured errors

---

### Phase 6.1 Exit Criteria

Phase 6.1 is complete when all of the following are true:

- Bid years can be created and listed deterministically
- Areas can be created only within existing bid years
- Users can be created only against existing bid years and areas
- Invalid bootstrap order fails explicitly
- Crew validation is enforced consistently
- User type validation is enforced consistently
- All successful actions emit audit events
- All failed actions emit no audit events
- Read models reflect the structural state correctly
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

---

## Phase 6.2: Bootstrap API Completeness

### Phase 6.2 Goal

Ensure the entire bootstrap process is fully accessible, enforceable, and observable through explicit API endpoints.

Phase 6.2 exists to guarantee that a system can be initialized from an empty database using only supported HTTP APIs, without implicit behavior or out-of-band setup.

---

### Phase 6.2 Scope

Phase 6.2 includes:

- API endpoints for bid year creation and listing
- API endpoints for area creation and listing
- API endpoints for user listing (structural visibility only)
- End-to-end bootstrap via HTTP from an empty database
- Authorization and audit coverage for all bootstrap actions
- API-level tests validating bootstrap behavior

Phase 6.2 explicitly excludes:

- Bidding logic or crew reassignment
- Seniority logic
- Eligibility or capacity rules
- Round modeling or round lifecycle
- New domain rules or state transitions
- Persistence schema changes
- UI or frontend concerns

---

### Phase 6.2 Bootstrap API Requirements

The following bootstrap steps must be achievable exclusively via API calls:

1. Create a bid year
2. List existing bid years
3. Create one or more areas within a bid year
4. List areas for a given bid year
5. Create users within existing bid years and areas
6. List users per bid year and area

No implicit creation or side effects are allowed.

---

### Phase 6.2 Required API Endpoints

Phase 6.2 must expose API endpoints for:

#### Bid Years

- Create bid year
- List bid years

Bid year listing must never fail.

---

#### Areas

- Create area within a bid year
- List areas for a bid year

Area creation must fail explicitly if the bid year does not exist.

---

#### Users (Structural)

- Create user (already exists)
- List users for a given bid year and area

User listing is read-only and must not mutate state.

---

### Phase 6.2 Failure Semantics

Bootstrap API calls must fail explicitly if:

- A bid year does not exist
- An area does not exist
- A bid year is duplicated
- Structural preconditions are violated

Failure guarantees:

- No state mutation
- No audit event emission
- Structured, deterministic errors

---

### Phase 6.2 Audit Requirements

- All successful bootstrap actions must emit exactly one audit event
- Audit events must attribute:
  - acting actor
  - cause
  - action performed
- Failed bootstrap actions must not emit audit events

---

### Phase 6.2 Exit Criteria

Phase 6.2 is complete when all of the following are true:

- A fresh database can be fully bootstrapped via API alone
- Bid years can be created and listed via API
- Areas can be created and listed via API
- Users can be created and listed via API
- All bootstrap actions are auditable
- No domain rules are duplicated in the API layer
- No new domain behavior is introduced
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

## Phase 7: Canonical State Refactor

**Goal:**
Introduce explicit, relational canonical storage for current operational state while preserving all existing audit, snapshot, and rollback guarantees.

This phase aligns the implementation with clarified architectural intent.
Earlier phases are not reinterpreted or modified.

---

### Phase 7 Scope

Phase 7 includes:

- Introduction of canonical relational tables for current state
  - users
  - areas
  - bid years
  - (future) bids
- Refactoring write paths to:
  - update canonical tables
  - emit audit events
  - optionally persist snapshots
- Refactoring read APIs to:
  - read from canonical tables for current state
  - use audit/snapshots only for historical queries
- Transactional consistency between:
  - canonical state mutation
  - audit event persistence
- Migration of existing persistence logic to respect this separation

Phase 7 explicitly excludes:

- New domain rules
- New business logic
- Changes to audit semantics
- Changes to rollback semantics
- Performance optimization
- Schema versioning strategies
- Data migrations for existing deployments

---

### Phase 7 Canonical vs Derived Responsibilities

- Canonical tables represent **current authoritative state**
- Audit events represent **what happened and when**
- Snapshots represent **accelerators for historical reconstruction**
- Current-state APIs must not replay audit logs
- Historical APIs must not read canonical tables directly

---

### Phase 7 Invariants

Phase 7 must preserve all existing invariants:

- All state mutations are auditable
- Audit logs remain append-only
- Rollbacks are explicit events
- Historical state reconstruction remains deterministic
- No state change occurs without an audit event

---

### Phase 7 Testing Requirements

Tests must demonstrate:

- Canonical tables reflect current state correctly
- Audit events are emitted for all mutations
- Snapshots reflect canonical state at the correct event
- Read APIs return correct data without audit replay
- Historical queries remain correct and deterministic

---

### Phase 7 Exit Criteria

Phase 7 is complete when:

- Canonical tables exist and are populated correctly
- Current-state APIs read exclusively from canonical tables
- Historical APIs remain functional and unchanged
- Audit and snapshot behavior is preserved
- No existing domain rules are altered
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes

---

## Phase 8.1: Bid Year Canonical Definition

### Phase 8.1 Purpose

Phase 8.1 formalizes the **canonical domain definition of a bid year**.

This phase establishes _what a bid year is_ in operational terms, independent of persistence, APIs, or UI concerns.
It creates the foundation required for correct pay-period modeling and leave accrual logic in later phases.

---

### Phase 8.1 Goal

Define a **deterministic, auditable, and testable bid year model** that:

- Represents real FAA leave years
- Supports both 26- and 27-pay-period years
- Enables precise pay-period derivation
- Does not depend on runtime clocks, databases, or external systems

---

### Phase 8.1 Scope

Phase 8.1 includes:

- Canonical bid year domain modeling
- Validation of bid year structural correctness
- Deterministic pay-period derivation
- Pure, side-effect-free domain logic
- Exhaustive unit tests for bid year behavior

Phase 8.1 explicitly excludes:

- Leave accrual calculations
- Seniority or eligibility logic
- Persistence schema changes
- API exposure or request handling
- Bidding, scheduling, or round logic
- Time-zone handling or clock access

---

### Phase 8.1 Canonical Bid Year Definition

A bid year MUST be defined by the following canonical inputs:

- **Bid year identifier** (human-readable, e.g. `2026`)
- **Start date** (ISO-8601 date, inclusive)
- **Number of pay periods** (26 or 27 only)

A bid year MUST NOT be defined by calendar year boundaries.

---

### Phase 8.1 Derived Bid Year Properties

From the canonical definition, the following properties MUST be derived deterministically:

- Bid year end date
- Ordered list of pay periods
- Each pay period’s:
  - index (1-based)
  - start date (inclusive)
  - end date (inclusive)

Derived properties MUST NOT be persisted as canonical data.

---

### Phase 8.1 Pay Period Semantics

- Pay periods are **bi-weekly (14 days)**
- The first pay period starts on the bid year start date
- Pay periods are contiguous and non-overlapping
- The final pay period ends exactly at the derived bid year end date
- Pay periods are immutable once derived

---

### Phase 8.1 Validation Rules

Bid year creation MUST fail explicitly if:

- The start date is invalid
- The number of pay periods is not exactly 26 or 27
- The derived end date is inconsistent with the number of pay periods
- Any derived pay period would overlap or be non-contiguous

Validation failures MUST:

- Produce structured domain errors
- Be deterministic
- Prevent any downstream use of the invalid bid year

---

### Phase 8.1 Domain Placement

Phase 8.1 logic MUST reside in the `domain` crate.

- No persistence concerns
- No API concerns
- No global state
- No time-based logic
- No side effects

Core and persistence layers MUST treat the bid year as opaque domain data.

---

### Phase 8.1 Testing Requirements

Tests MUST demonstrate:

- Creation of valid 26-pay-period bid years
- Creation of valid 27-pay-period bid years
- Correct derivation of pay period boundaries
- Deterministic behavior across repeated executions
- Explicit failure on invalid definitions
- Absence of side effects or hidden dependencies

Tests MUST be exhaustive for boundary conditions.

---

### Phase 8.1 Failure Semantics

On failure:

- No state mutation may occur
- No audit events may be emitted
- Errors MUST be structured and testable
- Failures MUST be deterministic

---

### Phase 8.1 Exit Criteria

Phase 8.1 is complete when all of the following are true:

- A canonical bid year domain model exists
- Pay periods are derived deterministically
- Invalid bid year definitions fail explicitly
- All logic is pure and side-effect free
- All validation and derivation paths are fully tested
- No persistence or API changes were required
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

## Phase 8.2: Canonical Bid Year Ownership & Persistence

### Phase 8.2 Goal

Phase 8.2 establishes authoritative ownership, persistence, and API surfacing of
canonical bid year metadata so the system no longer relies on placeholder or
inferred values.

This phase makes canonical bid year definition first-class, explicit input.

---

### Phase 8.2 Scope

Phase 8.2 includes:

- Updating the Create Bid Year API to accept canonical bid year metadata
- Persisting canonical bid year metadata as canonical state
- Returning canonical bid year metadata from read endpoints
- Removing placeholder canonical bid year construction from core
- Ensuring all changes remain auditable and deterministic
- Tests for success and failure paths across domain, core, API, and persistence

Phase 8.2 explicitly excludes:

- Leave accrual calculations
- Pay period expansion into persisted per-period rows
- Any bidding logic (bids, rounds, eligibility, capacity)
- Changes to rollback semantics
- Changes to snapshot semantics beyond reflecting canonical state
- UI or frontend work

---

### Phase 8.2 Canonical Bid Year Definition

A canonical bid year is defined by:

- `year: u16`
- `start_date: Date`
- `num_pay_periods: u8` (must be exactly 26 or 27)

These values are the sole authoritative definition of bid year boundaries
and pay period structure.

---

### Phase 8.2 API Requirements

The Create Bid Year endpoint must require explicit canonical metadata.

Create Bid Year requests must include:

- `year`
- `start_date`
- `num_pay_periods`

No defaults are allowed.
No inferred or derived values are allowed.

If any canonical field is missing, the request must fail explicitly.

List Bid Years responses must include canonical metadata:

- `year`
- `start_date`
- `num_pay_periods`

List Bid Years must remain read-only and side-effect free.

---

### Phase 8.2 Core Requirements

- `Command::CreateBidYear` must accept canonical bid year metadata
- Core must construct and validate `CanonicalBidYear` from provided input
- Placeholder canonical validation logic introduced in Phase 8.1.3 must be removed
- Duplicate bid year behavior remains unchanged
- Failure guarantees remain unchanged:
  - no state mutation
  - no audit event emission

---

### Phase 8.2 Persistence Requirements

- Canonical bid year metadata must be stored in canonical state
- Current-state reads must use canonical tables, not snapshots
- Canonical bid year persistence must be transactionally consistent with audit events
- Snapshots must reflect canonical bid year metadata when created

No schema changes beyond what is required to store canonical bid year metadata
are allowed.

---

### Phase 8.2 Audit Requirements

- Successful Create Bid Year emits exactly one audit event
- Audit events must include:
  - actor
  - cause
  - action performed
  - canonical bid year metadata
- Failed Create Bid Year emits no audit event

---

### Phase 8.2 Failure Semantics

Create Bid Year must fail explicitly if:

- `num_pay_periods` is not 26 or 27
- Canonical date arithmetic is invalid or overflows
- The bid year already exists
- Required canonical metadata is missing

Read endpoints must fail explicitly if canonical data is requested for
a non-existent bid year.

List Bid Years must never fail.

---

### Phase 8.2 Testing Requirements

Tests must demonstrate:

- Successful creation with valid canonical metadata (26 and 27 periods)
- Explicit failure for invalid canonical definitions
- Removal of all placeholder canonical logic
- Correct persistence of canonical bid year metadata
- Accurate read-back of canonical metadata
- No audit emission on failure
- Exactly one audit event on success
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes

---

### Phase 8.2 Exit Criteria

Phase 8.2 is complete when all of the following are true:

- Create Bid Year requires canonical metadata
- Placeholder canonical validation logic is fully removed
- Canonical bid year metadata is persisted as canonical state
- List Bid Years returns canonical metadata
- Canonical validation uses only operator-supplied data
- All success and failure paths are fully tested
- Audit semantics remain unchanged
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

## Phase 8.3: Pay Period Alignment & Bid Year Temporal Semantics

### Phase 8.3 Goal

Introduce **authoritative, FAA-aligned temporal semantics** for bid years and pay periods.
This phase formalizes _when_ a bid year starts and ends, ensuring correctness for downstream
leave accrual, eligibility, and historical reconstruction.

Phase 8.3 makes bid year **time boundaries a domain invariant**, not an operator convention.

---

### Phase 8.3 Scope

Phase 8.3 includes:

- Enforcing valid bid year start-date alignment
- Formalizing pay period (PP) boundaries
- Deterministic bid year end-date derivation
- Exposing derived bid year end dates via read APIs
- Updating API tooling to reflect API surface changes

Phase 8.3 explicitly excludes:

- Leave accrual calculations
- Seniority-based logic
- Bidding rounds or bid lifecycle logic
- Capacity, limits, or eligibility rules
- UI or frontend changes
- Performance optimizations

---

### Phase 8.3 Domain Definitions

#### Bid Year Start Date

A bid year start date **must** satisfy all of the following:

- Must be a **Sunday**
- Must occur in **January**
- Does **not** need to be the first Sunday of the year
- Is provided explicitly by the operator (no inference)

Invalid start dates must fail domain validation explicitly.

---

#### Pay Period (PP) Semantics

- A pay period is exactly **14 consecutive days**
- Each pay period:
  - Starts on **Sunday**
  - Ends on **Saturday**
- Pay periods are:
  - Contiguous
  - Non-overlapping
  - Gap-free

---

#### Bid Year Duration

- A bid year consists of **exactly 26 or 27 pay periods**
- The bid year:
  - Starts on the start date of PP #1
  - Ends on the **Saturday** of the final pay period
- The end date:
  - Is **derived**, never stored independently
  - May occur in the following calendar year
  - Is not required to fall in the same year as the start date

---

### Phase 8.3 Validation Rules

The domain must reject bid years where:

- `start_date` is not a Sunday
- `start_date` is not in January
- `num_pay_periods` is not exactly 26 or 27
- Any pay period would:
  - Overflow date arithmetic
  - Break contiguity
  - Violate Sunday–Saturday boundaries

All failures must return **structured, explicit domain errors**.

---

### Phase 8.3 Canonical Model Behavior

- `CanonicalBidYear` remains the authoritative representation
- `CanonicalBidYear::new()` must:
  - Validate start-date alignment
  - Validate pay-period count
  - Derive all pay periods deterministically
  - Derive the bid year end date deterministically
- No inferred or default values are permitted

---

### Phase 8.3 API Changes

#### Read API Enhancements

- Area listing responses must include:
  - `bid_year_end_date` (derived, ISO 8601)
- End date must be derived from canonical bid year data
- No persistence of end dates as standalone fields

#### API Contract Rules

- Any API change (add/remove/rename/modify fields) must:
  - Be reflected in API request/response DTOs
  - Be reflected in `api_cli.py`
  - Maintain consistency across server, API, and tooling layers

---

### Phase 8.3 Tooling Requirements

- `api_cli.py` must be updated whenever:
  - API endpoints change
  - Request schemas change
  - Response schemas change
- CLI updates are considered **required**, not optional
- CLI behavior must remain aligned with the current API surface

---

### Phase 8.3 Audit Semantics

- Audit behavior remains unchanged
- No new audit event types are introduced
- Bid year creation continues to emit exactly one audit event on success
- Validation failures emit no audit events

---

### Phase 8.3 Testing Requirements

Tests must demonstrate:

- Rejection of non-Sunday start dates
- Rejection of non-January start dates
- Acceptance of valid January Sundays
- Correct derivation of:
  - Pay period boundaries
  - Bid year end date
- Deterministic behavior across repeated executions
- Read APIs returning correct derived end dates
- No persistence of derived-only values

All validation paths must be covered by unit tests.

---

### Phase 8.3 Exit Criteria

Phase 8.3 is complete when all of the following are true:

- Bid year start-date alignment is enforced
- Pay period boundaries are validated explicitly
- Bid year end dates are derived correctly
- Derived end dates are exposed via read APIs
- No inferred temporal logic exists
- No persistence schema stores redundant derived values
- `api_cli.py` reflects the current API surface
- All validation and derivation paths are tested
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

## Phase 9: Leave Accrual Calculation (Canonical, Deterministic)

### Phase 9 Goal

Implement a **pure, deterministic leave accrual calculation** for a single user within a single canonical bid year.

Phase 9 establishes the authoritative model for how leave is earned.
No persistence, bidding, or carryover logic is introduced.

---

### Phase 9 Scope

Phase 9 includes:

- Leave accrual calculation for **one user**
- Accrual across **one canonical bid year**
- Pay-period–based accrual logic
- Anniversary-based service thresholds
- 26-PP and 27-PP year handling
- Bonus-hour handling for the 6-hour tier
- Rounding behavior to full leave days
- Rich, auditable calculation output

Phase 9 explicitly excludes:

- Leave bidding
- Leave usage or depletion
- Carryover between years
- Cross-year accrual aggregation
- Persistence or database storage
- API endpoints
- Audit event emission
- Authorization or role logic
- Performance optimization

---

### Phase 9 Inputs

The calculation operates on:

- `User`
  - `service_computation_date (SCD)`
- `CanonicalBidYear`
  - `start_date` (Sunday in January)
  - `num_pay_periods` (26 or 27)
  - Derived pay periods (Sunday → Saturday)

The calculation must not depend on:

- current system time
- external state
- persistence
- API context

---

### Phase 9 Service Threshold Semantics

Years of service are determined using **anniversary-based logic**.

#### Phase 9 Rules

- Service thresholds are crossed **only on or after the calendar anniversary**
  of the user’s SCD.
- Threshold evaluation is based on the **start date of each pay period**.
- If a threshold anniversary occurs **during** a pay period:
  - That entire pay period earns the **prior accrual rate**
  - The new rate applies starting with the **next pay period**

No fractional years, rounding, or day-count division is permitted.

---

### Phase 9 Accrual Rates

Accrual rates are determined by years of service at the start of each pay period.

| Years of Service | Rate per Pay Period |
| ---------------- | ------------------- |
| < 3 years        | 4 hours             |
| ≥ 3 and < 15     | 6 hours             |
| ≥ 15             | 8 hours             |

---

### Phase 9 Bonus Hour Semantics

Users in the **6-hour tier** receive a **flat annual bonus of 4 hours**.

#### Phase 9 Bonus Hour Rules

- The bonus:
  - Is applied **once per bid year**
  - Has **no associated pay period**
  - Exists solely to reach the contractual annual total
- The bonus is **not** modeled as:
  - a virtual pay period
  - a dated accrual event

The bonus must be represented explicitly in the calculation output.

---

### Phase 9 27-Pay-Period Year Handling

For bid years with **27 pay periods**:

- The extra pay period earns leave at the **rate applicable at the start of PP #27**
- No special casing beyond normal PP logic is permitted
- Bonus hours (if applicable) are applied independently

---

### Phase 9 Rounding Rules

After all accrual calculations:

- If the total accrued hours are **not divisible by 8**:
  - The total is **rounded up** to the next full 8-hour day
- The rounding adjustment must be:
  - explicit
  - visible in the output
  - auditable

---

### Phase 9 Output (Rich Model)

The calculation must return a **rich, explainable structure**.

#### Phase 9 Required Output Fields

- Total accrued hours (after rounding)
- Total accrued days
- Whether rounding was applied
- A detailed breakdown explaining **why** the total was reached

#### Phase 9 Conceptual Output Shape

- `total_hours`
- `total_days`
- `rounded_up: bool`
- `breakdown: Vec<PayPeriodAccrual>`

Each breakdown entry must capture:

- Pay period index
- Pay period start date
- Pay period end date
- Accrual rate used
- Hours earned
- Reason (normal, transition, 27th PP, bonus)

The breakdown is part of the domain output and is **not optional**.

---

### Phase 9 Determinism Requirements

- Identical inputs must always produce identical outputs
- No randomness, clocks, or global state are permitted
- The calculation must be:
  - pure
  - side-effect free
  - repeatable

---

### Phase 9 Validation Requirements

The calculation must fail explicitly if:

- Canonical bid year validation fails
- Pay period derivation fails
- Date arithmetic overflows
- Required user fields are missing or invalid

Failures must return structured domain errors.

---

### Phase 9 Testing Requirements

Tests must demonstrate:

- Accrual for users under 3 years
- Accrual for users between 3 and 15 years
- Accrual for users 15+ years
- Transition across:
  - 3-year threshold
  - 15-year threshold
- Transitions occurring mid-pay-period
- Correct bonus hour application
- Correct handling of 27-PP years
- Correct rounding behavior
- Deterministic repeatability
- Rich breakdown correctness

Tests must not rely on persistence, APIs, or audit logs.

---

### Phase 9 Exit Criteria

Phase 9 is complete when all of the following are true:

- Leave accrual is computed correctly for one user and one bid year
- Anniversary-based service thresholds are enforced
- Bonus hours are applied correctly and explicitly
- 27-pay-period years are handled correctly
- Rounding rules are enforced and visible
- Output includes a rich, explainable breakdown
- All logic is pure and deterministic
- All validation and error paths are tested
- No persistence or API changes were introduced
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

## Phase 10: Leave Availability & Balance (Read-Only)

### Phase 10 Goal

Expose **deterministic, explainable leave availability** for a user within a single bid year by combining:

- Canonical leave accrual (Phase 9)
- Recorded leave usage

Phase 10 answers the question:

> _“How much leave does this user have available right now, and why?”_

This phase introduces **no new domain rules**, **no mutation**, and **no bidding behavior**.

---

### Phase 10 Scope

Phase 10 includes:

- Calculation of remaining leave balance for a user
- Read-only aggregation of:
  - Earned leave (from Phase 9)
  - Used leave (from persisted records)
- Deterministic subtraction of usage from rounded entitlement
- Rich, explainable output suitable for operators and audits
- API exposure of leave availability data

Phase 10 explicitly excludes:

- Leave bidding
- Leave reservation or locking
- Carryover between bid years
- Partial-day or fractional usage rules
- Persistence schema changes
- Mutation of leave usage records
- Authorization changes
- Performance optimization

---

### Phase 10 Core Principle

**Accrual is sealed before usage is applied.**

Formally:

earned_hours (Phase 9)
→ apply bonus
→ apply rounding to full 8-hour days
→ subtract used leave
→ available balance

yaml
Copy code

Usage **must never influence accrual or rounding behavior**.

---

### Phase 10 Inputs

For a given user and bid year:

- `CanonicalBidYear`
- `LeaveAccrualResult` (Phase 9 output)
- Set of leave usage records scoped to:
  - the same bid year
  - the same user

All inputs are read-only.

---

### Phase 10 Domain Model

#### Leave Usage

- Leave usage records represent **hours consumed**
- Usage records:
  - Are additive
  - Are immutable once written
  - Are assumed valid for Phase 10
- Phase 10 does **not** validate usage legality

---

#### Leave Availability Result

The core output must include:

- `earned_hours` (rounded, from Phase 9)
- `earned_days`
- `used_hours`
- `remaining_hours`
- `remaining_days`
- `is_exhausted` (remaining_hours == 0)
- `is_overdrawn` (remaining_hours < 0)
- Optional explanatory breakdown

---

### Phase 10 Calculation Rules

- Used hours are summed deterministically
- Remaining hours are calculated as:

remaining_hours = earned_hours - used_hours

csharp
Copy code

- Remaining days are derived as:

remaining_days = remaining_hours / 8

yaml
Copy code

- Negative balances are allowed and surfaced explicitly
- No rounding is applied after usage subtraction

---

### Phase 10 Error Handling

Phase 10 must fail explicitly if:

- Leave accrual data is missing
- Leave usage data cannot be read
- Bid year mismatch occurs between inputs

Errors must be:

- Structured
- Deterministic
- Side-effect free

---

### Phase 10 API Behavior

Read-only API endpoints must expose:

- Total earned leave (hours + days)
- Total used leave
- Remaining available leave
- Breakdown explaining:
  - accrual
  - rounding
  - usage subtraction

API responses must not:

- Mutate state
- Emit audit events
- Infer or recompute accrual logic

---

### Phase 10 Audit Semantics

- No audit events are emitted
- Reads are strictly side-effect free
- Availability queries are observational only

---

### Phase 10 Testing Requirements

Tests must demonstrate:

- Correct subtraction of used leave from rounded accrual
- Deterministic results across repeated calls
- Correct handling of:
  - zero usage
  - partial usage
  - full exhaustion
  - overdrawn balances
- No mutation of state during calculation
- Alignment with Phase 9 accrual outputs

---

### Phase 10 Exit Criteria

Phase 10 is complete when all of the following are true:

- Leave availability can be computed deterministically
- Accrual is never recomputed or altered
- Usage subtraction is explicit and auditable
- Negative balances are surfaced clearly
- API exposes availability data read-only
- No persistence or audit semantics changed
- All calculation paths are tested
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently

## Phase 11: Operator-Centric Read Models & API Ergonomics

### Phase 11 Goal

Design and implement **operator-focused, ergonomic read APIs** that support real-world workflows and UI development **without introducing new domain logic or weakening invariants**.

Phase 11 exists to shape the API around **how the system is used**, not how it is stored or validated.

This phase intentionally responds to UI-driven needs while preserving strict backend authority.

---

### Phase 11 Scope

Phase 11 includes:

- Operator-centric read API design
- Composite and derived read models
- Ergonomic APIs for bootstrapping and operational workflows
- Explicit support for UI-driven data access patterns
- Read-only API refactoring and additions
- API surface changes driven by UX needs
- Updates to API tooling (`api_cli.py`) to match API changes
- Tests validating read correctness and determinism

Phase 11 explicitly excludes:

- Any new domain rules
- Any write semantics changes
- Any persistence model changes
- Any audit or rollback semantic changes
- Any authorization rule changes
- UI implementation itself
- Performance optimization
- Caching or background processing

---

### Phase 11 Core Principles

- **The backend remains the sole authority**
- **The frontend may validate, but never decide**
- **All business rules live in the domain**
- **All state mutation remains auditable**
- **Read APIs may be reshaped freely**
- **No domain invariants may be bypassed**

---

### Phase 11 Active Context Model

- The system operates on **exactly one active bid year at a time**
- Only one bid year may be active in the system
- This invariant must be enforced at the domain or core level
- All operator-facing APIs implicitly or explicitly reference the active bid year
- UI must not manage multiple bid years simultaneously

---

### Phase 11 Read API Ergonomics

Phase 11 allows the introduction of **ergonomic, composite read APIs**, including but not limited to:

- Listing all users in the active bid year
- Listing users by area
- Returning users with derived data (e.g. leave availability)
- Returning area summaries including:
  - user counts
  - bid year metadata
- Returning bid year metadata alongside operational data

These APIs may:

- Join canonical tables
- Compute derived, read-only values
- Aggregate related data
- Flatten nested data for UI consumption

These APIs must NOT:

- Perform state mutation
- Emit audit events
- Depend on mutable in-memory state
- Introduce new business rules

---

### Phase 11 Leave Availability Exposure

- Leave accrual calculations from Phase 9 may be exposed via read APIs
- Leave availability must be:
  - Derived server-side
  - Deterministic
  - Explainable
- UI must not implement accrual logic
- UI may only display or pre-check values for user experience

---

### Phase 11 Frontend Validation Rules

- Frontend validation is permitted for:
  - UX feedback
  - Early error detection
- Frontend validation is NOT authoritative
- Backend validation remains final and required
- All backend validation failures must still be handled gracefully by the UI

---

### Phase 11 API Contract Rules

- API changes are explicitly allowed in this phase
- Any API change must:
  - Update request/response DTOs
  - Update server handlers
  - Update `api_cli.py`
  - Be covered by tests
- No silent or implicit API changes are permitted
- API responses should favor clarity over minimalism

---

### Phase 11 Tooling Requirements

- `api_cli.py` must be updated whenever:
  - Endpoints are added or removed
  - Request schemas change
  - Response schemas change
- Tooling drift is considered a failure condition
- CLI behavior must match the current API surface

---

### Phase 11 Testing Requirements

Tests must demonstrate:

- Correctness of new read APIs
- Deterministic derived values
- No state mutation on read paths
- Accurate leave availability exposure
- Correct handling of active bid year context
- API and CLI consistency

---

### Phase 11 Exit Criteria

Phase 11 is complete when all of the following are true:

- Operator workflows are supported by ergonomic read APIs
- Users can be listed meaningfully without manual API composition
- Leave availability is accessible via read APIs
- Active bid year context is enforced consistently
- No domain rules were added or modified
- No persistence semantics were changed
- No audit behavior was altered
- `api_cli.py` reflects the current API surface
- All read paths are fully tested
- `cargo xtask ci` passes consistently
- `pre-commit run --all-files` passes consistently
