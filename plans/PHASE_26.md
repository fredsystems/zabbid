# Phase 26 — Lifecycle-Aware Administrative Editing & Review

## Objective

Make the system **operable by humans** without undermining the canonical and lifecycle guarantees established in Phase 25.

Phase 26 does **not** change core domain rules. Instead, it exposes, enforces, and explains them through explicit, lifecycle-aware administrative editing and review workflows.

This phase answers the question:

> “Given the current lifecycle state, what _can_ I change — and why?”

---

## Core Principle

> Canonical truth is protected — but human intent must still be expressible.

Phase 26 focuses on **editing surfaces**, not domain reshaping.

---

## Context (Post Phase 25)

The system now has:

- Explicit bid-year lifecycle states
- Canonical tables as authoritative truth post-canonicalization
- Override semantics with audit trails
- A formalized No Bid system area
- Lifecycle-aware backend enforcement

What is **missing** is clarity and tooling for administrators to:

- Review state
- Understand constraints
- Perform allowed edits safely
- Apply overrides intentionally

---

## In Scope

### 1. Lifecycle-Aware Editing Surfaces

Expose editing capabilities that already exist in the domain, gated by lifecycle state.

#### User Editing (Non-Structural)

Allowed across all lifecycle states unless otherwise noted:

- Edit user name
- Edit user initials
- View:
  - canonical values
  - overridden values
  - override reasons
  - audit history

Area reassignment:

- Pre-canonicalization: normal edit
- Post-canonicalization: explicit override workflow (already implemented in Phase 25D)

No new domain rules are introduced here — only correct routing and UI surfacing.

---

### 2. Area Editing (Operational Metadata Only)

Allowed **pre-canonicalization only**:

- Edit area display name
- Edit expected user count
- View actual vs expected user count

Not allowed:

- Delete system areas
- Rename system areas
- Delete or structurally alter areas post-canonicalization

UI must clearly communicate:

- Why an action is disabled
- Which lifecycle rule applies

---

### 3. Bid Year Editing (Metadata & Control)

Allowed:

- View lifecycle state and history
- View canonicalization status
- View blocking conditions
- Trigger lifecycle transitions (already implemented)

Allowed metadata edits (pre-canonicalization only):

- Labels
- Notes
- Non-structural descriptive fields

Not allowed:

- Structural reshaping post-canonicalization

---

### 4. No Bid Area Review Workflow

Phase 26 must address the **operational reality** of No Bid.

#### Required Fixes & Clarifications

- No Bid **must not** count toward expected area count
- No Bid **may legitimately have zero users**
- Zero users in No Bid is **success**, not an error
- Other areas may not have zero users unless explicitly allowed

#### Review Workflow

Administrators must be able to:

- View users currently in No Bid
- See _why_ No Bid blocks bootstrap
- Explicitly mark users as “reviewed” (mechanism to be defined)
- Assign users out of No Bid once reviewed

Bootstrap completion must remain blocked until:

- No users remain in No Bid
- Or users are explicitly acknowledged (if that rule is adopted)

This workflow must be explicit and auditable.

---

### 5. Override Visibility (Read-Only)

Phase 26 does **not** add new override mechanics.

It must:

- Clearly display overridden fields
- Show override reason
- Link to audit event
- Make it obvious that a value is overridden

No override execution UI is required beyond what already exists.

---

## Explicit Non-Goals

Phase 26 does **not** include:

- New bidding logic
- Round management
- Proxy bidding
- Websocket live bidding
- Bulk editing
- Import tooling
- Performance optimization

---

## Editing Rules Summary

| Lifecycle State   | Editing Allowed                            |
| ----------------- | ------------------------------------------ |
| Draft             | Full editing                               |
| BootstrapComplete | Full editing                               |
| Canonicalized     | Canonical overrides only                   |
| BiddingActive     | Restricted edits (names, notes, overrides) |
| BiddingClosed     | Read-only                                  |

User name and initials remain editable in all states.

---

## UX Requirements

- Disabled actions must explain _why_
- Lifecycle state must be visible everywhere relevant
- System vs operational entities must be clearly labeled
- Canonical vs overridden data must be distinguishable
- Bootstrap blockers must be actionable, not just visible

---

## Testing Expectations

Required:

- Lifecycle-aware enable/disable tests
- No Bid review behavior tests
- Editing permission enforcement
- Override visibility tests

Not required:

- Bidding execution tests
- Time-based simulations
- Concurrency testing

---

## Exit Criteria

Phase 26 is complete when:

- Administrators can clearly see what is editable and why
- All allowed edits are surfaced and functional
- Forbidden edits are disabled with explanation
- No Bid workflow is operable and intuitive
- Canonical and override data are transparent
- No domain invariants from Phase 25 are weakened

---

## Why Phase 26 Matters

Phase 25 made the system **correct**.

Phase 26 makes the system **usable**.

Without this phase:

- Admins fight the system
- Rules feel arbitrary
- Overrides feel dangerous instead of intentional

After Phase 26:

- Human judgment is supported
- Canonical truth remains intact
- Bidding logic can be built on solid ground
