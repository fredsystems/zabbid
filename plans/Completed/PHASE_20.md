# Phase 20 — UI Alignment with Active Bid Year Invariant

## Phase Objective

Phase 20 exists to **realign the frontend UI with the backend’s Active Bid Year invariant**, ensuring that:

- The UI no longer encodes or assumes bid-year context for mutating actions
- All mutations clearly and exclusively apply to the backend-defined _active_ bid year
- The UI cannot place the system into an invalid or misleading state
- Domain invariants enforced in Phase 19 are reflected correctly in UX and routing

This phase is **UI-only**.
No domain, core, persistence, or API behavior changes are permitted.

---

## Non-Goals (Explicitly Out of Scope)

Phase 20 does **not** include:

- UI polish, theming, contrast fixes, or accessibility work
- User identifier refactors (numeric IDs, initials changes)
- CSV import enhancements
- Operator credential work
- New backend endpoints
- API ergonomics changes
- Public-facing UI expansion

Those are deferred to later phases.

---

## Core Invariant (Authoritative)

> **All bootstrap and mutating actions apply only to the single active bid year, as determined by the backend.**

The UI must treat this as **non-negotiable and non-configurable**.

---

## Required UI Changes

### 1. Remove Bid-Year-Scoped Routing for Mutations

The UI must **eliminate routes that imply mutation within an arbitrary bid year**, including (but not limited to):

/bid-year/:year/areas
/bid-year/:year/users
/bid-year/:year/bootstrap

yaml
Copy code

These routes must **not** be used for:

- Creating areas
- Registering users
- Editing users
- Setting expected counts
- CSV preview or import

Any remaining bid-year-specific routes must be **read-only** or removed.

---

### 2. Active Bid Year as Backend-Resolved Context

The UI must:

- Fetch the active bid year via:
  GET /api/bootstrap/bid-years/active

yaml
Copy code

- Display the active bid year **clearly and prominently**
- Treat it as **read-only context**, not user-selectable

The UI must not allow selecting or mutating non-active bid years.

---

### 3. Remove Bid Year Inputs from All Mutation Flows

All UI forms and API calls for the following actions must **not** include `bid_year`:

- Create Area
- Register User
- Update User
- Set Expected Area Count
- Set Expected User Count
- CSV Preview
- CSV Import

The UI must rely entirely on backend resolution of the active bid year.

---

### 4. Navigation Restructuring

Admin navigation must be refactored to reflect system state, not bid-year scope.

Required structure (example):

- **Admin Dashboard**
- Active Bid Year (display only)
- Bootstrap Status
- Areas
- Users
- Operators
- CSV Import

Navigation must **never** imply working in multiple bid years simultaneously.

---

### 5. Error Handling & Guard Rails

If the backend returns:

- `NoActiveBidYear`
- `MultipleBidYearsActive`

The UI must:

- Block all mutating actions
- Display a clear, actionable message explaining the issue
- Guide the admin to resolve the problem (e.g., “Set an active bid year”)

No silent failures. No partial rendering.

---

### 6. Read-Only Bid Year Views (Optional, Safe)

If bid-year-specific views remain (e.g., historical inspection):

- They must be explicitly **read-only**
- No mutation controls may appear
- Visual distinction between “Active” and “Historical” must be clear

---

## UX Requirements

- No dropdowns or selectors for bid year in mutation flows
- No hidden assumptions about bid year context
- Clear labeling: “Applies to Active Bid Year: XXXX”
- Mobile-safe layouts maintained
- No tables introduced where cards are currently used

---

## Testing Requirements

- Manual verification of all mutation flows
- Ensure no API call includes `bid_year`
- Verify UI behavior when:
- No active bid year exists
- Backend returns invariant errors
- Regression check: existing bootstrap flows still function

No automated frontend tests are required in this phase, but no existing behavior may regress.

---

## Exit Criteria

Phase 20 is complete when all of the following are true:

- UI no longer encodes bid-year context for mutations
- Active bid year is backend-resolved and read-only in UI
- All mutation flows operate correctly against active bid year
- UI blocks mutations when no active bid year exists
- Navigation reflects system state, not bid-year scope
- No domain invariants are enforced only at the UI layer
- No backend or API changes were required
- Manual testing confirms correct behavior

---

## Phase Summary

Phase 20 ensures the UI is **honest, invariant-aligned, and structurally correct**.

After this phase:

- The backend owns bid year authority
- The UI reflects, not invents, system state
- Future work (IDs, ergonomics, polish) can proceed safely without structural risk
