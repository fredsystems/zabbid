# Phase 25B — "No Bid" Area Formalization

## Objective

Formalize the concept of a system-managed "No Bid" area that serves as a staging ground for imported users, a deletion sink (pre-canonicalization only), and a manual review queue. Bootstrap is incomplete until all users have been reviewed and assigned to operational areas.

---

## In-Scope

- Add `is_system_area` flag to `areas` table
- Auto-create "No Bid" area when bid year is bootstrapped
- Update bootstrap completeness logic to check for users in No Bid area
- Add visibility rules for system areas (hidden from unauthenticated users)
- Prevent deletion of system areas
- Define "No Bid" area semantics and behavior
- Comprehensive tests for No Bid area lifecycle

---

## Out-of-Scope

- Canonical data tables (Phase 25C)
- Canonicalization action (Phase 25C)
- Override semantics (Phase 25D)
- Bidding execution logic
- Round management
- Import functionality (assumes future import phase will use No Bid area)

---

## "No Bid" Area Semantics

### Purpose

The "No Bid" area serves three functions:

1. **Import Staging**: Users imported from external systems are placed in No Bid by default
2. **Manual Review Queue**: Administrators must explicitly assign users to operational areas
3. **Deletion Sink** (pre-canonicalization only): Users "deleted" before canonicalization are moved to No Bid instead of hard-deleted

### Characteristics

- **System-Managed**: Created automatically, cannot be deleted
- **Always Exists**: One No Bid area per bid year
- **Hidden from Public**: Not visible to unauthenticated users
- **No Bidding Semantics**: Has no rounds, no limits, no bid windows
- **Blocks Bootstrap**: Bootstrap is incomplete while users remain in No Bid
- **Review Required**: Presence of users in No Bid indicates manual review needed

### Naming Convention

- Name: `"No Bid"`
- Identifying flag: `is_system_area = true`
- Uniqueness constraint: Only one system area per bid year

---

## Invariants Enforced

1. **One System Area Per Bid Year**
   - Each bid year has exactly one area with `is_system_area = true`
   - System area is created automatically during bid year bootstrap

2. **System Areas Cannot Be Deleted**
   - Attempting to delete an area with `is_system_area = true` must fail
   - This invariant applies in all lifecycle states

3. **System Areas Cannot Be Renamed**
   - The "No Bid" area name is fixed
   - Attempting to rename a system area must fail

4. **Bootstrap Incomplete While No Bid Occupied**
   - `bootstrap_complete` cannot be set to `true` if any users are in No Bid area
   - Setting `bootstrap_complete = true` requires all users assigned to operational areas

5. **No Bid Area Hidden from Public**
   - Public/unauthenticated API endpoints must filter out system areas
   - Admin endpoints may expose system areas

6. **Users May Be Moved Out, Not In (Post-Canonicalization)**
   - After canonicalization, users cannot be moved into No Bid
   - No Bid is only for pre-canonicalization staging

---

## Schema Changes

### Migration: Add `is_system_area` to `areas`

**SQLite:**

```sql
ALTER TABLE areas ADD COLUMN is_system_area INTEGER NOT NULL DEFAULT 0;
```

**MySQL:**

```sql
ALTER TABLE areas ADD COLUMN is_system_area TINYINT NOT NULL DEFAULT 0;
```

### Constraints

- `is_system_area` is a boolean (0 or 1 in SQLite, TINYINT in MySQL)
- Default value is `0` (false)
- NOT NULL constraint
- At most one area per bid year may have `is_system_area = true`

---

## Domain Actions Introduced

### 1. `CreateNoBidArea`

**Preconditions:**

- Bid year exists
- Bid year does not already have a system area

**Effects:**

- Create area with name "No Bid"
- Set `is_system_area = true`
- Emit `NoBidAreaCreated` audit event

**Errors:**

- `SystemAreaAlreadyExists` if bid year already has a system area

**Note:** This action is invoked automatically during bid year bootstrap, not exposed as a public API.

---

## Domain Actions Modified

### `MarkBootstrapComplete`

**New Precondition:**

- No users may be assigned to the No Bid area

**Errors:**

- `UsersInNoBidArea` if any users remain in No Bid area

**Logic:**

- Query users assigned to area where `is_system_area = true`
- If count > 0, fail with `UsersInNoBidArea` error
- Otherwise, proceed with existing logic

---

### `DeleteArea`

**New Precondition:**

- Area must not be a system area (`is_system_area = false`)

**Errors:**

- `CannotDeleteSystemArea` if `is_system_area = true`

---

### `RenameArea`

**New Precondition:**

- Area must not be a system area (`is_system_area = false`)

**Errors:**

- `CannotRenameSystemArea` if `is_system_area = true`

---

### `DeleteUser` (Pre-Canonicalization)

**New Behavior:**

- If bid year lifecycle state ≤ BootstrapComplete:
  - Move user to No Bid area instead of hard-deleting
  - Emit `UserMovedToNoBid` audit event
- If bid year lifecycle state ≥ Canonicalized:
  - Fail with `CannotDeleteUserAfterCanonicalization` error

**Rationale:**

- Preserves audit trail
- Allows undo via reassignment
- Signals "needs review" rather than permanent deletion

---

### `AssignUserToArea`

**New Validation (Post-Canonicalization):**

- If target area is No Bid area and lifecycle state ≥ Canonicalized:
  - Fail with `CannotAssignToNoBidAfterCanonicalization` error
- Moving users OUT of No Bid is always allowed (subject to lifecycle state rules)

**Rationale:**

- No Bid is for pre-canonicalization staging only
- Post-canonicalization, users must be in operational areas or use override semantics

---

## Audit Events

### `NoBidAreaCreated`

**Payload:**

```rust
{
    bid_year_id: i64,
    area_id: i64,
    area_name: "No Bid",
    actor: String,
    timestamp: DateTime,
}
```

---

### `UserMovedToNoBid`

**Payload:**

```rust
{
    bid_year_id: i64,
    user_id: i64,
    previous_area_id: i64,
    reason: String, // "User deletion requested before canonicalization"
    actor: String,
    timestamp: DateTime,
}
```

---

## API Visibility Rules

### Public/Unauthenticated Endpoints

The following endpoints must filter out system areas:

- `GET /api/bid_years/{year}/areas` → exclude `is_system_area = true`
- `GET /api/bid_years/{year}/areas/{area_id}` → return 404 if system area
- Any public listing of areas

**Implementation:**

- Add `WHERE is_system_area = 0` to queries
- Or filter results in application layer before serialization

---

### Admin/Authenticated Endpoints

Admin endpoints may expose system areas for review purposes:

- Admin area listings include No Bid area
- Admin user listings show users in No Bid area
- Bootstrap completeness checks highlight No Bid occupancy

---

## Bootstrap Workflow

### Initial State

1. Bid year created in `Draft` state
2. No Bid area auto-created
3. `bootstrap_complete = false`

### User Import/Creation

1. Users imported or created
2. Users without explicit area assignment go to No Bid area
3. Bootstrap remains incomplete

### Manual Review

1. Administrator reviews users in No Bid area
2. Administrator assigns users to operational areas
3. No Bid area becomes empty

### Bootstrap Completion

1. Administrator attempts to mark bootstrap complete
2. System checks: No Bid area empty?
3. If yes → `bootstrap_complete = true`, can transition to BootstrapComplete state
4. If no → Fail with `UsersInNoBidArea` error, listing users still in No Bid

---

## UI Implications

### Admin Interface

**Bootstrap Status Display:**

- Show count of users in No Bid area
- Highlight if count > 0 (blocks bootstrap completion)
- Provide link to review users in No Bid

**User Management:**

- Filter to show "Users needing review" (i.e., users in No Bid)
- Bulk assign functionality to move users out of No Bid

**Area Listings:**

- Show No Bid area with special indicator (e.g., badge: "System Area")
- Disable delete/rename actions for system areas

### Public Interface

**Area Listings:**

- No Bid area not visible
- No Bid area not selectable in any public forms

---

## Exit Criteria

Phase 25B is complete when:

1. ✅ `is_system_area` column exists in `areas` table (both SQLite and MySQL)
2. ✅ No Bid area auto-created during bid year bootstrap
3. ✅ Bootstrap completeness check fails if users in No Bid area
4. ✅ System areas cannot be deleted
5. ✅ System areas cannot be renamed
6. ✅ System areas hidden from public API endpoints
7. ✅ User "deletion" (pre-canonicalization) moves user to No Bid instead
8. ✅ Cannot assign users to No Bid after canonicalization
9. ✅ All tests pass (`cargo xtask ci` and `pre-commit run --all-files`)
10. ✅ Migrations verified for both SQLite and MySQL (`cargo xtask verify-migrations`)
11. ✅ No breaking changes to existing APIs

---

## Testing Requirements

### System Area Creation Tests

- No Bid area created automatically on bid year bootstrap
- Cannot create multiple system areas per bid year
- System area has correct name and flag

### System Area Protection Tests

- Cannot delete system area
- Cannot rename system area
- Attempting either operation returns appropriate error

### Bootstrap Completeness Tests

- Cannot mark bootstrap complete with users in No Bid area
- Can mark bootstrap complete when No Bid area is empty
- Error message lists users in No Bid area

### Visibility Tests

- Public endpoints exclude system areas
- Admin endpoints include system areas
- Direct access to system area via public API returns 404

### User Deletion Tests (Pre-Canonicalization)

- Deleting user moves them to No Bid area
- Audit event records the move
- User can be reassigned out of No Bid

### User Assignment Tests (Post-Canonicalization)

- Cannot assign users to No Bid area after canonicalization
- Can still move users OUT of No Bid after canonicalization (if any remain)

### Audit Event Tests

- No Bid area creation emits audit event
- User moved to No Bid emits audit event
- Audit events are persisted and queryable

---

## Notes

- No Bid area is conceptually similar to a "trash" or "quarantine" folder
- It makes implicit state (users needing review) explicit and queryable
- It provides a recoverable alternative to hard deletion
- It serves as a forcing function for administrators to complete bootstrap
- Future import functionality will rely on No Bid area as the staging ground
