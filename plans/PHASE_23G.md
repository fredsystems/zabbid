# Phase 23G — Canonical API Input Migration

## Goal

Complete the canonical identity transition by making the **API boundary fully canonical**.

After this phase:

- All API **inputs** use numeric IDs
- All API **outputs** already use numeric IDs (completed in 23C–23E)
- The API contract is internally consistent and stable
- The UI can be safely updated without guesswork

This phase intentionally introduces **breaking API changes**.
The UI will be updated in the following phase (23F).

---

## Scope

IN SCOPE:

- API layer
- Server layer
- API tests
- Server tests (if applicable)

OUT OF SCOPE:

- UI changes
- Domain changes
- Persistence changes
- CSV formats
- ORMs or migrations

---

## Identity Rules (Authoritative After This Phase)

- `bid_year_id: i64` is the sole identifier for bid years
- `area_id: i64` is the sole identifier for areas
- `user_id: i64` is the sole identifier for users
- Display values (`year`, `area_code`, `initials`) are **never used for identity**
- Display values remain read-only metadata

---

## API Input Changes

### Route Parameters

Replace all display-based identifiers with canonical IDs.

#### Examples

| Old Route                          | New Route                       |
| ---------------------------------- | ------------------------------- |
| `/bid-years/:year`                 | `/bid-years/:bid_year_id`       |
| `/bid-year/:year/areas`            | `/bid-years/:bid_year_id/areas` |
| `/bid-year/:year/area/:area/users` | `/areas/:area_id/users`         |
| `/users/:initials`                 | `/users/:user_id`               |

---

### Query Parameters

Replace display identifiers with canonical IDs.

| Old Param       | New Param        |
| --------------- | ---------------- |
| `bid_year=2026` | `bid_year_id=12` |
| `area=DEN`      | `area_id=4`      |

No endpoint may accept both forms.

---

### Request Bodies

All mutation requests must reference canonical IDs only.

Examples:

- `set_active_bid_year { bid_year_id }`
- `update_user { user_id, area_id, ... }`
- `set_expected_user_count { area_id, count }`

---

## Server Layer Responsibilities

- Parse canonical IDs from routes and query parameters
- Validate existence of referenced entities
- Reject legacy identifiers with clear errors
- Do NOT perform any ID lookups based on display fields

---

## API Layer Responsibilities

- Handler signatures must accept canonical IDs directly
- No internal conversion from display values
- Errors must surface missing or invalid IDs explicitly
- No fallback or compatibility logic

---

## Tests

### Required Updates

- All API tests must:
  - Bootstrap canonical entities via persistence
  - Extract IDs from responses
  - Use IDs in all subsequent requests
- Tests must NOT fabricate IDs
- Tests must fail if canonical state is missing

### Explicitly Forbidden

- Sentinel IDs
- Magic numbers
- Auto-creation of canonical entities
- Helper functions that hide persistence

---

## Error Handling

- Missing IDs → `NotFound`
- Invalid IDs → `ValidationError`
- Legacy identifiers → `InvalidRequest`

Errors must be:

- Deterministic
- Structured
- Testable

---

## Exit Criteria

- No API endpoint accepts display identifiers
- All identity-bearing inputs use numeric IDs
- All API tests pass using canonical IDs
- No UI code modified
- No domain or persistence changes
- No compatibility shims added

---

## Notes

This phase intentionally breaks the existing UI.
That is expected and correct.

UI updates will be handled in **Phase 23F** once the API contract is final.
