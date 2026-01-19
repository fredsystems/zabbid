# Phase 28C — Fix No-Bid Area Exclusion in Completeness Logic

## Purpose

Correct the domain bug where system-designated areas (identified by `is_system_area = true`, including No Bid) are incorrectly counted toward expected area totals in completeness and readiness calculations.

This sub-phase enforces the domain rule:

> **System-designated areas (identified by `is_system_area = true`) must NOT count toward expected area totals**

---

## Scope

### Files to Modify

#### Persistence Layer — Query Functions

- `crates/persistence/src/queries/canonical.rs`
  - **Update:** `get_actual_area_count()` (lines ~457-466)
  - Add filter: `.filter(areas::is_system_area.eq(false))`
  - Exclude system-designated areas from count (flag-based, not name-based)

**Current Implementation (Incorrect):**

```rust
pub fn get_actual_area_count(conn: &mut _, bid_year_id: i64) -> Result<usize, PersistenceError> {
    let count: i64 = areas::table
        .filter(areas::bid_year_id.eq(bid_year_id))
        .count()  // ❌ Counts ALL areas including system-designated areas
        .get_result(conn)?;

    count
        .to_usize()
        .ok_or_else(|| PersistenceError::DatabaseError("Count conversion failed".to_string()))
}

```

**Correct Implementation:**

```rust
pub fn get_actual_area_count(conn: &mut _, bid_year_id: i64) -> Result<usize, PersistenceError> {
    let count: i64 = areas::table
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::is_system_area.eq(false))  // ✅ Exclude system-designated areas by flag
        .count()
        .get_result(conn)?;

    count
        .to_usize()
        .ok_or_else(|| PersistenceError::DatabaseError("Count conversion failed".to_string()))
}

```

---

#### Legacy SQLite Query (If Still Used)

- `crates/persistence/src/sqlite/bootstrap.rs`
  - **Check:** `get_actual_area_count()` (lines ~474-482)
  - If this function is still in use, update SQL query:
    - **Before:** `SELECT COUNT(*) FROM areas WHERE bid_year = ?1`
    - **After:** `SELECT COUNT(*) FROM areas WHERE bid_year = ?1 AND is_system_area = 0`

---

#### Completeness Module (Verify No Changes Needed)

- `crates/persistence/src/queries/completeness.rs`
  - **Review:** `count_areas_by_bid_year()` (lines ~75-100)
  - Verify this query does NOT need system area exclusion
  - This query aggregates across all bid years; filtering may or may not be needed
  - If it's purely informational (not used for completeness checks), leave unchanged

---

### Files to Add/Update — Tests

- `crates/persistence/src/tests/canonical_tests/*.rs`
  - Add test: `test_actual_area_count_excludes_system_areas`
  - Verify system-designated areas (identified by flag) are excluded from count

- `crates/api/src/tests/api_tests.rs`
  - Add test: `test_completeness_with_system_area_does_not_block`
  - Verify system-designated areas don't prevent readiness

---

## Invariants Being Enforced

1. **System Area Exclusion:** System-designated areas (identified by `is_system_area = true`) MUST NOT count toward `expected_area_count` comparisons
2. **Flag-Based Identification:** Logic MUST identify system areas by `is_system_area` flag, never by area code string matching or area names
3. **Order Independence:** Exclusion logic must not depend on area creation order or IDs

---

## Explicit Non-Goals

- **Do NOT** change how system-designated areas are created or marked
- **Do NOT** alter expected area count setting logic
- **Do NOT** modify user count queries (only area count queries)
- **Do NOT** change Command definitions (already handled in 28B)
- **Do NOT** remove system area enforcement rules (e.g., cannot assign users to system-designated areas after canonicalization)
- **Do NOT** use area code or area name string matching to identify system areas

---

## Implementation Strategy

### Step 1: Identify All Area Count Queries

**Known Query Functions:**

1. `queries/canonical.rs::get_actual_area_count()` — **MUST update**
2. `sqlite/bootstrap.rs::get_actual_area_count()` — **Check if still used**
3. `queries/completeness.rs::count_areas_by_bid_year()` — **Review for necessity**

**Verification Command:**

```bash
grep -rn "COUNT.*areas" crates/persistence/src/
grep -rn "get_actual_area_count" crates/

```

---

### Step 2: Update Diesel Query (Primary)

**File:** `crates/persistence/src/queries/canonical.rs`

**Change:**

```rust
backend_fn! {
pub fn get_actual_area_count(conn: &mut _, bid_year_id: i64) -> Result<usize, PersistenceError> {
    let count: i64 = areas::table
        .filter(areas::bid_year_id.eq(bid_year_id))
        .filter(areas::is_system_area.eq(false))  // ← ADD THIS LINE
        .count()
        .get_result(conn)?;

    count
        .to_usize()
        .ok_or_else(|| PersistenceError::DatabaseError("Count conversion failed".to_string()))
}
}

```

**Backend Coverage:**

- This change applies to both SQLite and MySQL via `backend_fn!` macro
- No backend-specific divergence required

---

### Step 3: Update Legacy SQLite Query (If Present)

**File:** `crates/persistence/src/sqlite/bootstrap.rs`

**Current (If Exists):**

```rust
pub fn get_actual_area_count(conn: &Connection, year: u16) -> Result<usize, PersistenceError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM areas WHERE bid_year = ?1",
        params![year],
        |row| row.get(0),
    )?;
    Ok(usize::try_from(count).expect("count out of usize range"))
}

```

**Updated:**

```rust
pub fn get_actual_area_count(conn: &Connection, year: u16) -> Result<usize, PersistenceError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM areas WHERE bid_year = ?1 AND is_system_area = 0",
        params![year],
        |row| row.get(0),
    )?;
    Ok(usize::try_from(count).expect("count out of usize range"))
}

```

**Note:** Verify if this function is still called or deprecated in favor of Diesel queries.

---

### Step 4: Review Completeness Aggregation Query

**File:** `crates/persistence/src/queries/completeness.rs`

**Function:** `count_areas_by_bid_year()`

**Decision Criteria:**

- If this query is used for **informational display only** → No change needed
- If this query is used for **completeness validation** → Add system area filter

**Most Likely:** No change needed (this is an aggregation query for display, not validation)

---

### Step 5: Add Regression Tests

#### Test 1: Persistence Layer

File: `crates/persistence/src/tests/canonical_tests/area_counting.rs` (or similar)

```rust
#[test]
fn test_actual_area_count_excludes_system_areas() {
    let mut persistence = setup_test_persistence();
    let bid_year_id = create_test_bid_year(&mut persistence, 2026);

    // Create regular areas
    create_area(&mut persistence, bid_year_id, "North", false);
    create_area(&mut persistence, bid_year_id, "South", false);

    // Create system area (No Bid)
    create_area(&mut persistence, bid_year_id, "NO BID", true);

    let count = persistence.get_actual_area_count(&BidYear::new(2026)).unwrap();

    assert_eq!(count, 2, "System area must not be counted");
}

```

#### Test 2: Completeness Logic

File: `crates/api/src/tests/api_tests.rs`

```rust
#[test]
fn test_completeness_with_no_bid_area_does_not_block() {
    let mut persistence = setup_test_persistence();

    // Set expected area count to 2
    set_expected_area_count(&mut persistence, 2026, 2);

    // Create 2 regular areas + 1 system area
    create_area(&mut persistence, 2026, "North", false);
    create_area(&mut persistence, 2026, "South", false);
    create_area(&mut persistence, 2026, "NO BID", true);

    let completeness = get_bootstrap_completeness(&mut persistence, &metadata).unwrap();

    // Should be complete: 2 expected, 2 actual (No Bid excluded)
    assert!(completeness.bid_years[0].is_complete);
    assert!(completeness.bid_years[0].blocking_reasons.is_empty());
}

```

---

## Completion Conditions

Phase 28C is complete when:

- ✅ `get_actual_area_count()` filters `is_system_area = false` (flag-based)
- ✅ Legacy SQLite query (if exists) updated or removed
- ✅ `count_areas_by_bid_year()` reviewed and updated if needed
- ✅ Test added: system-designated areas excluded from actual count
- ✅ Test added: system-designated areas don't block completeness
- ✅ Expected area count of 2 matches actual count of 2 when system-designated area exists
- ✅ All tests pass
- ✅ `cargo xtask ci` passes
- ✅ `pre-commit run --all-files` passes

---

## Expected Risks & Edge Cases

### Risk: Multiple System Areas

**Likelihood:** Low (currently only No Bid exists)

**Mitigation:**

- Filter by `is_system_area` flag exclusively, never by area code or name
- Logic is safe for any number of system-designated areas

---

### Risk: Completeness Aggregation Query Divergence

**Likelihood:** Low

**Concern:** If `count_areas_by_bid_year()` is used for validation, it may need the same filter.

**Mitigation:**

- Review query usage in completeness logic
- If uncertain, add the filter conservatively
- Test both display and validation paths

---

### Risk: Breaking Existing Completeness Tests

**Likelihood:** Medium

**Concern:** Existing tests may expect system-designated areas to be counted.

**Mitigation:**

- Update test fixtures to account for flag-based exclusion
- Adjust expected counts in assertions
- Verify tests reflect correct domain behavior, not incorrect prior implementation

---

## Stop-and-Ask Conditions

If any of the following occur, stop and request guidance:

- System areas are identified by area code or name string matching instead of `is_system_area` flag
- The `is_system_area` column is missing from schema
- Multiple area count queries exist with conflicting semantics
- Completeness logic requires system-designated areas to be counted for correctness
- Backend-specific query divergence is required (SQLite vs MySQL)
- Tests reveal that system area exclusion breaks lifecycle transitions

---

## Validation Checklist

Before marking Phase 28C complete:

- [ ] `get_actual_area_count()` includes `.filter(areas::is_system_area.eq(false))`
- [ ] Legacy SQLite query updated or confirmed unused
- [ ] `count_areas_by_bid_year()` reviewed and updated if needed
- [ ] Test added: `test_actual_area_count_excludes_system_areas`
- [ ] Test added: `test_completeness_with_system_area_does_not_block`
- [ ] Existing completeness tests updated to account for flag-based exclusion
- [ ] No area code or name string matching used for system area detection
- [ ] System areas identified exclusively by `is_system_area` flag
- [ ] CI passes: `cargo xtask ci`
- [ ] Linting passes: `pre-commit run --all-files`
- [ ] No new clippy warnings introduced

---

## References

- **Authoritative Spec:** `plans/PHASE_28.md`
- **Domain Rules:** `AGENTS.md` § Domain Invariants (Areas section)
- **System Area Creation:** `crates/domain/src/types.rs` (`Area::new_system_area()`)
- **Completeness Logic:** `crates/api/src/handlers.rs` (`get_bootstrap_completeness()`)
- **Related Issue:** Phase 25B (system-designated area enforcement)
