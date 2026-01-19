# Phase 27C Findings — Area and Bid Year Identity Audit

**Phase**: 27C — Area and Bid Year Identity Audit
**Date**: 2026-01-XX
**Status**: ✓ COMPLETE — Identity models verified as compliant

---

## Executive Summary

Both **areas** and **bid years** follow the canonical identity model correctly:

- ✓ Canonical numeric IDs (`area_id`, `bid_year_id`) are used for all persistence operations
- ✓ Display values (`area_code`, `year`) are metadata only
- ✓ Foreign keys reference canonical IDs, not display values
- ✓ Lookup functions translate display values to canonical IDs at ingress boundaries only
- ✓ Audit events store canonical IDs for queries (denormalized display values are acceptable)

**No architectural violations found.**

Minor findings documented below for completeness.

---

## Area Identity Model

### Structure (✓ COMPLIANT)

**Domain Model** (`crates/domain/src/types.rs`):

```rust
pub struct Area {
    area_id: Option<i64>,      // Canonical ID (None = not persisted)
    area_code: String,          // Display metadata (normalized to uppercase)
    area_name: Option<String>,  // Optional display metadata
    is_system_area: bool,       // System flag
}
```

- `area_id` is the canonical identifier
- `area_code` is display metadata only
- `area_id()` returns `Option<i64>` for safe access
- `area_code()` returns `&str` for display purposes

**Database Schema** (`crates/persistence/src/diesel_schema.rs`):

```sql
areas (
    area_id BIGINT PRIMARY KEY,    -- Canonical ID
    bid_year_id BIGINT NOT NULL,   -- FK to bid_years
    area_code TEXT NOT NULL,       -- Display metadata
    area_name TEXT,                -- Optional display metadata
    ...
)
```

- Primary key: `area_id` ✓
- Foreign key to bid years: `bid_year_id` ✓
- Display metadata: `area_code`, `area_name` ✓

### Persistence Operations (✓ COMPLIANT)

**Area Creation** (`crates/persistence/src/mutations/bootstrap.rs`):

```rust
diesel::insert_into(areas::table)
    .values((
        areas::bid_year_id.eq(bid_year_id),  // Uses canonical bid_year_id
        areas::area_code.eq(area_code),      // Stores display value
    ))
    .execute(conn)?;

let area_id: i64 = conn.get_last_insert_rowid()?;
```

- Auto-generated `area_id` is assigned ✓
- Foreign key uses `bid_year_id` (canonical), not `year` ✓

**Area Lookups** (`crates/persistence/src/queries/canonical.rs`):

```rust
pub fn lookup_area_id(
    conn: &mut _,
    bid_year_id: i64,        // Requires canonical bid_year_id
    area_code: &str,         // Display value used for lookup only
) -> Result<i64, PersistenceError> {
    areas::table
        .select(areas::area_id)                    // Returns canonical ID
        .filter(areas::bid_year_id.eq(bid_year_id)) // Filters by canonical ID
        .filter(areas::area_code.eq(area_code))    // Filters by display value
        .first::<i64>(conn)
}
```

- Function translates `(bid_year_id, area_code)` → `area_id` ✓
- Used at ingress boundaries (CSV import, API input, audit reconstruction) ✓
- Not used for mutations after initial lookup ✓

### Foreign Key Usage (✓ COMPLIANT)

All tables referencing areas use `area_id`:

- `users.area_id` → `areas.area_id` ✓
- `canonical_area_membership.area_id` → `areas.area_id` ✓
- `state_snapshots.area_id` → `areas.area_id` ✓
- `audit_events.area_id` → `areas.area_id` ✓

No foreign keys reference `area_code`. ✓

### Area Code Normalization (✓ COMPLIANT)

Area codes are normalized to uppercase:

```rust
pub fn new(area_code: &str) -> Self {
    Self {
        area_id: None,
        area_code: area_code.to_uppercase(),  // Normalized
        ...
    }
}
```

- Ensures case-insensitive uniqueness ✓
- Consistent display representation ✓

### Legacy Compatibility Method (⚠️ MINOR)

**Finding**: `Area::id()` method exists for backward compatibility:

```rust
/// Legacy method for backward compatibility - returns `area_code`.
/// This will be removed in Phase 23B when API layer is updated.
pub fn id(&self) -> &str {
    &self.area_code
}
```

**Status**: Documented technical debt
**Impact**: Low — method returns display value, but all persistence uses `area_id()`
**Action**: None required for Phase 27C (addressed in future phase)

---

## Bid Year Identity Model

### Bid Year Structure (✓ COMPLIANT)

**Domain Model** (`crates/domain/src/types.rs`):

```rust
pub struct BidYear {
    bid_year_id: Option<i64>,  // Canonical ID (None = not persisted)
    year: u16,                 // Display metadata
}
```

- `bid_year_id` is the canonical identifier
- `year` is display metadata only
- `bid_year_id()` returns `Option<i64>` for safe access
- `year()` returns `u16` for display purposes

**Database Schema** (`crates/persistence/src/diesel_schema.rs`):

```sql
bid_years (
    bid_year_id BIGINT PRIMARY KEY,  -- Canonical ID
    year INTEGER NOT NULL,           -- Display metadata
    start_date TEXT NOT NULL,
    num_pay_periods INTEGER NOT NULL,
    is_active INTEGER NOT NULL,
    ...
)
```

- Primary key: `bid_year_id` ✓
- Display metadata: `year` ✓

### Bid Year Persistence Operations (✓ COMPLIANT)

**Bid Year Creation** (`crates/persistence/src/mutations/bootstrap.rs`):

```rust
diesel::insert_into(bid_years::table)
    .values((
        bid_years::year.eq(year_i32),            // Stores display value
        bid_years::start_date.eq(&start_date_str),
        bid_years::num_pay_periods.eq(num_pay_periods_i32),
    ))
    .execute(conn)?;

let bid_year_id: i64 = conn.get_last_insert_rowid()?;
```

- Auto-generated `bid_year_id` is assigned ✓
- Display value `year` is stored ✓

**Bid Year Lookups** (`crates/persistence/src/queries/canonical.rs`):

```rust
pub fn lookup_bid_year_id(
    conn: &mut _,
    year: u16,  // Display value used for lookup only
) -> Result<i64, PersistenceError> {
    bid_years::table
        .select(bid_years::bid_year_id)         // Returns canonical ID
        .filter(bid_years::year.eq(year_i32))   // Filters by display value
        .first::<i64>(conn)
}
```

- Function translates `year` → `bid_year_id` ✓
- Used at ingress boundaries (CSV import, API input, audit reconstruction) ✓
- Not used for mutations after initial lookup ✓

### Bid Year Foreign Key Usage (✓ COMPLIANT)

All tables referencing bid years use `bid_year_id`:

- `areas.bid_year_id` → `bid_years.bid_year_id` ✓
- `users.bid_year_id` → `bid_years.bid_year_id` ✓
- `canonical_area_membership.bid_year_id` → `bid_years.bid_year_id` ✓
- `canonical_bid_order.bid_year_id` → `bid_years.bid_year_id` ✓
- `canonical_eligibility.bid_year_id` → `bid_years.bid_year_id` ✓
- `canonical_bid_windows.bid_year_id` → `bid_years.bid_year_id` ✓
- `state_snapshots.bid_year_id` → `bid_years.bid_year_id` ✓
- `audit_events.bid_year_id` → `bid_years.bid_year_id` ✓

No foreign keys reference `year`. ✓

### Active Bid Year Logic (✓ COMPLIANT)

Active bid year flag is stored in `bid_years` table:

```sql
is_active INTEGER NOT NULL DEFAULT 0
```

Queries for active bid year return the full `BidYear` domain object with `bid_year_id`:

```rust
pub fn get_active_bid_year(conn: &mut _) -> Result<BidYear, PersistenceError> {
    let (bid_year_id, year_i32) = bid_years::table
        .select((bid_years::bid_year_id, bid_years::year))
        .filter(bid_years::is_active.eq(1))
        .first::<(i64, i32)>(conn)?;

    let year = year_i32.to_u16()...;
    Ok(BidYear::with_id(bid_year_id, year))  // Returns canonical ID
}
```

- Uses `bid_year_id` for identity ✓
- `year` is metadata only ✓

---

## Audit Event Identity Handling

### Canonical IDs for Queries (✓ COMPLIANT)

**Schema** (`crates/persistence/src/diesel_schema.rs`):

```sql
audit_events (
    event_id BIGINT PRIMARY KEY,
    bid_year_id BIGINT,        -- Canonical ID for queries
    area_id BIGINT,            -- Canonical ID for queries
    year INTEGER,              -- Denormalized display value
    area_code TEXT,            -- Denormalized display value
    ...
)
```

**Audit Event Queries** (`crates/persistence/src/queries/audit.rs`):

All audit queries filter by canonical IDs:

```rust
pub fn get_events_after(
    conn: &mut _,
    bid_year_id: i64,  // Canonical ID
    area_id: i64,      // Canonical ID
    after_event_id: i64,
) -> Result<Vec<AuditEvent>, PersistenceError> {
    audit_events::table
        .filter(audit_events::bid_year_id.eq(bid_year_id))  // Canonical ID
        .filter(audit_events::area_id.eq(area_id))          // Canonical ID
        .filter(audit_events::event_id.gt(after_event_id))
        ...
}
```

- Queries use `bid_year_id` and `area_id` ✓
- Display values are not used for filtering ✓

### Denormalized Display Values (✓ ACCEPTABLE)

**Finding**: Audit events store both canonical IDs and display values.

**Rationale**:

1. **Primary identifiers**: `bid_year_id` and `area_id` are used for all queries
2. **Human readability**: `year` and `area_code` make audit logs self-documenting
3. **No lookup misuse**: Display values are never used as filter criteria
4. **Audit semantics**: Records what the display values WERE at the time of the event

**Pattern**:

```rust
diesel::insert_into(audit_events::table)
    .values((
        audit_events::bid_year_id.eq(bid_year_id),  // Canonical ID (primary)
        audit_events::area_id.eq(area_id),          // Canonical ID (primary)
        audit_events::year.eq(year),                // Display value (denormalized)
        audit_events::area_code.eq(area_code),      // Display value (denormalized)
        ...
    ))
```

**Conclusion**: This is a valid audit pattern. Denormalization for human readability does not violate identity correctness as long as canonical IDs remain authoritative for queries.

---

## CSV Import Pattern (✓ COMPLIANT)

**CSV Structure**:

```csv
initials,name,area_id,user_type,crew,...
ABC,Alice Bob,NORTH,CPC,1,...
```

- `area_id` column contains area codes (display values)
- This is acceptable naming — reflects user-facing vocabulary

**CSV Processing Flow** (`crates/api/src/csv_preview.rs`):

```rust
// Step 1: Parse display value from CSV
let area_id_str: String = parse_required_field(&get_field, "area_id", &mut errors);

// Step 2: Create domain object with display value
let area: Area = Area::new(&area_id_str);

// Step 3: Validate area exists (checks against metadata)
let area_exists: bool = metadata
    .areas
    .iter()
    .any(|(by, a)| by == &user.bid_year && a.id() == user.area.id());
```

**Command Execution** (`crates/core/src/command.rs`):

```rust
Command::RegisterUser {
    initials,
    name,
    area,  // Domain object with display value
    ...
}
```

**Persistence Layer** (`crates/persistence/src/mutations/audit.rs`):

```rust
// Lookup canonical IDs before persisting
let (bid_year_id, area_id) = match (&event.bid_year, &event.area) {
    (Some(bid_year), Some(area)) => {
        let bid_year_id = lookup_bid_year_id(conn, bid_year.year())?;
        let area_id = lookup_area_id(conn, bid_year_id, area.id())?;
        (Some(bid_year_id), Some(area_id))
    }
    ...
};
```

**Pattern**:

1. CSV contains display values ✓
2. Domain objects constructed with display values ✓
3. Lookup functions resolve display → canonical IDs at persistence boundary ✓
4. All persistence uses canonical IDs ✓

This is the correct ingress translation pattern.

---

## Comparison with User Identity Pattern

### Consistency Verification

| Aspect                | Users                                            | Areas                                    | Bid Years                           |
| --------------------- | ------------------------------------------------ | ---------------------------------------- | ----------------------------------- |
| Canonical ID field    | `user_id: Option<i64>`                           | `area_id: Option<i64>`                   | `bid_year_id: Option<i64>`          |
| Display field         | `initials: Initials`                             | `area_code: String`                      | `year: u16`                         |
| Primary key           | `user_id`                                        | `area_id`                                | `bid_year_id`                       |
| Display normalization | Uppercase                                        | Uppercase                                | N/A                                 |
| Foreign key usage     | `user_id`                                        | `area_id`                                | `bid_year_id`                       |
| Lookup function       | `lookup_user_id(bid_year_id, area_id, initials)` | `lookup_area_id(bid_year_id, area_code)` | `lookup_bid_year_id(year)`          |
| Audit storage         | Canonical ID + denormalized display              | Canonical ID + denormalized display      | Canonical ID + denormalized display |

✓ All three domain entities follow the same identity pattern.

---

## Regression Test Coverage

### Existing Test Coverage

The following behaviors are already tested in the existing test suite:

1. **Bid year creation** (`crates/persistence/src/tests/bootstrap_tests/mod.rs`)
   - Verifies `bid_year_id` is assigned after creation
   - Verifies audit events are created with canonical IDs

2. **Area creation** (`crates/persistence/src/tests/bootstrap_tests/mod.rs`)
   - Verifies `area_id` is assigned after creation
   - Verifies foreign key to `bid_year_id`

3. **User creation** (`crates/persistence/src/tests/bootstrap_tests/mod.rs`)
   - Verifies users reference `area_id` and `bid_year_id`

4. **Audit event queries** (`crates/persistence/src/tests/state_tests/`)
   - Verifies audit timeline retrieval uses canonical IDs

5. **CSV import** (`crates/api/src/csv_preview.rs` tests)
   - Verifies area validation using display values
   - Verifies error messages reference display values

### Additional Regression Tests Not Required

Based on the audit findings:

- Identity model is already correct
- Existing tests verify canonical ID usage
- No violations to prevent via new tests

**Conclusion**: No new regression tests are needed for Phase 27C.

---

## Recommendations

### 1. Area::id() Deprecation (Future Phase)

**Current State**: `Area::id()` method returns `area_code` for backward compatibility.

**Recommendation**: Remove this method when API layer is fully updated (Phase 23B).

**Risk**: Low — method is clearly marked as legacy and does not affect persistence.

### 2. Documentation Update (Optional)

Consider adding architectural documentation explaining:

- The distinction between canonical IDs and display values
- Why lookup functions exist at ingress boundaries
- Why audit events denormalize display values

This would help future maintainers understand the identity model.

---

## Conclusion

**Phase 27C Verdict**: ✓ **COMPLETE — NO VIOLATIONS FOUND**

Both areas and bid years follow the canonical identity model correctly:

- Canonical numeric IDs are used for all persistence operations
- Display values are metadata only
- Lookup functions correctly translate at ingress boundaries
- Foreign keys reference canonical IDs
- Audit events use canonical IDs for queries

The architecture is consistent with the user identity model established in Phase 27B.

**No redesign or refactoring required.**

**No Phase 28 work required for area or bid year identity.**

---

## Files Audited

### Domain Layer

- `crates/domain/src/bid_year.rs` — `CanonicalBidYear` structure

### Persistence Layer

- `crates/persistence/src/diesel_schema.rs` — Database schema
- `crates/persistence/src/mutations/bootstrap.rs` — Bid year and area creation
- `crates/persistence/src/mutations/canonical.rs` — Area mutations
- `crates/persistence/src/mutations/audit.rs` — Audit event persistence
- `crates/persistence/src/queries/canonical.rs` — Lookup functions

- `crates/persistence/src/queries/audit.rs` — Audit queries

### Core Layer

- `crates/core/src/command.rs` — Command definitions
- `crates/core/src/apply.rs` — Command execution

### API Layer

- `crates/api/src/csv_preview.rs` — CSV import handling

### Tests

- `crates/persistence/src/tests/bootstrap_tests/mod.rs`
- `crates/api/src/csv_preview.rs` (inline tests)

---

**Phase 27C Status**: ✓ COMPLETE
**Next Phase**: Phase 27D or proceed to Phase 28 as needed
