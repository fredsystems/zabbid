# Phase 28

## Phase 28 — Rounds & Group Rule Foundations (No Bidding Yet)

### Goal

Define the structural foundation for bidding without implementing bidding logic.

---

### New Entities

- `RoundGroup`
  - `group_id` (canonical)
  - `display_name`
- `Round`
  - `round_id`
  - `order`
  - `group_id`
  - `available_slots`
  - `editing_enabled`

---

### Rules

- Areas (except No Bid) must have a RoundGroup assigned
- Bootstrap incomplete until:
  - All applicable areas assigned
- No Bid explicitly excluded

---

### Holidays

- Holiday override structure:
  - date
  - slot adjustment
  - enabled flag
- Holidays treated uniformly (no per-holiday semantics)

---

### Exit Criteria

- All structures persisted
- No bidding logic implemented
- UI supports configuration only
