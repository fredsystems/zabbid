# Phase 23

## Phase 23 — Canonical Identity Completion (Area & Bid Year)

### Goal

Finish the identity model so all internal references use stable numeric IDs, while preserving human-facing identifiers for display.

---

### Area Identity

- Introduce `area_id` (numeric, canonical)
- Preserve `area_code` / `area_name` as display fields
- All domain logic and persistence must use `area_id`
- API responses include both:
  - `area_id`
  - `area_code` (display)

---

### Bid Year Identity

- Introduce `bid_year_id` (numeric, canonical)
- Preserve `year` as a display value
- All references move to `bid_year_id`
- Active bid year tracked by `bid_year_id`, not year

---

### Constraints

- No UI changes
- No workflow changes
- No CSV changes
- Tests updated in:
  - persistence
  - domain
  - API
  - server

---

### Exit Criteria

- No mutation uses area code or year as identity
- All read APIs expose IDs plus display fields
- All tests pass
- No UI touched
