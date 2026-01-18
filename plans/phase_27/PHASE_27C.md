# Phase 27C — Area and Bid Year Identity Audit

## Purpose

Apply identity correctness patterns from Phase 27B to areas and bid years, ensuring canonical IDs are used for all operations and display values are metadata only.

## Scope

### Analysis Tasks

#### Areas

- Determine if areas use canonical `area_id` vs display `area_code` pattern
- Review `crates/domain/src/area.rs` structure and identity semantics
- Review `crates/persistence/src/canonical/areas.rs` implementation
- Grep for `area.id()` in mutation and lookup contexts
- Grep for `area_code` usage in foreign keys and queries
- Audit CSV preview logic (`crates/api/src/csv_preview.rs`) for area references
- Check if area codes can change without breaking references

#### Bid Years

- Determine if bid years use canonical `bid_year_id` vs display `year` pattern
- Review `crates/domain/src/bid_year.rs` structure and identity semantics
- Review `crates/persistence/src/canonical/bid_years.rs` implementation
- Grep for `bid_year.year()` in mutation and lookup contexts
- Check if year values are used as foreign keys
- Verify active bid year logic uses canonical ID

### Verification Targets

- Area lookups use canonical ID not display string
- Bid year lookups use canonical ID not year value
- CSV validation uses canonical references appropriately
- Foreign keys reference canonical IDs not display values
- Audit events record canonical IDs as identifiers

### Test Additions

- Tests verifying area code can change without breaking references (if applicable)
- Tests verifying year value is not used as foreign key (if applicable)
- Tests confirming canonical ID usage in state transitions
- Regression tests preventing future display-value-as-ID misuse

## Explicit Non-Goals

- Do NOT change existing identity models if already correct
- Do NOT introduce new canonical tables if not needed
- Do NOT modify CSV import format or file structure
- Do NOT refactor domain models for style
- Do NOT add features unrelated to identity verification

## Files Likely to Be Affected

### Backend - Areas

- `crates/domain/src/area.rs`
- `crates/persistence/src/canonical/areas.rs`
- `crates/persistence/src/queries/areas.rs`
- `crates/api/src/handlers/areas.rs`
- `crates/api/src/csv_preview.rs`
- `crates/core/src/commands/` (area-related commands)

### Backend - Bid Years

- `crates/domain/src/bid_year.rs`
- `crates/persistence/src/canonical/bid_years.rs`
- `crates/persistence/src/queries/bid_years.rs`
- `crates/api/src/handlers/bid_years.rs`
- `crates/core/src/commands/` (bid year-related commands)

### Frontend

- `ui/src/components/admin/areas/`
- `ui/src/components/admin/bid_years/`
- Any forms submitting area or bid year data

### Tests

- `crates/api/tests/` (area and bid year endpoint tests)
- `crates/core/tests/` (state transition tests)
- `crates/persistence/tests/` (query tests)

## Search Patterns

Execute the following searches to identify potential violations:

```bash
# Find area code usage in persistence
grep -rn "area_code\|area\.id()" crates/persistence/src/ --include="*.rs"

# Find area lookups and mutations
grep -rn "find.*area\|get.*area\|lookup.*area" crates/persistence/src/ --include="*.rs"

# Find bid year value usage
grep -rn "\.year()\|bid_year\.year" crates/persistence/src/ --include="*.rs"

# Find bid year lookups
grep -rn "find.*bid_year\|get.*bid_year" crates/persistence/src/ --include="*.rs"

# Check CSV preview area handling
grep -rn "area_id\|area_code" crates/api/src/csv_preview.rs
```

## Critical Questions to Answer

1. Does the `Area` domain type have separate canonical ID and display code?
2. Does the `BidYear` domain type have separate canonical ID and year value?
3. Are foreign keys in the database schema using canonical IDs?
4. Can area codes be changed without breaking user or audit references?
5. Can bid year values be changed without breaking references?

## Completion Conditions

- Area identity pattern documented and verified correct
- Bid year identity pattern documented and verified correct
- Any identity misuse corrected or flagged for user decision
- Tests confirm identity correctness for both areas and bid years
- All existing tests still pass
- Git commit focused on area/bid year identity only

## Dependencies

- Phase 27A must be complete (requires clear identity rules for areas and bid years)

## Blocks

None (parallel work allowed with 27B after 27A)

## Execution Notes

### If Identity Models Are Already Correct

If areas and bid years already use canonical IDs correctly:

- Document this finding
- Add regression tests to prevent future violations
- Update any misleading comments or documentation
- Commit verification tests only

### If Identity Violations Are Found

If areas or bid years use display values as identifiers:

- Document specific violations found
- Assess scope of required changes
- If fixes are straightforward (similar to user pattern), implement them
- If fixes require architectural changes, stop and report findings

### CSV Import Considerations

CSV import likely uses string-based area codes as input. This is acceptable if:

- Codes are immediately resolved to canonical IDs
- Canonical IDs are used for all persistence and validation
- Display codes are not stored as foreign keys

Verify this pattern is followed correctly.

### Comparison with User Identity Pattern

Use Phase 27B findings as template:

- Users have `user_id` (canonical) and `initials` (display)
- Areas should have equivalent pattern (if not already)
- Bid years should have equivalent pattern (if not already)

Consistency across domain entities improves maintainability.
