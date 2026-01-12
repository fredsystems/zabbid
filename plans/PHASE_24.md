# Phase 24

## Phase 24 — Domain State Machine: Bid Year Lifecycle

### Goal

Make bid year phases explicit, enforceable, and auditable.

---

### New Concepts

- `BidYearStatus` enum:
  - `Bootstrapping`
  - `Active`
  - `Archived`

---

### Rules

- Only one bid year may be `Active`
- Only `Bootstrapping` bid years allow:
  - area creation
  - user creation
  - metadata edits
- Transition `Bootstrapping → Active` requires:
  - bootstrap completeness = true
- Transition emits an audit event

---

### Explicit Locks

- When `Active`:
  - Editing locked except:
    - user name
    - user initials
- When `Archived`:
  - No edits allowed

---

### Exit Criteria

- Bid year status enforced in domain/core
- UI unchanged
- Clear audit events for transitions
