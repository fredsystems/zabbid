# Phase 26

## Phase 26 — Editing Semantics & Late Changes

### Goal

Handle real-world late changes without breaking invariants.

---

### Editing Rules

- After bootstrap:
  - Adding users allowed
  - Removing users restricted
- If actual user count exceeds expected:
  - Expected count auto-increments
  - Audit event emitted

---

### Deletion Rules

- User deletion allowed only if:
  - No bids
  - No leave usage
  - Only metadata edits exist
- Otherwise:
  - User may be disabled only

---

### Exit Criteria

- Editing rules enforced in domain
- UI reflects allowed actions
- No silent state changes
