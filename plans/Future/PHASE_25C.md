# Phase 25C — Canonical Tables & Canonicalization

## Objective

Introduce canonical data tables and implement the canonicalization action that locks the bid year data structure. Once canonicalized, all operational queries read from canonical tables rather than deriving data on-the-fly. This establishes the foundation for override semantics and bidding execution.

---

## In-Scope

- Create canonical data tables for area membership, eligibility, bid order, and bid windows
- Implement `CanonicalizeBidYear` domain action
- Populate canonical tables by copying current state
- Transition bid year to `Canonicalized` lifecycle state
- Route read queries through canonical tables when lifecycle state ≥ Canonicalized
- Emit comprehensive audit event capturing canonicalization snapshot
- Ensure canonicalization is idempotent (safe to retry)
- Handle NULL values for bid order and windows (populated in future phases)
- Comprehensive tests for canonicalization behavior

---

## Out-of-Scope

- Override semantics and override actions (Phase 25D)
- Actual bid order computation (future seniority phase)
- Actual bid window computation (future crew/RDO phase)
- Bidding execution logic
- Round management
- "Recanonicalization" after rollback (future phase)

---

## Canonical Data Model

Canonical tables represent the **locked, authoritative state** after canonicalization.

### Design Principles

1. **Snapshot at Point in Time**: Canonical data captures state at canonicalization event
2. **Immutable After Creation**: Canonical records are not mutated (only overridden)
3. **Audit Trail**: Each canonical record links to the audit event that created it
4. **Override Support**: Each record tracks whether it has been overridden
5. **Null Tolerance**: Bid order and windows may be NULL (populated later)

---

## Schema Changes

### New Tables

#### `canonical_area_membership`

Captures which users are assigned to which areas at canonicalization.

**SQLite:**

```sql
CREATE TABLE canonical_area_membership (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    area_id INTEGER NOT NULL,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (area_id) REFERENCES areas(id)
);

CREATE INDEX idx_canonical_area_membership_user
    ON canonical_area_membership(user_id);
CREATE INDEX idx_canonical_area_membership_area
    ON canonical_area_membership(area_id);
```

**MySQL:**

```sql
CREATE TABLE canonical_area_membership (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    area_id BIGINT NOT NULL,
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (area_id) REFERENCES areas(id)
) ENGINE=InnoDB;

CREATE INDEX idx_canonical_area_membership_user
    ON canonical_area_membership(user_id);
CREATE INDEX idx_canonical_area_membership_area
    ON canonical_area_membership(area_id);
```

---

#### `canonical_eligibility`

Captures which users are eligible to bid.

**SQLite:**

```sql
CREATE TABLE canonical_eligibility (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    can_bid INTEGER NOT NULL,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE UNIQUE INDEX idx_canonical_eligibility_user
    ON canonical_eligibility(user_id);
```

**MySQL:**

```sql
CREATE TABLE canonical_eligibility (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    can_bid TINYINT NOT NULL,
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE UNIQUE INDEX idx_canonical_eligibility_user
    ON canonical_eligibility(user_id);
```

---

#### `canonical_bid_order`

Captures the order in which users bid (based on seniority).

**SQLite:**

```sql
CREATE TABLE canonical_bid_order (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    bid_order INTEGER, -- NULL until seniority computation implemented
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE UNIQUE INDEX idx_canonical_bid_order_user
    ON canonical_bid_order(user_id);
```

**MySQL:**

```sql
CREATE TABLE canonical_bid_order (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    bid_order INT, -- NULL until seniority computation implemented
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE UNIQUE INDEX idx_canonical_bid_order_user
    ON canonical_bid_order(user_id);
```

---

#### `canonical_bid_windows`

Captures the date range during which each user may submit bids.

**SQLite:**

```sql
CREATE TABLE canonical_bid_windows (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    window_start_date TEXT, -- NULL until bid window computation implemented
    window_end_date TEXT,   -- NULL until bid window computation implemented
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE UNIQUE INDEX idx_canonical_bid_windows_user
    ON canonical_bid_windows(user_id);
```

**MySQL:**

```sql
CREATE TABLE canonical_bid_windows (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    window_start_date VARCHAR(50), -- NULL until bid window computation implemented
    window_end_date VARCHAR(50),   -- NULL until bid window computation implemented
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE UNIQUE INDEX idx_canonical_bid_windows_user
    ON canonical_bid_windows(user_id);
```

---

## Invariants Enforced

1. **One Canonical Record Per User Per Table**
   - Each user has exactly one canonical eligibility record
   - Each user has exactly one canonical bid order record
   - Each user has exactly one canonical bid windows record
   - Each user has exactly one canonical area membership record

2. **Canonical Records Immutable**
   - Canonical records are created once and never updated
   - Overrides create new audit events but set `is_overridden` flag

3. **Audit Linkage**
   - Every canonical record links to the audit event that created it
   - Audit event must be `BidYearCanonicalized` type

4. **NULL Tolerance**
   - `bid_order` may be NULL (populated in future seniority phase)
   - `window_start_date` and `window_end_date` may be NULL (populated in future phases)
   - `can_bid` must always be a boolean (never NULL)
   - `user_id` and `area_id` must always be valid (never NULL)

5. **Post-Canonicalization Read Routing**
   - When lifecycle_state ≥ Canonicalized, queries must read from canonical tables
   - When lifecycle_state < Canonicalized, queries compute derived state

---

## Domain Actions Introduced

### `CanonicalizeBidYear`

**Preconditions:**

- Current lifecycle state = `BootstrapComplete`
- No users in No Bid area (enforced by Phase 25B)
- Bid year not already canonicalized

**Effects:**

1. **Create Audit Event**
   - Type: `BidYearCanonicalized`
   - Capture snapshot of all users and their state

2. **Populate Canonical Area Membership**
   - For each user:
     - Query current area assignment
     - Insert into `canonical_area_membership`
     - Link to audit event

3. **Populate Canonical Eligibility**
   - For each user:
     - Set `can_bid = true` if user not in No Bid area
     - Set `can_bid = false` if user in No Bid area (should be impossible at this point)
     - Insert into `canonical_eligibility`
     - Link to audit event

4. **Populate Canonical Bid Order**
   - For each user:
     - Set `bid_order = NULL` (to be populated in future seniority phase)
     - Insert into `canonical_bid_order`
     - Link to audit event

5. **Populate Canonical Bid Windows**
   - For each user:
     - Set `window_start_date = NULL` (to be populated in future phase)
     - Set `window_end_date = NULL` (to be populated in future phase)
     - Insert into `canonical_bid_windows`
     - Link to audit event

6. **Transition Lifecycle State**
   - Invoke `TransitionToCanonicalized` action (from Phase 25A)
   - Set `lifecycle_state = Canonicalized`

**Errors:**

- `InvalidStateTransition` if current state ≠ BootstrapComplete
- `UsersInNoBidArea` if any users remain in No Bid area
- `AlreadyCanonicalized` if canonical tables already populated for this bid year

**Idempotency:**

- If canonicalization fails partway through (e.g., database error), retry must be safe
- Check for existing canonical records before attempting to create new ones
- If canonical records exist, treat as already canonicalized

---

## Audit Events

### `BidYearCanonicalized`

**Payload:**

```rust
{
    bid_year_id: i64,
    canonicalized_at: DateTime,
    user_count: usize,
    area_count: usize,
    snapshot: {
        users: Vec<CanonicalUserSnapshot>,
        areas: Vec<AreaSnapshot>,
    },
    actor: String,
    timestamp: DateTime,
}
```

**CanonicalUserSnapshot:**

```rust
{
    user_id: i64,
    initials: String,
    name: String,
    area_id: i64,
    area_name: String,
    can_bid: bool,
    bid_order: Option<i32>,
    window_start: Option<String>,
    window_end: Option<String>,
}
```

**AreaSnapshot:**

```rust
{
    area_id: i64,
    area_name: String,
    user_count: usize,
}
```

---

## Read Query Routing

### Principle

After canonicalization, the system must read from canonical tables, not compute derived state.

### Implementation Strategy

#### Option 1: Query Layer Branching

- Persistence layer checks `lifecycle_state` before executing query
- If state < Canonicalized: compute derived state (current behavior)
- If state ≥ Canonicalized: query canonical tables

#### Option 2: Domain Layer Abstraction

- Domain layer provides trait: `CanonicalDataSource`
- Implementations: `DerivedDataSource`, `CanonicalDataSource`
- Select implementation based on lifecycle state

#### Recommendation

Option 1 (simpler, less abstraction overhead)

### Affected Queries

- **User area assignment**: Read from `canonical_area_membership` instead of `users.area_id`
- **User eligibility**: Read from `canonical_eligibility` instead of computing
- **Bid order**: Read from `canonical_bid_order` (will be NULL initially)
- **Bid windows**: Read from `canonical_bid_windows` (will be NULL initially)

---

## Canonicalization Snapshot Requirements

The audit event must capture a complete, human-readable snapshot of the canonicalized state.

**Requirements:**

- All users included in snapshot
- Each user's area assignment recorded
- Each user's eligibility recorded
- Snapshot must be JSON-serializable
- Snapshot must be readable without querying other tables
- Snapshot must enable historical reconstruction

**Purpose:**

- Audit trail
- Debugging
- Historical analysis
- Rollback support (future phase)

---

## Handling Partial Canonicalization

Since bid order and windows cannot be computed yet:

**Approach:**

- Canonical records are created with NULL values
- Future phases will populate these fields via update or override
- NULL is a valid canonical state meaning "not yet computed"
- Queries must handle NULL gracefully (e.g., return None/Option)

**Alternative Rejected:**

- Waiting until all canonicalization logic is complete
- **Reason:** Would delay lifecycle enforcement indefinitely

---

## Testing Requirements

### Canonicalization Tests

- Canonicalize creates canonical records for all users
- Canonical records link to audit event
- Canonical eligibility set correctly (true if not in No Bid)
- Canonical area membership matches current assignment
- Bid order and windows are NULL
- Lifecycle state transitions to Canonicalized

### Idempotency Tests

- Calling canonicalize twice does not create duplicate records
- Second call either succeeds as no-op or fails gracefully

### Read Routing Tests

- Before canonicalization: queries compute derived state
- After canonicalization: queries read canonical tables
- Results are consistent across the transition

### Audit Event Tests

- Canonicalization emits audit event
- Audit event contains complete snapshot
- Snapshot includes all users and areas
- Snapshot is JSON-serializable

### Error Handling Tests

- Cannot canonicalize if state ≠ BootstrapComplete
- Cannot canonicalize if users in No Bid area
- Cannot canonicalize twice

### NULL Handling Tests

- Querying bid order returns None/Option when NULL
- Querying bid windows returns None/Option when NULL
- NULL values do not cause query failures

---

## Exit Criteria

Phase 25C is complete when:

1. ✅ All four canonical tables exist (both SQLite and MySQL)
2. ✅ `CanonicalizeBidYear` action implemented and tested
3. ✅ Canonical tables populated correctly on canonicalization
4. ✅ Lifecycle state transitions to `Canonicalized`
5. ✅ Audit event `BidYearCanonicalized` emitted with complete snapshot
6. ✅ Read queries route through canonical tables when state ≥ Canonicalized
7. ✅ Canonicalization is idempotent
8. ✅ NULL values handled gracefully in queries
9. ✅ All tests pass (`cargo xtask ci` and `pre-commit run --all-files`)
10. ✅ Migrations verified for both SQLite and MySQL (`cargo xtask verify-migrations`)
11. ✅ No breaking changes to existing APIs

---

## Notes

- Canonical tables are the **source of truth** after canonicalization
- Derived computation is disabled after canonicalization (no silent recomputation)
- NULL values are a temporary state, not an error condition
- Future phases will populate bid order and windows
- Override semantics (Phase 25D) will allow editing canonical data with audit trail
- Canonicalization is a one-way door (can only be undone via rollback in future phase)
