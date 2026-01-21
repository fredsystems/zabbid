# Phase 30 — UI Enablement and End-to-End Validation for Phase 29

## Purpose

Phase 30 delivers the **administrative UI workflows** required to operate
everything introduced in Phase 29 and provides **end-to-end validation**
that the pre-bid system can be driven entirely through the UI without
manual database intervention.

This phase is the **UI and operability companion** to Phase 29:

- Phase 29 defined and enforced domain concepts, persistence, APIs, and lifecycle mechanics.
- Phase 30 makes those concepts **usable**, **auditable**, and **verifiable**
  by a real operator.

This phase exists to answer one question conclusively:

> Can a real human take the system from empty → confirmed ready to bid,
> using only supported UI and API paths, without hacks?

---

## Core Outcomes

By the end of Phase 30, an operator can:

1. Complete bootstrap using a **structured, task-oriented UI**
   (not a single monolithic page).

2. Configure **per-area pre-bid requirements**, including:
   - round group selection
   - round sequencing
   - expected user counts (non-system areas only)
   - participation and leave-calculation flags

3. Review all **No Bid (system area) users** and explicitly confirm
   their disposition:
   - reassigned to a competitive area, or
   - confirmed to remain in No Bid

4. View and validate **bid order per area**, with the ability to:
   - see deterministic ordering
   - detect blocking seniority conflicts
   - understand _why_ readiness is blocked
   - resolve issues by correcting inputs (not reordering)

5. Manage users pre-canonicalization:
   - Delete users
   - Modify all metadata
   - Reassign area

6. Declare the **bid schedule**, including:
   - authoritative bid time zone
   - bid start date
   - daily bid window
   - bidders per area per day
     using timezone-aware, DST-safe, wall-clock semantics.

7. Confirm **“Ready to Bid”** through an explicit, irreversible UI action
   that transitions the system into the bidding lifecycle.

8. After confirmation:
   - view the **frozen canonical bid order** and derived bid windows
   - perform **explicit, controlled administrative adjustments**
     (where permitted by Phase 29),
     without waterfalling or implicit reordering.

---

## Phase 29 Alignment Note

If any capability described above depends on Phase 29 scope and was
**not implemented** during Phase 29 execution, that gap **must be explicitly
recorded** in the Phase 30 working state document, including:

- the missing capability
- why it was deferred
- any constraints this imposes on Phase 30 behavior
- a clear **stop-and-await-guidance** marker

Phase 30 **must not** silently paper over Phase 29 domain gaps or simulate
missing behavior in the UI.

---

## Non-Negotiable UI Invariants

### 1. Lifecycle-Driven Edit Locks

- Before confirmation:
  - all Phase 29 inputs are editable (except where already immutable).
- After confirmation:
  - structural edit locks must engage everywhere.
- The UI must not offer controls that will always be rejected by lifecycle rules.

### 2. Bid Order Semantics

- Pre-confirm:
  - bid order is **derived, read-only, and informational**.
- At confirmation:
  - bid order and bid windows are **frozen canonically**.
- Post-confirm:
  - only explicitly allowed administrative adjustments may be offered.
  - no implicit reordering or waterfalling.

### 3. No Bid Review Gate

- No Bid is a **system area**.
- No Bid having **zero users is valid**.
- No Bid must **never** block readiness due to expected counts.
- If No Bid contains users:
  - readiness is blocked until review is complete.
- UI must provide:
  - per-user confirmation to remain in No Bid, or
  - reassignment to a competitive area.

### 4. Participation Flags Are First-Class

- `excluded_from_bidding` and `excluded_from_leave_calculation` are:
  - visible
  - editable pre-confirm
  - immutable post-confirm
- UI must communicate and enforce the invariant:
  - excluded_from_leave_calculation ⇒ excluded_from_bidding

### 5. Time Semantics Are Wall-Clock

- All bid schedule values are wall-clock values in the selected timezone.
- UI must not imply elapsed-duration semantics across DST boundaries.
- UI must not hide or simplify timezone implications.

---

## Scope

### A. Admin Bootstrap UI Restructure

Replace the single, long bootstrap page with a **structured workflow**.

Required sections (route-based or sectional):

1. Bid Year Setup
2. Areas Setup
3. Users Import / Add / Edit
4. No Bid Review
5. Round Groups + Rounds Setup
6. Area → Round Group Assignment (exactly one per non-system area)
7. Bid Order View
8. Bid Schedule Declaration
9. Readiness Review + Confirm Ready to Bid

The operator must always know:

- where they are
- what is blocking readiness
- what remains to be done

---

### B. Round Groups + Rounds UI

Provide UI to:

- create and edit round groups
- define round sequencing within a group
- assign exactly one round group to each non-system area

Constraints:

- system areas have no round group
- round configuration is immutable after confirmation
- validation errors must be explicit and actionable

---

### C. Bid Order UI (Read-Only)

Provide a per-area bid order view that:

- shows deterministic ordering
- displays tie-breaker inputs for transparency
- clearly indicates whether the order is:
  - derived (pre-confirm), or
  - frozen (post-confirm)

No manual reordering pre-confirm.

---

### D. No Bid Review UI

Provide a dedicated review screen that:

- lists all users in No Bid
- supports bulk filtering
- allows:
  - confirmation to remain in No Bid, or
  - reassignment to a competitive area
- shows readiness impact in real time

---

### E. Bid Schedule Declaration UI

Provide UI to declare:

- bid time zone (IANA)
- bid start date (date-only, Monday, future at confirm time)
- daily bid window start/end (wall-clock)
- bidders per area per day

Rules:

- all values required at confirmation
- values mutable prior to bidding commencement
- UI should surface audit metadata if available

---

### F. Readiness Review + Confirmation UI

Provide:

- a computed readiness state
- explicit list of blockers
- an irreversible confirmation action

Confirmation must:

- require deliberate acknowledgement
- summarize frozen inputs
- clearly communicate irreversibility

---

### G. Post-Confirmation Views

After confirmation:

- show frozen bid order and bid windows
- show bid status table (if present from Phase 29)
- visually indicate locked fields

If Phase 29 allows adjustments:

- expose them explicitly
- require confirmation
- show that adjustments do not cascade implicitly

---

### H. End-to-End Validation & API Surface Audit

This phase must conclude with two validation passes:

#### 1. End-to-End UI Validation

Demonstrate that:

- full bootstrap is possible via UI + API
- readiness gates behave correctly
- No Bid review behaves correctly
- confirmation locks engage and persist

This may be manual, scripted, or automated.

#### 2. API Surface Audit (Deliverable)

At the end of Phase 30:

- enumerate **all active API endpoints**
- remove endpoints no longer reachable or used
- remove associated dead code where applicable
- produce a new documentation artifact:
  - `docs/api.md`

This document must list:

- endpoint method + path
- purpose
- lifecycle status (active / future / internal)

**Process rules for keeping this document up to date are explicitly
out of scope for Phase 30** and will be introduced in a later phase.

---

## Explicit Non-Goals

- No bidding execution logic
- No time-driven automation
- No styling redesign beyond structural usability
- No domain invariant changes from Phase 29
- No process-rule changes to AGENTS.md or phase_execution.md

---

## Completion Conditions

Phase 30 is complete when:

- Bootstrap UI is structured and usable with 200+ users
- All Phase 29 configuration is operable through UI
- No Bid zero-user state is treated as valid
- Readiness and confirmation semantics are faithfully represented
- Bid order and schedule are viewable and correct
- A full API surface audit is complete
- `docs/api.md` exists and reflects reality
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes
- You remind the user to update agents.md and other documents to encode api rules

---

## When to Stop and Ask

Stop immediately if:

- UI requirements imply changing Phase 29 invariants
- confirmation becomes reversible
- DST semantics would be simplified incorrectly
- UI requires post-confirm structural mutation

These indicate scope or invariant violations, not UI work.
