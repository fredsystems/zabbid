# Phase 30 — UI Enablement and End-to-End Validation for Phase 29

## Purpose

Phase 30 delivers the **admin UI workflows** required to operate everything introduced in Phase 29 and provides **end-to-end validation** that the pre-bid system can be driven entirely through the UI without manual DB intervention.

This phase is explicitly the UI companion to Phase 29:

- Phase 29 introduced/locked down domain concepts, persistence, API, and lifecycle mechanics.
- Phase 30 makes those concepts **usable**, **auditable**, and **verifiable** via the admin interface.

---

## Core Outcomes

By the end of Phase 30, an operator can:

1. Complete bootstrap using a **structured, task-oriented UI**
   (not a single, monolithic page).

2. Configure **per-area pre-bid requirements**, including:
   - round group assignment
   - round definitions and sequencing
   - expected user counts
   - participation and calculation flags

3. Review all **No Bid (system area) users** and explicitly confirm
   their disposition:
   - reassigned to a competitive area, or
   - confirmed to remain in No Bid

4. View and validate **bid order per area**, with the ability to:
   - detect blocking seniority conflicts
   - understand _why_ readiness is blocked
   - resolve issues by correcting inputs (not reordering)

5. Declare the **bid schedule**, including:
   - authoritative bid time zone
   - bid start date
   - daily bid window
   - bidders per area per day
     using timezone-aware, DST-safe semantics.

6. Confirm **“Ready to Bid”** through an explicit, irreversible UI action,
   transitioning the system into the bidding lifecycle.

7. After confirmation:
   - view the **frozen canonical bid order** and derived bid windows
   - perform **explicit, controlled administrative adjustments**
     (where permitted by Phase 29), without waterfalling or implicit
     reordering effects.

### Phase 29 Alignment Note

If any capability described above depends on Phase 29 scope and was
_not_ implemented during Phase 29 execution, that gap **must be
explicitly recorded** in the Phase 30 working state document, including:

- the missing capability
- the reason it was deferred
- any constraints this imposes on Phase 30 behavior

This ensures auditability of scope decisions and prevents silent
assumptions in the UI layer.

---

## Non-Negotiable UI Invariants

### 1) Lifecycle-Driven Edit Locks

- Before confirmation: all Phase 29 inputs are editable (except where already immutable by prior rules).
- After confirmation: editing locks must engage everywhere per Phase 29 (structural edits forbidden).
- The UI must not offer controls that will always be rejected by API due to lifecycle constraints.

### 2) Bid Order: Read-only Proof + Post-Confirm Adjustments (if enabled)

- Pre-confirm: bid order is derived and viewable; no manual reordering.
- At confirmation: bid order + computed bid windows are frozen.
- Post-confirm: if Phase 29 introduced adjustment endpoints, UI must expose them explicitly as administrative actions with audit transparency.

### 3) No Bid Review Is a Required Gate

- If No Bid contains users, readiness must show as blocked until review is complete.
- UI must provide review controls:
  - move user to area, OR
  - confirm user remains in No Bid

### 4) Participation Flags Are First-Class

- Excluded-from-bidding and excluded-from-leave-calculation are visible and editable pre-confirm.
- UI must enforce/communicate the directional invariant:
  - excluded_from_leave_calculation => excluded_from_bidding

### 5) Timezone + DST Semantics Are “Wall Clock”

- UI must treat bid schedule times as wall-clock values in selected timezone.
- UI must not imply elapsed-hour semantics across DST boundaries.

---

## Scope

### A. Admin Bootstrap UI Restructure

Replace the “147 miles long” bootstrap page with a multi-step or multi-section workflow.

Required UX:

- A left-nav or stepper with clear progress:
  1. Bid Year Setup
  2. Areas Setup
  3. Users Import / Add / Edit
  4. No Bid Review
  5. Round Groups + Rounds Setup
  6. Area Assignment (exactly one round group per non-system area)
  7. Bid Order View
  8. Bid Schedule Declaration
  9. Readiness Review + Confirm Ready to Bid

This can be one route with internal sections, or separate routes. Either is fine, but it must not be a single scrolling wall.

### B. Round Groups + Rounds UI

Provide UIs to:

- create/edit round groups (and their rounds)
- view round group contents (round sequence)
- assign exactly one round group to each non-system area
- show clear validation errors if assignment is missing or invalid

Constraints:

- system areas have no round group assignment
- round config is immutable after confirmation

### C. Bid Order UI (Read-only) + Conflict Visibility

Provide a viewable per-area bid order page:

- sortable/filterable by area
- shows deterministic order, including tie-breaker metadata inputs (as display)
- clearly shows whether order is “derived” or “frozen”
- if conflicts can exist at readiness time, UI must show the conflict and the blocking reason

No manual reordering pre-confirm.

### D. No Bid Review UI

Provide a dedicated screen for No Bid user review:

- bulk filters
- per-user “confirmed remains in No Bid” toggle OR “move to area” dropdown
- shows readiness impact (“X users remain unreviewed”)

### E. Bid Schedule Declaration UI

Provide a UI to set:

- timezone (IANA)
- start date (date only, Monday, future at time of confirm)
- daily window start/end time (wall-clock)
- bidders per area per day

Rules:

- values are required at confirmation
- values are mutable prior to bidding commencement (per Phase 29 semantics)
- every change should be auditable (UI should show “last changed” metadata if available)

### F. Readiness Review UI + Confirmation Action

Provide:

- computed readiness state (“Domain-Ready” / “Blocked”)
- explicit list of blockers (actionable)
- confirmation UI action (irreversible)

Confirmation must:

- require deliberate acknowledgement
- show a summary of critical frozen inputs at confirmation time
- surface the irreversibility clearly

### G. Post-Confirmation Views

After confirmation:

- show frozen bid order + bid windows (per area)
- show bid status table (if created in Phase 29)
- show which fields are locked

If Phase 29 includes “adjust bid order / windows” endpoints:

- provide explicit administrative screens for them
- require confirmation for adjustments
- show that adjustments do not waterfall others automatically (gaps/overlaps permitted)

### H. UI Validation & E2E Smoke Tests

Add end-to-end validation that:

- full bootstrap flow is operable through UI + API
- readiness gates behave correctly
- confirmation locks engage and remain enforced
- No Bid review gate works
- bid schedule declaration rules enforced (Monday/future/timezone required)

Tests can be:

- Playwright (preferred if you already have it)
- or API-level integration tests + minimal UI smoke tests if Playwright is too heavy right now

The goal is to catch “works in unit tests, fails when clicked” issues.

---

## Explicit Non-Goals

- Do not implement actual bidding execution.
- Do not implement time-driven automation (auto mark missed, auto proxy execution).
- Do not redesign styling beyond what is needed to support new UI flows.
- Do not refactor unrelated UI components unless required for correctness.

---

## Files Likely to Change

- `ui/src/components/admin/**`
- `ui/src/routes/**` (or equivalent routing/pages)
- `ui/src/api/**` client wrappers
- Possibly shared UI components: tables, forms, steppers, cards
- Possibly `AGENTS.md` UI guidance if new patterns are introduced

---

## Completion Conditions

Phase 30 is complete when:

- Bootstrap UI is segmented and usable with 200+ users (not a single long page).
- All Phase 29 configuration is operable through the UI:
  - participation flags
  - No Bid review
  - round groups/rounds config
  - area assignment to exactly one round group
  - bid order viewing
  - bid schedule declaration
  - readiness + confirm ready to bid
- UI accurately reflects lifecycle locks pre/post confirmation.
- At least one end-to-end validation path exists that exercises the full pre-bid workflow.
- `cargo xtask ci` passes
- `pre-commit run --all-files` passes

---

## When to Stop and Ask

Stop immediately if:

- UI requirements imply changing Phase 29 domain invariants.
- confirmation semantics become reversible.
- DST/timezone rules would be simplified into fixed-duration offsets.
- post-confirm editing would require structural data mutation (should be forbidden).

These indicate a scope or invariant mismatch, not a UI task.
