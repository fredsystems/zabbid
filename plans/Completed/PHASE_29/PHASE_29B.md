# Phase 29B — Round Groups and Rounds

## Purpose

Implement round configuration infrastructure required for bidding logic.

Round groups define reusable rule sets. Rounds apply those rules to specific bidding periods within areas.

This sub-phase creates the **structure only**. Execution logic is out of scope.

---

## Scope

### 1. Database Schema

#### Round Groups Table

```sql
CREATE TABLE round_groups (
    round_group_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    bid_year_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    editing_enabled INTEGER NOT NULL DEFAULT 1 CHECK(editing_enabled IN (0, 1)),
    UNIQUE (bid_year_id, name),
    FOREIGN KEY(bid_year_id) REFERENCES bid_years(bid_year_id)
);
```

#### Rounds Table

```sql
CREATE TABLE rounds (
    round_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    area_id INTEGER NOT NULL,
    round_group_id INTEGER NOT NULL,
    round_number INTEGER NOT NULL,
    name TEXT NOT NULL,
    slots_per_day INTEGER NOT NULL CHECK(slots_per_day > 0),
    max_groups INTEGER NOT NULL CHECK(max_groups > 0),
    max_total_hours INTEGER NOT NULL CHECK(max_total_hours > 0),
    include_holidays INTEGER NOT NULL DEFAULT 0 CHECK(include_holidays IN (0, 1)),
    allow_overbid INTEGER NOT NULL DEFAULT 0 CHECK(allow_overbid IN (0, 1)),
    UNIQUE (area_id, round_number),
    FOREIGN KEY(area_id) REFERENCES areas(area_id),
    FOREIGN KEY(round_group_id) REFERENCES round_groups(round_group_id)
);
```

### 2. Domain Types

Add to `domain/src/types.rs`:

```rust
pub struct RoundGroup {
    round_group_id: Option<i64>,
    bid_year: BidYear,
    name: String,
    editing_enabled: bool,
}

pub struct Round {
    round_id: Option<i64>,
    area: Area,
    round_group: RoundGroup,
    round_number: u32,
    name: String,
    slots_per_day: u32,
    max_groups: u32,
    max_total_hours: u32,
    include_holidays: bool,
    allow_overbid: bool,
}
```

### 3. Round Semantics (Documentation Only)

#### Group Formation Rules

- Groups are up to 5 consecutive days
- RDOs (Regular Days Off) are excluded from group length
- Skipped days split groups

**Example:** If a user bids Mon-Wed, skips Thu, bids Fri, that's 2 groups (Mon-Wed = 3 days, Fri = 1 day).

#### Overbid Policy

Each round must declare one of:

1. **No Overbid Allowed** (`allow_overbid = false`)
   - Accrued leave limits apply
   - Formula: `min(round.max_groups / round.max_hours, remaining_accrued_leave)`

2. **Overbid Allowed** (`allow_overbid = true`)
   - Accrued leave limits ignored
   - Round limits still apply
   - Typically used for carryover rounds

**This sub-phase only stores the configuration. No calculation logic is implemented.**

### 4. System Area Constraint

- System areas (e.g., No Bid) have **no rounds**
- Attempts to create rounds for system areas must fail
- Readiness checks must ignore system areas when validating round coverage

### 5. API Endpoints

#### Round Groups

- `POST /api/bid-years/{bid_year_id}/round-groups`
  - Create new round group
  - Request: `{ name: string, editing_enabled: bool }`
- `GET /api/bid-years/{bid_year_id}/round-groups`
  - List all round groups for bid year
- `PATCH /api/round-groups/{round_group_id}`
  - Update round group (name, editing_enabled)
- `DELETE /api/round-groups/{round_group_id}`
  - Delete round group (only if no rounds reference it)

#### Rounds

- `POST /api/areas/{area_id}/rounds`
  - Create new round
  - Request: `{ round_group_id, round_number, name, slots_per_day, max_groups, max_total_hours, include_holidays, allow_overbid }`
  - Validates area is not a system area
- `GET /api/areas/{area_id}/rounds`
  - List all rounds for area
- `PATCH /api/rounds/{round_id}`
  - Update round configuration
- `DELETE /api/rounds/{round_id}`
  - Delete round

### 6. Lifecycle Constraints

- Round groups and rounds are editable in `Draft` and `BootstrapComplete` states
- After confirmation/canonicalization, round configuration becomes immutable (or requires explicit override)

### 7. Readiness Contribution

This sub-phase **does not implement** readiness evaluation.

However, it establishes the constraint that readiness will later require:

- All non-system areas must have at least one round
- All rounds must reference a valid round group

---

## Explicit Non-Goals

- No bid execution logic
- No leave calculation
- No group formation implementation
- No overbid calculation
- No readiness evaluation (that's Sub-Phase 29D)
- No UI for round management (out of scope for Phase 29)

---

## Completion Checklist

- [ ] Migrations created for both SQLite and MySQL
- [ ] Schema verification passes (`cargo xtask verify-migrations`)
- [ ] Domain types created
- [ ] Persistence layer supports CRUD operations
- [ ] API endpoints implemented
- [ ] System area constraint enforced
- [ ] Lifecycle constraints enforced
- [ ] Unit tests for domain types
- [ ] Integration tests for API endpoints
- [ ] Constraint tests (system area rejection, unique round numbers)
- [ ] `cargo xtask ci` passes
- [ ] `pre-commit run --all-files` passes

---

## Stop-and-Ask Conditions

Stop if:

- Round semantics conflict with existing domain rules
- System area constraint enforcement is unclear
- Lifecycle constraints are ambiguous
- Round group reuse patterns need clarification
- Overbid policy semantics require additional rules

---

## Risk Notes

- Round groups are reusable across rounds, but deletion may be blocked if referenced
- Round numbers must be unique per area
- Existing areas will have no rounds until explicitly configured
- Readiness evaluation (later sub-phase) will fail until all non-system areas have rounds
