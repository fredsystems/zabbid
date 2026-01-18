# Phase 25A — Bid Year Lifecycle State Machine

## Objective

Introduce an explicit, enforceable lifecycle state machine for bid years that governs what operations are permitted at each stage of the bidding process. This establishes the foundation for canonicalization, locking, and controlled state transitions.

---

## In-Scope

- Add `lifecycle_state` column to `bid_years` table
- Define `BidYearLifecycle` enum with five states
- Implement state transition validation logic in domain layer
- Create domain actions for explicit state transitions
- Emit audit events for all lifecycle transitions
- Update domain validation to consult lifecycle state before allowing operations
- Define editing permissions based on current lifecycle state
- Comprehensive tests for state machine behavior

---

## Out-of-Scope

- Canonical data tables (Phase 25C)
- Canonicalization action (Phase 25C)
- Override semantics (Phase 25D)
- "No Bid" area formalization (Phase 25B)
- Bidding execution logic
- Round management
- Slot assignment

---

## Lifecycle States

```rust
pub enum BidYearLifecycle {
    Draft,              // Initial state after creation
    BootstrapComplete,  // Users and areas configured, ready to lock
    Canonicalized,      // Data locked, canonical tables authoritative
    BiddingActive,      // Bidding rounds in progress (future phases)
    BiddingClosed,      // Bidding finished, system read-only
}
```

### State Semantics

#### Draft

- Initial state when bid year is created
- Full editing allowed:
  - Create/delete areas
  - Create/delete users
  - Modify all user fields
  - Modify all area fields
  - Change bid year metadata

#### BootstrapComplete

- Indicates bootstrap phase is complete
- Same editing permissions as Draft
- Prerequisite for canonicalization
- Bootstrap completeness flag must be true to transition to this state

#### Canonicalized

- Data structure is locked
- Only specific edits allowed:
  - User name (always editable)
  - User initials (always editable)
- Prohibited operations:
  - Area creation/deletion
  - User creation/deletion
  - User area reassignment (must use override actions in Phase 25D)
  - Crew reassignment (must use override actions in Phase 25D)

#### BiddingActive

- Bidding rounds are executing (future phase)
- Restricted editing:
  - User name (always editable)
  - User initials (always editable)
- All structural changes prohibited

#### BiddingClosed

- Bidding process complete
- System effectively read-only
- Only user name/initials may be edited (for corrections)

---

## Valid State Transitions

```text
Draft → BootstrapComplete
    Requires: bootstrap_complete = true

BootstrapComplete → Canonicalized
    Requires: explicit canonicalization action (Phase 25C)

Canonicalized → BiddingActive
    Requires: explicit start bidding action (future phase)

BiddingActive → BiddingClosed
    Requires: explicit close bidding action (future phase)
```

### Transition Rules

- Only forward transitions are allowed (no backwards transitions)
- Rollback is modeled as a separate audit event, not a state transition
- Each transition must be explicit via domain action
- Each transition emits a distinct audit event
- Invalid transitions must fail with a clear domain error

---

## Invariants Enforced

1. **Single Active Bid Year**
   - Only one bid year may be in `BiddingActive` state at any time
   - Attempting to activate a second bid year must fail

2. **Forward-Only Progression**
   - State may only advance, never regress
   - `Draft` cannot transition directly to `Canonicalized` (must pass through `BootstrapComplete`)

3. **Bootstrap Prerequisite**
   - Cannot transition to `BootstrapComplete` unless `bootstrap_complete = true`

4. **State-Based Authorization**
   - Domain actions must check lifecycle state before executing
   - Operations forbidden in current state must fail with explicit error

5. **Audit Trail**
   - Every state transition produces an audit event
   - Audit events must capture: previous state, new state, actor, timestamp

---

## Schema Changes

### Migration: Add `lifecycle_state` to `bid_years`

**SQLite:**

```sql
ALTER TABLE bid_years ADD COLUMN lifecycle_state TEXT NOT NULL DEFAULT 'Draft';
```

**MySQL:**

```sql
ALTER TABLE bid_years ADD COLUMN lifecycle_state VARCHAR(50) NOT NULL DEFAULT 'Draft';
```

### Constraints

- `lifecycle_state` must be one of: `Draft`, `BootstrapComplete`, `Canonicalized`, `BiddingActive`, `BiddingClosed`
- Default value is `Draft`
- NOT NULL constraint

---

## Domain Actions Introduced

### 1. `TransitionToBootstrapComplete`

**Preconditions:**

- Current state = `Draft`
- `bootstrap_complete = true`

**Effects:**

- Set `lifecycle_state = BootstrapComplete`
- Emit `BidYearTransitionedToBootstrapComplete` audit event

**Errors:**

- `InvalidStateTransition` if current state ≠ Draft
- `BootstrapIncomplete` if bootstrap_complete ≠ true

---

### 2. `TransitionToCanonicalized`

**Preconditions:**

- Current state = `BootstrapComplete`
- Must be invoked via canonicalization action (Phase 25C)

**Effects:**

- Set `lifecycle_state = Canonicalized`
- Emit `BidYearTransitionedToCanonicalized` audit event

**Errors:**

- `InvalidStateTransition` if current state ≠ BootstrapComplete

**Note:** This action is a placeholder in Phase 25A. The full canonicalization logic is implemented in Phase 25C.

---

### 3. `TransitionToBiddingActive` (Placeholder)

**Preconditions:**

- Current state = `Canonicalized`

**Effects:**

- Set `lifecycle_state = BiddingActive`
- Emit `BidYearTransitionedToBiddingActive` audit event

**Errors:**

- `InvalidStateTransition` if current state ≠ Canonicalized
- `AnotherBidYearAlreadyActive` if another bid year is already active

**Note:** This action is defined in Phase 25A but will be fully utilized in future bidding phases.

---

### 4. `TransitionToBiddingClosed` (Placeholder)

**Preconditions:**

- Current state = `BiddingActive`

**Effects:**

- Set `lifecycle_state = BiddingClosed`
- Emit `BidYearTransitionedToBiddingClosed` audit event

**Errors:**

- `InvalidStateTransition` if current state ≠ BiddingActive

**Note:** This action is defined in Phase 25A but will be fully utilized in future bidding phases.

---

## Audit Events

### `BidYearTransitionedToBootstrapComplete`

**Payload:**

```rust
{
    bid_year_id: i64,
    previous_state: "Draft",
    new_state: "BootstrapComplete",
    actor: String,
    timestamp: DateTime,
}
```

---

### `BidYearTransitionedToCanonicalized`

**Payload:**

```rust
{
    bid_year_id: i64,
    previous_state: "BootstrapComplete",
    new_state: "Canonicalized",
    actor: String,
    timestamp: DateTime,
}
```

---

### `BidYearTransitionedToBiddingActive`

**Payload:**

```rust
{
    bid_year_id: i64,
    previous_state: "Canonicalized",
    new_state: "BiddingActive",
    actor: String,
    timestamp: DateTime,
}
```

---

### `BidYearTransitionedToBiddingClosed`

**Payload:**

```rust
{
    bid_year_id: i64,
    previous_state: "BiddingActive",
    new_state: "BiddingClosed",
    actor: String,
    timestamp: DateTime,
}
```

---

## Editing Permission Matrix

| Operation               | Draft | BootstrapComplete | Canonicalized | BiddingActive | BiddingClosed |
| ----------------------- | ----- | ----------------- | ------------- | ------------- | ------------- |
| Create Area             | ✓     | ✓                 | ✗             | ✗             | ✗             |
| Delete Area             | ✓     | ✓                 | ✗             | ✗             | ✗             |
| Edit Area Name          | ✓     | ✓                 | ✗             | ✗             | ✗             |
| Create User             | ✓     | ✓                 | ✗             | ✗             | ✗             |
| Delete User             | ✓     | ✓                 | ✗             | ✗             | ✗             |
| Edit User Name          | ✓     | ✓                 | ✓             | ✓             | ✓             |
| Edit User Initials      | ✓     | ✓                 | ✓             | ✓             | ✓             |
| Edit User Crew          | ✓     | ✓                 | ✗\*           | ✗\*           | ✗             |
| Assign User to Area     | ✓     | ✓                 | ✗\*           | ✗\*           | ✗             |
| Edit Bid Year Metadata  | ✓     | ✓                 | ✗             | ✗             | ✗             |
| Mark Bootstrap Complete | ✓     | ✓                 | ✗             | ✗             | ✗             |

\* Requires override action (Phase 25D)

---

## Domain Validation Changes

Existing domain actions must be updated to check `lifecycle_state`:

### Area Operations

- `CreateArea` → require state ≤ BootstrapComplete
- `DeleteArea` → require state ≤ BootstrapComplete
- `RenameArea` → require state ≤ BootstrapComplete

### User Operations

- `CreateUser` → require state ≤ BootstrapComplete
- `DeleteUser` → require state ≤ BootstrapComplete
- `AssignUserToArea` → require state ≤ BootstrapComplete
- `AssignUserToCrew` → require state ≤ BootstrapComplete
- `UpdateUserName` → allowed in all states
- `UpdateUserInitials` → allowed in all states

### Bid Year Operations

- `MarkBootstrapComplete` → allowed in Draft and BootstrapComplete states
- `MarkBootstrapIncomplete` → allowed in Draft and BootstrapComplete states

---

## Exit Criteria

Phase 25A is complete when:

1. ✅ `lifecycle_state` column exists in `bid_years` table (both SQLite and MySQL)
2. ✅ `BidYearLifecycle` enum defined in domain layer
3. ✅ All four state transition actions implemented and tested
4. ✅ State transition validation prevents invalid transitions
5. ✅ Audit events emitted for all transitions
6. ✅ Existing domain actions updated to check lifecycle state
7. ✅ Single active bid year invariant enforced
8. ✅ All tests pass (`cargo xtask ci` and `pre-commit run --all-files`)
9. ✅ Migrations verified for both SQLite and MySQL (`cargo xtask verify-migrations`)
10. ✅ No breaking changes to existing APIs

---

## Testing Requirements

### State Machine Tests

- Valid transitions succeed
- Invalid transitions fail with correct error
- Cannot skip states (e.g., Draft → Canonicalized)
- Cannot reverse states

### Single Active Bid Year Tests

- Only one bid year may be BiddingActive
- Attempting to activate second bid year fails
- Other states (Draft, BootstrapComplete, Canonicalized, BiddingClosed) may coexist

### Bootstrap Prerequisite Tests

- Cannot transition to BootstrapComplete if bootstrap_complete = false
- Can transition if bootstrap_complete = true

### Audit Event Tests

- Each transition emits correct audit event
- Audit events capture previous and new state
- Audit events are persisted and queryable

### Permission Tests

- Area creation forbidden when Canonicalized or later
- User creation forbidden when Canonicalized or later
- User name/initials always editable
- User area reassignment forbidden when Canonicalized or later

---

## Notes

- This phase establishes the state machine only
- Canonicalization action implementation is deferred to Phase 25C
- Override semantics are deferred to Phase 25D
- "No Bid" area logic is deferred to Phase 25B
- The lifecycle state is the authoritative source for operation permissions
- UI must respect lifecycle state (queried via API, not inferred)
