# Phase 25 — Canonicalization, Locking, and Override Semantics

## Objective

Introduce a clear, auditable transition from a **computed / bootstrap world** to an **authoritative / operational world**, while preserving the ability for administrators to apply human judgment and corrective action without breaking the system.

This phase establishes **when data stops being derived**, **what becomes canonical**, and **how controlled exceptions are handled**.

This phase **does not implement bidding logic**. It prepares the ground so bidding can be implemented safely and flexibly later.

---

## Core Principle

> Everything is computed until it is locked.
> Once locked, the system operates only on canonical data — but canonical data is allowed to be edited, with audit and intent.

This principle exists to prevent rigidity without sacrificing determinism.

---

## Current State (Post Phase 24)

- Persistence layer is backend-agnostic and stable
- Canonical IDs exist for users, areas, and bid years
- Bootstrap completeness is enforced
- Editing rules are informal and implicit
- Bid-year lifecycle is not explicitly modeled

---

## Goals

1. Make bid-year lifecycle state explicit and enforced
2. Clearly separate **derived data** from **canonical data**
3. Define when and how canonical data is materialized
4. Allow post-lock edits without breaking domain rules
5. Preserve full auditability for all overrides
6. Avoid encoding bidding rules prematurely

---

## Non-Goals

This phase explicitly does **not** include:

- Bid execution
- Round slot enforcement
- Proxy bid processing
- Websocket live bidding
- UI workflows for bidding
- Performance optimization

---

## Phase Structure

### 1. Bid-Year Lifecycle State Machine

Introduce an explicit bid-year state enum, persisted in the database:

- `Draft`
- `BootstrapComplete`
- `Canonicalized`
- `BiddingActive`
- `BiddingClosed`

Rules:

- Transitions are explicit domain actions
- Invalid transitions are rejected
- State is consulted by domain logic, not UI hints

This replaces implicit assumptions with enforceable truth.

---

### 2. Derived vs Canonical Data Definition

Formally define which data is derived and which is canonical.

#### Derived (pre-canonicalization)

- Bid order (computed from seniority)
- Bid windows
- Eligibility
- Area membership defaults
- Round structure (future phase)

#### Canonical (post-canonicalization)

- Canonical bid order
- Canonical bid windows
- Canonical user eligibility flags
- Canonical area membership
- Canonical round definitions (future)

This distinction must be reflected in:

- Table naming
- API boundaries
- Domain logic

---

### 3. Canonicalization Action

Introduce a **single, explicit domain action**:

#### Canonicalize Bid Year

This action:

- Requires `BootstrapComplete`
- Computes all required derived data
- Writes canonical tables
- Locks further derivation
- Transitions bid year to `Canonicalized`

After this point:

- Derived computations are no longer used
- All reads come from canonical tables

This is the most important domain action in the system.

---

### 4. Editing & Locking Semantics

Define editing rules based on bid-year state:

| State             | Editing Allowed       |
| ----------------- | --------------------- |
| Draft             | Full editing          |
| BootstrapComplete | Full editing          |
| Canonicalized     | Canonical edits only  |
| BiddingActive     | Restricted edits only |
| BiddingClosed     | Read-only             |

Examples:

- User name and initials are always editable
- Area deletion is prohibited post-canonicalization
- User reassignment requires canonical override
- Expected user count auto-adjusts pre-lock only

---

### 5. Override Semantics (Critical)

Introduce **override-aware canonical fields**, not special-case logic.

Pattern:

- Canonical value
- Optional override value
- Override reason
- Audit event

Rules:

- Overrides are explicit, never implicit
- Overrides never recompute derived data automatically
- Overrides become the source of truth once applied

This preserves flexibility without undermining determinism.

---

### 6. “No Bid” Area Formalization

Define “No Bid” as a first-class canonical area with explicit rules:

- Always exists
- Hidden from unauthenticated users
- Has no rounds or limits
- Used as:
  - Import fallback
  - Deletion sink (pre-bidding only)
  - Manual review staging area

Bootstrap completion requires:

- Manual confirmation of all users in No Bid

#### No Bid Review Semantics

- Having users in No Bid is **not inherently invalid**
- Bootstrap completion requires **explicit review**, not automatic emptiness
- Review means:
  - An administrator has consciously confirmed that remaining users are intentionally unassigned
- The system may block bootstrap until review is acknowledged
- Review does NOT imply assignment

UI workflows may distinguish:

- "Users present and unreviewed" (blocking)
- "Users present but reviewed" (allowed)

---

### 7. Audit & Observability Guarantees

Every canonicalization and override must:

- Emit an audit event
- Capture intent and reason
- Be replayable in event history

This ensures:

- Trust
- Debuggability
- Post-hoc justification for decisions

---

## Testing Expectations

Required:

- State transition validation
- Canonicalization idempotency tests
- Override application tests
- Editing permission enforcement

Not required:

- Time-based bidding tests
- Slot exhaustion logic
- Concurrency testing

---

## Exit Criteria

Phase 25 is complete when:

- Bid-year lifecycle is explicit and enforced
- Canonicalization exists as a single domain action
- Canonical tables are the sole source of truth post-lock
- Overrides are possible, auditable, and intentional
- No bidding logic depends on derived data
- System remains flexible without being permissive

---

## Why This Phase Matters

This phase determines whether the system:

- Supports human judgment
- Or fights it at every turn

By freezing derivation but not authority, the system gains:

- Determinism where it matters
- Flexibility where reality demands it

After this phase, bidding logic becomes straightforward because it operates on **truth**, not assumptions.
