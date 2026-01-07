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

---

## Phase 1: Minimal Domain Logic

**Goal:** Implement the smallest set of domain rules necessary to validate the system design.

### Phase 1: Objectives

- Implement one or two representative domain rules
- Validate rule enforcement paths (success and failure)
- Ensure auditability of all rule-driven transitions

### Phase 1: Non-Goals

- Complete rule coverage
- Performance optimization
- UI concerns

### Phase 1: Exit Criteria

- Rules are validated and tested
- Invalid actions fail explicitly
- Audit trails are complete and immutable

---

## Phase 2: API Skeleton

**Goal:** Expose a minimal API surface without committing to final shapes.

### Phase 2: Objectives

- Define request/response types
- Validate inputs server-side
- Ensure API reflects domain authority

### Phase 2: Non-Goals

- Frontend integration
- Authentication/authorization
- Backwards compatibility guarantees

### Phase 2: Exit Criteria

- API endpoints compile and pass tests
- API does not leak domain internals
- Errors are structured and meaningful

---

## Phase 3: Infrastructure & Persistence

**Goal:** Introduce persistence while preserving invariants.

### Phase 3: Objectives

- Introduce database schema
- Ensure persistence preserves auditability
- Integrate test database infrastructure

### Phase 3: Non-Goals

- Performance tuning
- Schema optimization

### Phase 3: Exit Criteria

- State can be persisted and restored
- Audit history remains immutable
- Database-backed tests pass in CI

---

## Phase 3: Working Rules

- Work must stay within the current phase
- If a requirement appears to belong to a future phase, stop and ask
- Phases may be refined or reordered intentionally
- Do not speculate about later features

---

## When to Update This Plan

- After completing a phase
- When a phase proves insufficient
- When assumptions change

Changes to this plan should be explicit and intentional.
