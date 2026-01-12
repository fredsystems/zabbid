# Phase 25

## Phase 25 — “No Bid” Area Introduction

### Goal

Introduce a canonical safety area to formalize edge cases.

---

### Behavior

- System creates a reserved `No Bid` area per bid year
- Properties:
  - `is_no_bid = true`
- Not visible to unauthenticated users
- Cannot be deleted
- Cannot have rounds or limits

---

### Automatic Usage

- Users with no area → auto-assigned to No Bid
- Deleting an area → users moved to No Bid
- CSV import with missing area → No Bid

---

### Bootstrap Rules

- Bootstrap incomplete if:
  - Any user exists in No Bid
  - AND user not explicitly reviewed

---

### UI Flags

- Per-user “reviewed” toggle (admin only)

---

### Exit Criteria

- No Bid area enforced at domain level
- Bootstrap completeness depends on No Bid review
- No bidding logic introduced
