# Phase 29 — Pre-Bid Readiness, Ordering, and Bootstrap Finalization

## Purpose

Phase 29 completes **all structural, domain, and configuration work required before bidding may begin**.

This phase exists to ensure that once the system enters bidding:

- no structural changes are needed
- no bootstrap concepts need to be revisited
- no manual correction mechanisms are required
- all domain rules required for bidding are already enforced

Phase 29 intentionally front-loads correctness to avoid fragile fixes later.

---

## Core Invariants (Non-Negotiable)

### 1. Canonical Identity

- Users are identified **only** by `user_id`
- Areas and bid years are identified **only** by their canonical IDs
- No mutable display attribute is ever used for selection or mutation

---

### 2. Seniority Must Be Total and Conflict-Free

> There must never be a seniority conflict.

- Bid order determination **must always produce a strict total ordering**
- No two users may resolve to the same seniority position
- There is **no manual resolution**, override, or UI-based correction path
- Any seniority conflict is a **domain violation**

If a conflict occurs:

- the system **must refuse readiness**
- the error must be explicit and blocking
- the underlying data or logic must be fixed

The bid order UI is **proof of correctness**, not a correction tool.

---

### 3. Explicit, Irreversible Transition to Bidding

- The system must never auto-enter bidding
- A manual confirmation action is required to enter bidding
- This action is **irreversible**
- After confirmation, structural editing locks engage

---

### 4. System Areas Are Excluded from Competition

- System-designated areas (`is_system_area = true`, including No Bid):
  - do not count toward expected area totals
  - do not participate in rounds or limits
  - do not block readiness
- Users in system areas **may still bid**, but:
  - do not compete against others
  - are not subject to round constraints

---

## User Participation Flags (Explicit Domain Modeling)

Each user has two explicit, independent participation flags:

- `excluded_from_bidding`
- `excluded_from_leave_calculation`

These flags are:

- first-class domain data
- auditable
- editable prior to confirmation
- immutable once bidding begins

These flags affect:

- bid order derivation
- readiness evaluation
- round capacity calculations

They do not:

- perform actions automatically
- trigger execution behavior
- imply time-based transitions

### Directional Invariant (Non-Negotiable)

A user may **never** be included in bidding while excluded from leave calculation.

Formally:

```text
excluded_from_leave_calculation == true
⇒ excluded_from_bidding == true
```

The reverse is permitted.

Any violation blocks readiness.

---

### Flag Semantics

#### Excluded From Leave Calculation

If `excluded_from_leave_calculation = true`:

- the user does **not** count toward:
  - area leave capacity
  - maximum bid slots
  - group availability limits
- the user **must** also be excluded from bidding

#### Excluded From Bidding

If `excluded_from_bidding = true`:

- **pre-confirmation**:
  - user is excluded from bid order derivation
  - no bid window is assigned
- **post-confirmation**:
  - the user retains a slot in the frozen order
  - the slot is skipped
  - no waterfall or compaction occurs

This preserves schedule integrity.

---

## Scope

### A. Canonical Area and Bid Year Identity

- Areas use unique IDs everywhere
- Bid years use unique IDs everywhere
- No lookups by name or code outside display or validation contexts

---

### B. No Bid (System Area) Semantics

Introduce a system-designated area with the following rules:

- Identified by `is_system_area = true`
- Canonical name: “No Bid” (name is not semantically significant)
- Characteristics:
  - not visible to unauthenticated users
  - excluded from expected area counts
  - has no rounds or limits
  - users may bid, but do not compete

#### Expected User Count Semantics

- System-designated areas:
  - **do not have an expected user count**
  - **must never block readiness**
- Expected count rules apply **only** to non-system areas

No Bid’s population is irrelevant to bootstrap completeness.

#### Domain Behaviors

- Users without an area during import:
  - are automatically assigned to No Bid
- If an area is deleted:
  - users are reassigned to No Bid
- Bootstrap is **not complete** until:
  - all No Bid users have been reviewed

Review actions:

- move user to a non-system area
- explicitly confirm they remain in No Bid

---

### C. Bootstrap Editing Model (Pre-Bid)

Until bidding is explicitly confirmed:

- **All structural data is editable**, including:
  - areas
  - bid years
  - users
  - participation flags
  - rounds
  - round groups
  - expected user counts
  - seniority inputs

#### Expected Count Adjustment Rule

If actual users exceed an area’s expected count:

- the expected count is automatically incremented
- readiness is not blocked

Editing exists to reach correctness, not violate it.

---

### D. Rounds and Round Groups

Introduce round configuration required for bidding logic.

#### Round Group

A round group defines bidding rules and is identified by:

- unique ID
- canonical visible name
- editing-enabled flag

Round groups:

- may be reused across rounds
- define rule sets
- are referenced by ID, not name

#### Round Definition

Each round includes:

- unique ID
- canonical name
- order number
- round group ID
- slots per day
- maximum groups
- maximum total hours
- holiday behavior
- overbid policy

Rules:

- groups are up to 5 consecutive days
- RDOs are excluded from group length
- skipped days split groups
- No Bid area has no rounds

Bootstrap is **not complete** until all non-system areas have round groups.

---

### E. Overbid Rules (Configuration Only)

Each round must declare one of:

1. **No Overbid Allowed**

   ```text
   min(
     round.max_groups / round.max_hours,
     remaining_accrued_leave
   )
   ```

2. **Overbid Allowed (Carryover Round)**

- accrued leave limits ignored
- round limits still apply

This is configuration only in Phase 29.

---

### F. Bid Order Determination and Freezing

#### Pre-Confirmation

- Bid order is derived
- Recomputed on input change
- Read-only
- Non-authoritative

#### Pre-Confirmation Review

- Derived bid order is computed and **visible via preview API**
- Operators may review ordering before confirmation
- No irreversible action occurs without operator visibility
- Readiness evaluation **includes real bid order computation**

#### At Confirmed Ready to Bid

At confirmation:

- bid order is **materialized** per area
- each record includes:
  - `user_id`
  - order position
  - bid window start
  - bid window end
- this becomes the authoritative schedule
- derive and store canonical bid order + windows

#### Post-Confirmation Adjustments

Administrators may:

- explicitly reorder users
- adjust bid windows

Constraints:

- seniority data is never changed
- no recomputation occurs
- no waterfall effects
- changes apply to:
  - current round (if not yet bid)
  - all future rounds

All adjustments are auditable.

This supports operational reality without corrupting domain truth.

---

### G. Bid Windows and Status Tracking (Structure Only)

- bid start date (Monday)
- M–F weeks
- daily window (default 08:00–18:00)
- bidders per area per day
- Bid status transitions are operator-initiated only.
- The system never advances status based on time alone.

Statuses tracked per user, per round, per area:

- Not Started (pre-window)
- Not Started (in window)
- In Progress
- Completed (on time)
- Completed (late)
- Missed (no call / management pause)
- Voluntarily Not Bidding
- Proxy

Status history begins **only after confirmation**.

---

### H. Readiness vs. Confirmation

#### Domain-Ready (Computed)

The system evaluates:

- all non-system areas exist
- all non-system areas have round groups
- expected counts satisfied (non-system only)
- all No Bid users reviewed
- no seniority conflicts
- participation flag invariants satisfied
- overbid rules defined

Passing these checks does **not** enter bidding.

---

#### Confirmed Ready to Bid (Manual, Irreversible)

Once Domain-Ready:

- administrator explicitly confirms
- bidding lifecycle begins
- editing locks engage
- bid order history starts
- action cannot be undone
- derive and store canonical bid order + windows

---

### Bid Schedule Declaration

As part of confirmation, the administrator must declare:

#### Bid Time Zone

- explicit IANA identifier
- authoritative local wall-clock context
- no implicit defaults

#### Bid Start Date

- date only
- future
- Monday
- interpreted in declared time zone

#### Daily Bid Window

- start time
- end time
- uniform across areas
- wall-clock times

#### Bidders Per Area Per Day

- integer
- used to derive windows later

---

### Time Semantics (Normative)

- All bid times are wall-clock times
- Nominal labels define windows
- DST:
  - does not shift labels
  - may change duration
  - must never make users early or late

Execution logic **must** use timezone-aware arithmetic.

---

### I. Deployment via Docker Compose

Provide a `docker-compose.yml` including:

- MariaDB
- backend
- UI
- NGINX (no SSL)

---

## Explicit Non-Goals

- No simulated time
- No bid execution
- No manual seniority fixes
- No reversible confirmation
- No post-bid structural edits

---

## Completion Conditions

Phase 29 is complete when:

- participation flags enforced
- bid order frozen then adjustable
- capacity math correct
- human judgment supported
- no domain truth corruption
- CI passes

---

## When to Stop and Ask

Stop if:

- flag invariants cannot be enforced
- bid order edits require seniority changes
- readiness becomes bypassable
- correctness requires post-bid exceptions
