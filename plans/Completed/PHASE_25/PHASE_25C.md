# Phase 25C — Canonical Tables & Canonicalization

## Objective

Introduce canonical data tables and implement the canonicalization action that locks
the bid year data structure. Once canonicalized, all operational queries read from
canonical tables rather than deriving data on-the-fly.

Canonical tables exist structurally at all times, but **remain empty until
canonicalization occurs**. The presence of canonical rows for a bid year is the
authoritative signal that canonicalization has completed.

This phase establishes the foundation for override semantics and bidding execution.

---

## In-Scope

- Create canonical data tables for:
  - Area membership
  - Eligibility
  - Bid order
  - Bid windows
- Implement `CanonicalizeBidYear` domain action
- Populate canonical tables by copying current derived state
- Transition bid year to `Canonicalized` lifecycle state
- Route read queries through canonical tables when lifecycle state ≥ Canonicalized
- Emit comprehensive audit event capturing canonicalization snapshot
- Ensure canonicalization is idempotent and retry-safe
- Handle NULL values for bid order and windows (populated in future phases)
- Comprehensive tests for canonicalization behavior

---

## Out-of-Scope

- Override semantics and override actions (Phase 25D)
- Actual bid order computation (future seniority phase)
- Actual bid window computation (future crew/RDO phase)
- Bidding execution logic
- Round management
- Re-canonicalization after rollback (future phase)

---

## Canonical Data Model

Canonical tables represent the **locked, authoritative state** after canonicalization.

### Design Principles

1. **Explicit Lifecycle Gate**
   - Canonical tables exist but are empty until canonicalization
   - Canonical rows exist _if and only if_ the bid year has been canonicalized

2. **Snapshot at Point in Time**
   - Canonical data captures the derived state at canonicalization

3. **Per–Bid Year Isolation**
   - Canonical data is scoped by `bid_year_id`
   - Multiple bid years may coexist safely

4. **Stable Rows, Mutable via Override**
   - Canonical rows are created once
   - Later changes update rows in-place with `is_overridden = true`
   - All changes are captured via audit events

5. **Null Tolerance**
   - Bid order and windows may be NULL until computed later

---

## Schema Changes

### New Tables

All canonical tables include:

- `bid_year_id` — scoping and uniqueness
- `audit_event_id` — creation linkage
- `is_overridden` / `override_reason` — future Phase 25D support

---

### `canonical_area_membership`

Captures which users are assigned to which areas at canonicalization.

#### SQLite

```sql
CREATE TABLE canonical_area_membership (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bid_year_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    area_id INTEGER NOT NULL,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (area_id) REFERENCES areas(id)
);

CREATE UNIQUE INDEX idx_canonical_area_membership_unique
    ON canonical_area_membership(bid_year_id, user_id);

CREATE INDEX idx_canonical_area_membership_area
    ON canonical_area_membership(bid_year_id, area_id);
```

#### MySQL

```sql
CREATE TABLE canonical_area_membership (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    bid_year_id BIGINT NOT NULL,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    area_id BIGINT NOT NULL,
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (area_id) REFERENCES areas(id)
) ENGINE=InnoDB;

CREATE UNIQUE INDEX idx_canonical_area_membership_unique
    ON canonical_area_membership(bid_year_id, user_id);

CREATE INDEX idx_canonical_area_membership_area
    ON canonical_area_membership(bid_year_id, area_id);
```

---

### `canonical_eligibility`

Captures whether a user is eligible to bid.

#### SQLite Example

```sql
CREATE TABLE canonical_eligibility (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bid_year_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    can_bid INTEGER NOT NULL,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE UNIQUE INDEX idx_canonical_eligibility_unique
    ON canonical_eligibility(bid_year_id, user_id);
```

#### MySQL Example eligibility

```sql
CREATE TABLE canonical_eligibility (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    bid_year_id BIGINT NOT NULL,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    can_bid TINYINT NOT NULL,
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE UNIQUE INDEX idx_canonical_eligibility_unique
    ON canonical_eligibility(bid_year_id, user_id);
```

---

### `canonical_bid_order`

Captures bid order (NULL until computed).

#### SQLite Bid Order

```sql
CREATE TABLE canonical_bid_order (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bid_year_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    bid_order INTEGER,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE UNIQUE INDEX idx_canonical_bid_order_unique
    ON canonical_bid_order(bid_year_id, user_id);
```

#### MySQL Bid Order

```sql
CREATE TABLE canonical_bid_order (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    bid_year_id BIGINT NOT NULL,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    bid_order INT,
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE UNIQUE INDEX idx_canonical_bid_order_unique
    ON canonical_bid_order(bid_year_id, user_id);
```

---

### `canonical_bid_windows`

Captures bid submission windows (NULL until computed).

#### SQLite Bid Windows

```sql
CREATE TABLE canonical_bid_windows (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bid_year_id INTEGER NOT NULL,
    audit_event_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    window_start_date TEXT,
    window_end_date TEXT,
    is_overridden INTEGER NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE UNIQUE INDEX idx_canonical_bid_windows_unique
    ON canonical_bid_windows(bid_year_id, user_id);
```

#### MySQL Bid Windows

```sql
CREATE TABLE canonical_bid_windows (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    bid_year_id BIGINT NOT NULL,
    audit_event_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    window_start_date VARCHAR(50),
    window_end_date VARCHAR(50),
    is_overridden TINYINT NOT NULL DEFAULT 0,
    override_reason TEXT,
    FOREIGN KEY (bid_year_id) REFERENCES bid_years(id),
    FOREIGN KEY (audit_event_id) REFERENCES audit_events(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE UNIQUE INDEX idx_canonical_bid_windows_unique
    ON canonical_bid_windows(bid_year_id, user_id);
```

---

## Invariants Enforced

1. **Exactly One Canonical Row Per User Per Table Per Bid Year**
   - Enforced by `(bid_year_id, user_id)` unique indexes

2. **Canonical Rows Exist Only After Canonicalization**
   - Tables exist, but rows are populated only by canonicalization action

3. **Lifecycle ≥ Canonicalized ⇒ Canonical Tables Are Source of Truth**
   - Read routing is lifecycle-aware and explicit

4. **Canonical Rows Are Updated In-Place for Overrides**
   - Overrides (Phase 25D) update canonical rows:
     - `is_overridden = true`
     - `override_reason` updated
   - Audit events store original and new values

5. **NULL Values Are Valid Canonical State**
   - `bid_order`, `window_start_date`, `window_end_date` may be NULL

6. **Audit Event Presence Implies Canonicalization Completed**
   - Canonical rows + audit event define completion

---

## Domain Action: `CanonicalizeBidYear`

### Preconditions

- `lifecycle_state == BootstrapComplete`
- No users in No Bid area (Phase 25B)
- No canonical rows exist for this `bid_year_id`

### Effects (Single Transaction)

1. Create `BidYearCanonicalized` audit event
2. Insert canonical rows for all users:
   - Area membership
   - Eligibility
   - Bid order (NULL)
   - Bid windows (NULL)
3. Transition lifecycle to `Canonicalized`

### Idempotency

- If canonical rows already exist → no-op success
- If rows exist but lifecycle state stale → correct lifecycle

---

## Audit Event: `BidYearCanonicalized`

Canonicalization must emit a complete, human-readable snapshot:

- All users
- Area assignments
- Eligibility values
- Placeholder bid order and windows
- Area-level user counts

Snapshot must be JSON-serializable and readable without additional queries.

---

## Read Query Routing

### Rule

- `lifecycle_state < Canonicalized` → derived state
- `lifecycle_state ≥ Canonicalized` → canonical tables

### Implementation

Prefer explicit effective-query functions:

- `get_effective_area_membership(bid_year_id, user_id)`
- `get_effective_eligibility(bid_year_id, user_id)`
- `get_effective_bid_order(bid_year_id, user_id)`
- `get_effective_bid_windows(bid_year_id, user_id)`

---

## Testing Requirements

### Canonicalization Tests

- Canonical rows created for all users
- Rows link to audit event
- Eligibility correct
- Area membership matches
- Bid order/windows NULL
- Lifecycle updated

### Idempotency Tests

- Re-running canonicalization is safe
- No duplicate rows created

### Read Routing Tests

- Derived reads before canonicalization
- Canonical reads after canonicalization

### Audit Snapshot Tests

- Event emitted
- Snapshot complete and JSON-serializable

### Transaction Safety Tests

- Partial failures leave no rows
- Retry succeeds cleanly

---

## Exit Criteria

Phase 25C is complete when:

1. Canonical tables exist (SQLite + MySQL)
2. Rows populated only at canonicalization
3. Lifecycle transitions to `Canonicalized`
4. Canonical tables are source of truth
5. Canonicalization is transactional and idempotent
6. Audit snapshot emitted
7. All tests and migrations pass

---

## Notes

- Canonicalization is a **one-way door**
- Overrides mutate canonical rows (Phase 25D)
- Canonical tables formalize locked truth
- Derived computation must not silently resume post-canonicalization
