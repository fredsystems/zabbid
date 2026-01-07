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

crates/
domain/ # Pure domain types and rule validation
audit/ # Audit record types and invariants
core/ # State transitions and orchestration

yaml
Copy code

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

## When to Update This Plan

- After completing a phase
- When a phase proves insufficient
- When assumptions change

Changes to this plan should be explicit and intentional.
